use axum::extract::{FromRequestParts, Query, State};
use axum::http::header;
use axum::http::request::Parts;
use axum::{async_trait, Json};
use axum::{http::StatusCode, response::IntoResponse, routing::get};
use redact::Secret;
use serde::Deserialize;
use serde_json::json;
use tracing::instrument;

// Redis key constants
const ACCESS_TOKEN_KEY: &str = "twitch:access_token";
const REFRESH_TOKEN_KEY: &str = "twitch:refresh_token";

// The state of the application.
#[derive(Clone, Debug)]
struct AppState {
    redis: redis::Client,
    twitch_client_id: String,
    twitch_client_secret: Secret<String>,
    twitch_user_id: String,

    redirect_url: String,

    http_client: reqwest::Client,
}

#[tokio::main]
async fn main() -> Result<(), axum::BoxError> {
    let twitch_client_id = dotenvy::var("TWITCH_CLIENT_ID").expect("TWITCH_CLIENT_ID not set");
    let twitch_client_secret_path =
        dotenvy::var("TWITCH_CLIENT_SECRET_PATH").expect("TWITCH_CLIENT_SECRET_PATH not set");
    let twitch_user_id = dotenvy::var("TWITCH_USER_ID").expect("TWITCH_USER_ID not set");

    let state = AppState {
        redis: redis::Client::open(dotenvy::var("REDIS_URL").expect("REDIS_URL must be set"))?,
        redirect_url: dotenvy::var("REDIRECT_URL").expect("REDIRECT_URL must be set"),

        http_client: reqwest::Client::builder()
            .user_agent("saebyn-twitch-api/0.1")
            .connection_verbose(false)
            .build()
            .expect("failed to create http client"),

        twitch_client_id,
        twitch_user_id,

        twitch_client_secret: Secret::new(
            std::fs::read_to_string(twitch_client_secret_path)
                .expect("failed to read twitch secret from TWITCH_CLIENT_SECRET_PATH")
                .trim()
                .to_string(),
        ),
    };

    common_api_lib::run(state, |app| {
        app.route("/login", get(get_login_handler).post(post_login_handler))
            .route("/videos", get(list_videos_handler))
    })
    .await
}

/**
 * Extractor for the access token from Redis.
 *
 * This is a simple extractor that gets the access token from Redis
 * and injects it into the request's extensions.
 */
struct AccessToken(Secret<String>);

#[async_trait]
impl FromRequestParts<AppState> for AccessToken {
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let mut con = match state.redis.get_multiplexed_async_connection().await {
            Ok(con) => con,
            Err(_) => {
                tracing::error!("failed to get redis connection");

                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "internal server error" })),
                ));
            }
        };

        let access_token: Result<String, redis::RedisError> =
            redis::AsyncCommands::get(&mut con, ACCESS_TOKEN_KEY).await;

        match access_token {
            Ok(access_token) => Ok(AccessToken(Secret::new(access_token))),
            Err(_) => {
                let tokens = match update_refresh_token(state).await {
                    Some(tokens) => tokens,
                    None => {
                        tracing::error!("failed to update refresh token");

                        return Err((
                            StatusCode::UNAUTHORIZED,
                            Json(json!({ "error": "need to login to Twitch" })),
                        ));
                    }
                };

                Ok(AccessToken(tokens.access_token))
            }
        }
    }
}

#[instrument]
async fn get_login_handler(State(state): State<AppState>) -> impl IntoResponse {
    let scopes = vec!["chat:read"];

    let url = format!(
        "https://id.twitch.tv/oauth2/authorize?client_id={}&redirect_uri={}&response_type=code&scope={}",
        state.twitch_client_id,
        state.redirect_url,
        scopes.join("+")
    );

    (StatusCode::OK, Json(json!({ "url": url })))
}

/**
 * POST /login
 *
 * Completes the OAuth flow by exchanging the code for an access token
 * and refresh token. Stores the tokens in Redis and returns a
 * 202 Accepted response.
 */
#[instrument]
async fn post_login_handler(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let code = body["code"].as_str().expect("code not found in body");

    let AuthTokens {
        access_token,
        refresh_token,
    } = get_token(&state, code).await;

    let mut con = state
        .redis
        .get_multiplexed_async_connection()
        .await
        .expect("failed to get redis connection");

    let _: () =
        redis::AsyncCommands::set(&mut con, REFRESH_TOKEN_KEY, refresh_token.expose_secret())
            .await
            .expect("failed to set refresh token");

    let _: () = redis::AsyncCommands::set(&mut con, ACCESS_TOKEN_KEY, access_token.expose_secret())
        .await
        .expect("failed to set access token");

    (StatusCode::ACCEPTED,)
}

#[derive(Deserialize, Debug)]
struct ListVideosQuery {
    after: Option<String>,
}

#[instrument]
async fn list_videos_handler(
    State(state): State<AppState>,
    AccessToken(access_token): AccessToken,
    Query(params): Query<ListVideosQuery>,
) -> impl IntoResponse {
    let url = format!(
        "https://api.twitch.tv/helix/videos?user_id={}&after={}",
        state.twitch_user_id,
        match params.after {
            Some(after) => after,
            None => "".to_string(),
        }
    );

    let request = state
        .http_client
        .get(&url)
        .header(
            "Authorization",
            format!("Bearer {}", access_token.expose_secret()),
        )
        .header("Client-Id", state.twitch_client_id.clone())
        .send()
        .await;

    let response = match request {
        Ok(response) => match response.status() {
            StatusCode::OK => response,
            StatusCode::UNAUTHORIZED => {
                tracing::trace!("refreshing token");
                let tokens = match update_refresh_token(&state).await {
                    Some(tokens) => tokens,
                    None => {
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({ "error": "internal server error" })),
                        )
                            .into_response();
                    }
                };

                tracing::info!("retrying request with new token");
                let request = state
                    .http_client
                    .get(&url)
                    .header(
                        "Authorization",
                        format!("Bearer {}", tokens.access_token.expose_secret()),
                    )
                    .header("Client-Id", state.twitch_client_id.clone())
                    .send()
                    .await;

                match request {
                    Ok(response) => response,
                    Err(_e) => {
                        tracing::error!("failed to get videos after refreshing token");
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({ "error": "internal server error" })),
                        )
                            .into_response();
                    }
                }
            }
            status => {
                tracing::error!("failed to get videos: {:?}", status);
                return (status, Json(json!({ "error": "failed to get videos" }))).into_response();
            }
        },
        Err(e) => {
            tracing::error!("failed to get videos: {:?}", e);

            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "internal server error" })),
            )
                .into_response();
        }
    };

    let body = response
        .json::<serde_json::Value>()
        .await
        .expect("failed to parse response");

    (
        [(header::CONTENT_TYPE, "application/json".to_string())],
        axum::Json(body),
    )
        .into_response()
}

struct AuthTokens {
    access_token: Secret<String>,
    refresh_token: Secret<String>,
}

#[instrument]
async fn get_token(state: &AppState, code: &str) -> AuthTokens {
    let url = "https://id.twitch.tv/oauth2/token";

    // urlencoded form data
    let body = json!({
      "client_id": state.twitch_client_id,
      "client_secret": state.twitch_client_secret.expose_secret(),
      "code": code,
      "grant_type": "authorization_code",
      "redirect_uri": state.redirect_url,
    });

    let response = state
        .http_client
        .post(url)
        .form(&body)
        .send()
        .await
        .expect("failed to send request")
        .json::<serde_json::Value>()
        .await
        .expect("failed to parse response");

    AuthTokens {
        access_token: Secret::new(
            response["access_token"]
                .as_str()
                .expect("access_token not found")
                .to_string(),
        ),
        refresh_token: Secret::new(
            response["refresh_token"]
                .as_str()
                .expect("refresh_token not found")
                .to_string(),
        ),
    }
}

#[instrument]
async fn do_refresh_token(state: &AppState, refresh_token: Secret<String>) -> AuthTokens {
    let url = "https://id.twitch.tv/oauth2/token";

    // urlencoded form data
    let body = json!({
      "client_id": state.twitch_client_id,
      "client_secret": state.twitch_client_secret.expose_secret(),
      "refresh_token": refresh_token.expose_secret(),
      "grant_type": "refresh_token",
    });

    let response = state
        .http_client
        .post(url)
        .form(&body)
        .send()
        .await
        .expect("failed to send request")
        .json::<serde_json::Value>()
        .await
        .expect("failed to parse response");

    AuthTokens {
        access_token: Secret::new(
            response["access_token"]
                .as_str()
                .expect("access_token not found")
                .to_string(),
        ),
        refresh_token: Secret::new(
            response["refresh_token"]
                .as_str()
                .expect("refresh_token not found")
                .to_string(),
        ),
    }
}

#[instrument]
async fn update_refresh_token(state: &AppState) -> Option<AuthTokens> {
    let mut con = state
        .redis
        .get_multiplexed_async_connection()
        .await
        .expect("failed to get redis connection");

    let refresh_token: Secret<String> =
        match redis::AsyncCommands::get(&mut con, REFRESH_TOKEN_KEY).await {
            Ok(token) => Secret::new(token),
            Err(_) => return None,
        };

    let tokens = do_refresh_token(state, refresh_token).await;

    let mut con = state
        .redis
        .get_multiplexed_async_connection()
        .await
        .expect("failed to get redis connection");

    let _: () = redis::AsyncCommands::set(
        &mut con,
        REFRESH_TOKEN_KEY,
        tokens.refresh_token.expose_secret(),
    )
    .await
    .expect("failed to set refresh token");

    let _: () = redis::AsyncCommands::set(
        &mut con,
        ACCESS_TOKEN_KEY,
        tokens.access_token.expose_secret(),
    )
    .await
    .expect("failed to set access token");

    Some(AuthTokens {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
    })
}
