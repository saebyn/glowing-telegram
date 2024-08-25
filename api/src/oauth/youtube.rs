use crate::state::AppState;
use axum::extract::FromRequestParts;
use axum::Json;
use axum::{async_trait, http::request::Parts};
use oauth2::{
    basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl,
};
use redact::Secret;
use reqwest::StatusCode;
use serde_json::json;

// Redis key constants
pub const TOKEN_KEYS: crate::oauth::RedisTokenStorageKeys =
    crate::oauth::RedisTokenStorageKeys {
        access_token_key: "youtube:access_token",
        refresh_token_key: "youtube:refresh_token",
    };

/**
 * Extractor for the access token from Redis.
 *
 * This is a simple extractor that gets the access token from Redis
 * and injects it into the request's extensions.
 */
pub struct YouTubeAccessToken(pub Secret<String>);

#[async_trait]
impl FromRequestParts<AppState> for YouTubeAccessToken {
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let access_token = crate::oauth::get_access_token(
            &state.redis_client,
            &state.config,
            get_google_oauth_client,
            TOKEN_KEYS,
        );

        match access_token.await {
            Ok(access_token) => Ok(YouTubeAccessToken(access_token)),
            Err(e) => {
                tracing::error!("failed to get access token: {:?}", e);
                Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(
                        json!({ "error": "failed to get access token" }),
                    ),
                ))
            }
        }
    }
}

pub fn get_google_oauth_client(
    config: &crate::config::Config,
) -> Result<BasicClient, oauth2::url::ParseError> {
    Ok(BasicClient::new(
        ClientId::new(config.youtube_client_id.clone()),
        Some(ClientSecret::new(
            config.youtube_client_secret.expose_secret().to_string(),
        )),
        AuthUrl::new(config.youtube_auth_uri.clone())?,
        Some(TokenUrl::new(config.youtube_token_uri.clone())?),
    )
    .set_redirect_uri(RedirectUrl::new(config.youtube_redirect_url.clone())?))
}
