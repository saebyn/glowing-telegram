use axum::extract::{Query, State};
use axum::http::header;
use axum::Json;
use axum::{http::StatusCode, response::IntoResponse};
use oauth2::{AuthorizationCode, CsrfToken, Scope};
use serde::Deserialize;
use serde_json::json;
use tracing::instrument;

use crate::oauth::refresh_access_token;
use crate::oauth::twitch::{get_twitch_oauth_client, TwitchAccessToken};
use crate::state::AppState;

#[instrument]
pub async fn get_login_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    let client = match get_twitch_oauth_client(&state.config) {
        Ok(client) => client,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR,).into_response(),
    };

    let (authorize_url, csrf_state) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("chat:read".to_string()))
        .url();

    // TODO store csrf_state in Redis

    (StatusCode::OK, Json(json!({ "url": authorize_url }))).into_response()
}

#[derive(Deserialize, Debug)]
pub struct AuthCode {
    code: AuthorizationCode,
}

/**
 * POST /login
 *
 * Completes the OAuth flow by exchanging the code for an access token
 * and refresh token. Stores the tokens in Redis and returns a
 * 202 Accepted response.
 */
#[instrument]
pub async fn post_login_handler(
    State(state): State<AppState>,
    Json(body): Json<AuthCode>,
) -> impl IntoResponse {
    // TODO validate CSRF state

    let oauth_client = match get_twitch_oauth_client(&state.config) {
        Ok(client) => client,
        Err(e) => {
            tracing::error!("failed to get Twitch OAuth client: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "internal server error" })),
            )
                .into_response();
        }
    };

    let token_response = match oauth_client
        .exchange_code(body.code)
        .request_async(oauth2::reqwest::async_http_client)
        .await
    {
        Ok(token) => token,
        Err(e) => {
            tracing::error!("failed to exchange code: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "internal server error" })),
            )
                .into_response();
        }
    };

    match crate::oauth::save_tokens_to_redis(
        &state.redis_client,
        token_response,
        crate::oauth::youtube::TOKEN_KEYS,
    )
    .await
    {
        Ok(()) => (),
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR,).into_response(),
    };

    (StatusCode::ACCEPTED,).into_response()
}

#[derive(Deserialize, Debug)]
pub struct ListVideosQuery {
    after: Option<String>,
}

#[instrument]
pub async fn list_videos_handler(
    State(state): State<AppState>,
    TwitchAccessToken(access_token): TwitchAccessToken,
    Query(params): Query<ListVideosQuery>,
) -> impl IntoResponse {
    let url = format!(
        "https://api.twitch.tv/helix/videos?user_id={}&after={}",
        state.config.twitch_user_id,
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
        .header("Client-Id", state.config.twitch_client_id.clone())
        .send()
        .await;

    let response = match request {
        Ok(response) => match response.status() {
            StatusCode::OK => response,
            StatusCode::UNAUTHORIZED => {
                tracing::trace!("refreshing token");
                let oauth_client = match get_twitch_oauth_client(&state.config)
                {
                    Ok(client) => client,
                    Err(e) => {
                        tracing::error!(
                            "failed to get Twitch OAuth client: {:?}",
                            e
                        );
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({ "error": "internal server error" })),
                        )
                            .into_response();
                    }
                };
                let token = match refresh_access_token(
                    &state.redis_client,
                    &oauth_client,
                    crate::oauth::twitch::TOKEN_KEYS,
                )
                .await
                {
                    Ok(token) => token,
                    Err(e) => {
                        tracing::error!(
                            "failed to refresh access token: {:?}",
                            e
                        );
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
                        format!("Bearer {}", token.expose_secret()),
                    )
                    .header("Client-Id", state.config.twitch_client_id.clone())
                    .send()
                    .await;

                match request {
                    Ok(response) => response,
                    Err(_e) => {
                        tracing::error!(
                            "failed to get videos after refreshing token"
                        );
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
                return (
                    status,
                    Json(json!({ "error": "failed to get videos" })),
                )
                    .into_response();
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
