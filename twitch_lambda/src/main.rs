use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
use aws_sdk_secretsmanager::client::Client as SecretsManagerClient;
use axum::{
    async_trait,
    body::Body,
    extract::{FromRequestParts, State},
    http::{header, Request, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use figment::Figment;
use lambda_http::{tower, RequestExt};
use oauth2::{AuthorizationCode, CsrfToken, Scope};
use serde::Deserialize;
use serde_json::json;
use std::{collections::HashMap, sync::Arc};
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
use tracing::instrument;
use types::{
    AccessTokenResponse, AuthorizationUrlResponse, TwitchAuthRequest,
    TwitchCallbackRequest, TwitchCallbackResponse,
};

mod twitch;

#[derive(Debug, Clone, Deserialize)]
struct UserSecretPathProvider(String);

impl UserSecretPathProvider {
    fn secret_path(&self, cognito_user_id: &str) -> String {
        format!(
            "{prefix}/{cognito_user_id}",
            prefix = self.0,
            cognito_user_id = cognito_user_id
        )
    }
}

#[derive(Debug, Deserialize, Clone)]
#[allow(clippy::struct_field_names)]
struct Config {
    twitch_secret_arn: String,

    user_secret_path: UserSecretPathProvider,
}

fn load_config() -> Result<Config, figment::Error> {
    let figment = Figment::new().merge(figment::providers::Env::raw());

    figment.extract()
}

#[derive(Debug, Clone)]
struct AppState {
    secrets_manager: Arc<SecretsManagerClient>,
    twitch_credentials: twitch::Credentials,
    config: Config,
}

#[tokio::main]
async fn main() {
    // https://docs.aws.amazon.com/lambda/latest/dg/rust-logging.html
    tracing_subscriber::fmt()
        .json()
        .with_max_level(tracing::Level::INFO)
        // this needs to be set to remove duplicated information in the log.
        .with_current_span(false)
        // this needs to be set to false, otherwise ANSI color codes will
        // show up in a confusing manner in CloudWatch logs.
        .with_ansi(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        // remove the name of the function from every log entry
        .with_target(false)
        .init();

    let config = load_config().expect("failed to load config");
    let region_provider =
        RegionProviderChain::default_provider().or_else("us-east-1");
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    let secrets_manager = SecretsManagerClient::new(&aws_config);

    let twitch_credentials = match secrets_manager
        .get_secret_value()
        .secret_id(&config.twitch_secret_arn)
        .send()
        .await
    {
        Ok(secret) => match serde_json::from_str::<twitch::Credentials>(
            secret.secret_string.as_deref().unwrap_or("{}"),
        ) {
            Ok(credentials) => credentials,
            Err(e) => {
                tracing::error!("failed to parse Twitch secret: {:?}", e);
                return;
            }
        },
        Err(e) => {
            tracing::error!("failed to get Twitch secret: {:?}", e);
            return;
        }
    };

    // Create a shared state to pass to the handler
    let state = AppState {
        secrets_manager: Arc::new(secrets_manager),
        twitch_credentials,
        config,
    };

    // Set up a trace layer
    let trace_layer = TraceLayer::new_for_http().on_request(
        |request: &Request<Body>, _: &tracing::Span| {
            tracing::info!(
                "received request: {method} {uri}",
                method = request.method(),
                uri = request.uri()
            );
        },
    );

    let compression_layer = CompressionLayer::new().gzip(true).deflate(true);

    // Create Axum app
    let app = Router::new()
        .route(
            "/auth/twitch/url",
            post(obtain_twitch_authorization_url_handler),
        )
        .route("/auth/twitch/callback", post(twitch_callback_handler))
        .route(
            "/auth/twitch/token",
            get(obtain_twitch_access_token_handler),
        )
        .fallback(|| async {
            (
                StatusCode::NOT_FOUND,
                [(header::CONTENT_TYPE, "application/json")],
                Json(json!({
                    "message": "not found",
                })),
            )
        })
        .layer(trace_layer)
        .layer(compression_layer)
        .with_state(state);

    // Provide the app to the lambda runtime
    let app = tower::ServiceBuilder::new()
        .layer(axum_aws_lambda::LambdaLayer::default().trim_stage())
        .service(app);

    lambda_http::run(app).await.unwrap();
}

async fn obtain_twitch_authorization_url_handler(
    State(state): State<AppState>,
    CognitoUserId(cognito_user_id): CognitoUserId,
    Json(request): Json<TwitchAuthRequest>,
) -> impl IntoResponse {
    let Ok(client) = twitch::get_oauth_client(&state.twitch_credentials)
    else {
        return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
    };

    let (authorize_url, csrf_state) = client
        .authorize_url(CsrfToken::new_random)
        .add_scopes(
            request.scopes.iter().map(|scope| Scope::new(scope.clone())),
        )
        .url();

    // store csrf_state and redirect_url in secrets manager
    match update_secret_fields(
        &state.secrets_manager,
        &state.config.user_secret_path.secret_path(&cognito_user_id),
        HashMap::from_iter(vec![
            ("csrf_state".to_string(), csrf_state.secret().to_string()),
            ("redirect_url".to_string(), request.redirect_uri.clone()),
            ("scopes".to_string(), request.scopes.join(",")),
        ]),
    )
    .await
    {
        Ok(()) => (),
        Err(e) => {
            tracing::error!("failed to store in secrets manager: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
        }
    }

    let response = json!(AuthorizationUrlResponse {
        url: authorize_url.to_string(),
    });

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        Json(response),
    )
        .into_response()
}

async fn twitch_callback_handler(
    State(state): State<AppState>,
    CognitoUserId(cognito_user_id): CognitoUserId,
    Json(request): Json<TwitchCallbackRequest>,
) -> impl IntoResponse {
    let secret_id =
        state.config.user_secret_path.secret_path(&cognito_user_id);
    let secret = match state
        .secrets_manager
        .get_secret_value()
        .secret_id(&secret_id)
        .send()
        .await
    {
        Ok(secret) => secret,
        Err(e) => {
            tracing::error!("failed to get secret: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
        }
    };

    let secret_json_object = match serde_json::from_str::<serde_json::Value>(
        secret.secret_string.as_deref().unwrap_or("{}"),
    ) {
        Ok(secret_string) => secret_string,
        Err(e) => {
            tracing::error!("failed to parse secret string: {:?}", e);
            serde_json::Value::Object(serde_json::Map::new())
        }
    };

    let csrf_state =
        if let Some(csrf_state) = secret_json_object.get("csrf_state") {
            csrf_state.as_str().unwrap_or("")
        } else {
            tracing::error!("csrf_state not found in secret");
            return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
        };

    if csrf_state != request.state {
        tracing::error!("csrf_state mismatch");
        return (StatusCode::UNAUTHORIZED,).into_response();
    }

    let client = match twitch::get_oauth_client(&state.twitch_credentials) {
        Ok(client) => client,
        Err(e) => {
            tracing::error!("failed to get Twitch OAuth client: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
        }
    };

    let token_response = match client
        .exchange_code(AuthorizationCode::new(request.code.clone()))
        .request_async(oauth2::reqwest::async_http_client)
        .await
    {
        Ok(token) => token,
        Err(e) => {
            tracing::error!("failed to exchange code: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
        }
    };

    let fields = HashMap::from_iter(vec![
        (
            "access_token".to_string(),
            token_response.access_token.secret().to_string(),
        ),
        (
            "refresh_token".to_string(),
            token_response
                .refresh_token
                .map_or(String::new(), |t| t.secret().to_string()),
        ),
    ]);

    match update_secret_fields(&state.secrets_manager, &secret_id, fields)
        .await
    {
        Ok(()) => (),
        Err(e) => {
            tracing::error!("failed to store in secrets manager: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
        }
    };

    let response_body = TwitchCallbackResponse {
        url: secret_json_object
            .get("redirect_url")
            .unwrap_or(&serde_json::Value::Null)
            .as_str()
            .unwrap_or("")
            .to_string(),
    };

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        Json(json!(response_body)),
    )
        .into_response()
}

#[instrument(skip(state))]
async fn obtain_twitch_access_token_handler(
    State(state): State<AppState>,
    CognitoUserId(cognito_user_id): CognitoUserId,
) -> impl IntoResponse {
    let secret_id =
        state.config.user_secret_path.secret_path(&cognito_user_id);

    let secret = match state
        .secrets_manager
        .get_secret_value()
        .secret_id(&secret_id)
        .send()
        .await
    {
        Ok(secret) => secret,
        Err(e) => {
            tracing::error!("failed to get secret: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
        }
    };

    let secret_json_object = match serde_json::from_str::<serde_json::Value>(
        secret.secret_string.as_deref().unwrap_or("{}"),
    ) {
        Ok(secret_string) => secret_string,
        Err(e) => {
            tracing::error!("failed to parse secret string: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
        }
    };

    // TODO check if the token is expired and refresh it
    let access_token = secret_json_object
        .get("access_token")
        .and_then(|t| t.as_str());

    if access_token.is_none() {
        tracing::warn!("access_token not found in secret");
        return (StatusCode::UNAUTHORIZED,).into_response();
    }

    if let Ok(validation_response) =
        twitch::validate_token(access_token.unwrap()).await
    {
        let response_body = AccessTokenResponse {
            access_token: secret_json_object
                .get("access_token")
                .map_or("", |t| t.as_str().unwrap_or(""))
                .to_string(),

            broadcaster_id: validation_response.user_id,
        };

        return (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            Json(json!(response_body)),
        )
            .into_response();
    }

    // if the token is invalid, return unauthorized
    tracing::warn!("invalid access token");

    // clear the access token and refresh token from the secrets manager secret
    match state
        .secrets_manager
        .put_secret_value()
        .secret_id(&secret_id)
        .secret_string("{}")
        .send()
        .await
    {
        Ok(_) => (StatusCode::UNAUTHORIZED,).into_response(),
        Err(e) => {
            tracing::error!("failed to clear access token: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
        }
    }
}

// axum extractor to get cognito user id from the request
#[derive(Debug, Clone)]
struct CognitoUserId(String);

#[async_trait]
impl<S> FromRequestParts<S> for CognitoUserId
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        tracing::info!("extracting cognito user id");
        parts
            .request_context_ref()
            .and_then(|ctx| ctx.authorizer())
            .and_then(|auth| {
                auth.jwt
                    .as_ref()
                    .map(|jwt| &jwt.claims)
                    .and_then(|claims| claims.get("sub"))
            })
            .map_or(
                Err((StatusCode::UNAUTHORIZED, "Unauthorized")),
                |cognito_user_id| Ok(Self(cognito_user_id.to_string())),
            )
    }
}

async fn update_secret_fields(
    secrets_manager: &SecretsManagerClient,
    secret_id: &str,
    fields: HashMap<String, String>,
) -> Result<(), String> {
    let fetched_secret_string = match secrets_manager
        .get_secret_value()
        .secret_id(secret_id)
        .send()
        .await
    {
        Ok(secret) => secret.secret_string.unwrap_or_else(|| "{}".to_string()),
        Err(e) => {
            let e_message = e.to_string();
            tracing::error!("failed to get secret: {:?}", e);
            // if the secret doesn't exist, create it
            if e.into_service_error().is_resource_not_found_exception() {
                secrets_manager
                    .create_secret()
                    .name(secret_id)
                    .secret_string("{}")
                    .send()
                    .await
                    .map_err(|e| {
                        tracing::error!("failed to create secret: {:?}", e);
                        e.to_string()
                    })?;

                "{}".to_string()
            } else {
                return Err(e_message);
            }
        }
    };

    let mut parsed_secret = match serde_json::from_str::<serde_json::Value>(
        &fetched_secret_string,
    ) {
        Ok(secret_string) => secret_string,
        Err(e) => {
            tracing::error!("failed to parse secret string: {:?}", e);
            serde_json::Value::Object(serde_json::Map::new())
        }
    };

    for (key, value) in fields {
        parsed_secret[key] = serde_json::Value::String(value);
    }

    secrets_manager
        .put_secret_value()
        .secret_id(secret_id)
        .secret_string(
            serde_json::to_string(&parsed_secret)
                .map_err(|e| e.to_string())?,
        )
        .send()
        .await
        .map_err(|e| {
            tracing::error!("failed to update secret: {:?}", e);
            e.to_string()
        })?;

    Ok(())
}
