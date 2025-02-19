use axum::{
    Json, async_trait,
    extract::{FromRequestParts, State},
    http::{StatusCode, header},
    response::IntoResponse,
};
use lambda_http::RequestExt;
use oauth2::{AuthorizationCode, CsrfToken, Scope, TokenResponse};
use serde_json::json;
use tracing::instrument;
use types::{
    AccessTokenResponse, AuthorizationUrlResponse, TwitchAuthRequest,
    TwitchCallbackRequest, TwitchCallbackResponse, TwitchSessionSecret,
};

use crate::{structs::AppState, twitch};

pub async fn obtain_twitch_authorization_url_handler(
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
    match gt_secrets::create_or_replace::<TwitchSessionSecret>(
        &state.secrets_manager,
        &state.config.user_secret_path.secret_path(&cognito_user_id),
        &gt_secrets::SessionSecret::new(
            csrf_state.secret().to_string(),
            request.redirect_uri.clone(),
            request.scopes.clone(),
        ),
    )
    .await
    {
        Ok(()) => (),
        Err(e) => {
            tracing::error!("failed to store in secrets manager: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
        }
    };

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

pub async fn twitch_callback_handler(
    State(state): State<AppState>,
    CognitoUserId(cognito_user_id): CognitoUserId,
    Json(request): Json<TwitchCallbackRequest>,
) -> impl IntoResponse {
    let secret_id =
        state.config.user_secret_path.secret_path(&cognito_user_id);

    let Ok(secret) = gt_secrets::get::<TwitchSessionSecret>(
        &state.secrets_manager,
        &secret_id,
    )
    .await
    else {
        return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
    };

    if secret.csrf_token != request.state {
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

    match gt_secrets::set_tokens::<TwitchSessionSecret>(
        &state.secrets_manager,
        &secret_id,
        token_response.access_token.secret(),
        token_response
            .refresh_token
            .as_ref()
            .map_or("", |t| t.secret()),
        token_response.expires_in(),
    )
    .await
    {
        Ok(()) => (),
        Err(e) => {
            tracing::error!("failed to store in secrets manager: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
        }
    };

    let response_body = TwitchCallbackResponse {
        url: secret.redirect_url,
    };

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        Json(json!(response_body)),
    )
        .into_response()
}

#[instrument(skip(state))]
pub async fn obtain_twitch_access_token_handler(
    State(state): State<AppState>,
    CognitoUserId(cognito_user_id): CognitoUserId,
) -> impl IntoResponse {
    let secret_id =
        state.config.user_secret_path.secret_path(&cognito_user_id);

    let secret = match gt_secrets::get::<TwitchSessionSecret>(
        &state.secrets_manager,
        &secret_id,
    )
    .await
    {
        Ok(secret) => secret,
        Err(e) => {
            tracing::error!("failed to get secret: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
        }
    };

    let Some(access_token) = secret.access_token else {
        tracing::warn!("access_token not found in secret");
        return (StatusCode::UNAUTHORIZED,).into_response();
    };

    if let Ok(validation_response) =
        twitch::validate_token(&access_token).await
    {
        let response_body = AccessTokenResponse {
            access_token,
            broadcaster_id: validation_response.user_id,
        };

        return (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/json")],
            Json(json!(response_body)),
        )
            .into_response();
    }

    // if the token is invalid, try to refresh it
    tracing::warn!("invalid access token");

    let Some(refresh_token) = secret.refresh_token else {
        tracing::warn!("refresh_token not found in secret");
        return (StatusCode::UNAUTHORIZED,).into_response();
    };

    let client = match twitch::get_oauth_client(&state.twitch_credentials) {
        Ok(client) => client,
        Err(e) => {
            tracing::error!("failed to get Twitch OAuth client: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
        }
    };

    let token_response = match client
        .exchange_refresh_token(&oauth2::RefreshToken::new(refresh_token))
        .request_async(oauth2::reqwest::async_http_client)
        .await
    {
        Ok(token) => token,
        Err(e) => {
            // clear the access token and refresh token from the secrets manager secret

            tracing::error!("failed to refresh token: {:?}", e);
            return match gt_secrets::clear_tokens::<TwitchSessionSecret>(
                &state.secrets_manager,
                &secret_id,
            )
            .await
            {
                Ok(_) => (StatusCode::UNAUTHORIZED,).into_response(),
                Err(e) => {
                    tracing::error!("failed to clear access token: {:?}", e);
                    return (StatusCode::INTERNAL_SERVER_ERROR,)
                        .into_response();
                }
            };
        }
    };

    match gt_secrets::set_tokens::<TwitchSessionSecret>(
        &state.secrets_manager,
        &secret_id,
        token_response.access_token.secret(),
        token_response
            .refresh_token
            .as_ref()
            .map_or("", |t| t.secret()),
        token_response.expires_in(),
    )
    .await
    {
        Ok(()) => {
            if let Ok(validation_response) =
                twitch::validate_token(&access_token).await
            {
                let response_body = AccessTokenResponse {
                    access_token: token_response
                        .access_token
                        .secret()
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
            tracing::error!("failed to validate refreshed token");
            return (StatusCode::UNAUTHORIZED,).into_response();
        }
        Err(e) => {
            tracing::error!("failed to store in secrets manager: {:?}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
        }
    };
}

// axum extractor to get cognito user id from the request
#[derive(Debug, Clone)]
pub struct CognitoUserId(String);

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
