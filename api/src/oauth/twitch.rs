use crate::state::AppState;
use axum::extract::FromRequestParts;
use axum::Json;
use axum::{async_trait, http::request::Parts};
use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use redact::Secret;
use reqwest::StatusCode;
use serde_json::json;

// Redis key constants
pub const TOKEN_KEYS: crate::oauth::RedisTokenStorageKeys =
    crate::oauth::RedisTokenStorageKeys {
        access_token_key: "twitch:access_token",
        refresh_token_key: "twitch:refresh_token",
    };

/**
 * Extractor for the access token from Redis.
 *
 * This is a simple extractor that gets the access token from Redis
 * and injects it into the request's extensions.
 */
pub struct TwitchAccessToken(pub Secret<String>);

#[async_trait]
impl FromRequestParts<AppState> for TwitchAccessToken {
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let access_token = crate::oauth::get_access_token(
            &state.redis_client,
            &state.config,
            get_twitch_oauth_client,
            TOKEN_KEYS,
        );

        match access_token.await {
            Ok(access_token) => Ok(TwitchAccessToken(access_token)),
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

// implement the TokenResponse trait for a custom struct that matches Twitch's response.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct TwitchTokenResponse<TT> {
    access_token: oauth2::AccessToken,
    refresh_token: Option<oauth2::RefreshToken>,
    token_type: TT,

    #[serde(skip_serializing_if = "Option::is_none")]
    expires_in: Option<u64>,

    #[serde(rename = "scope")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    scopes: Option<Vec<oauth2::Scope>>,
}

impl<TT> oauth2::TokenResponse<TT> for TwitchTokenResponse<TT>
where
    TT: oauth2::TokenType,
{
    fn access_token(&self) -> &oauth2::AccessToken {
        &self.access_token
    }

    fn refresh_token(&self) -> Option<&oauth2::RefreshToken> {
        self.refresh_token.as_ref()
    }

    fn token_type(&self) -> &TT {
        &self.token_type
    }

    fn expires_in(&self) -> Option<std::time::Duration> {
        self.expires_in.map(std::time::Duration::from_secs)
    }

    fn scopes(&self) -> Option<&Vec<oauth2::Scope>> {
        self.scopes.as_ref()
    }
}

type Client = oauth2::Client<
    oauth2::basic::BasicErrorResponse,
    TwitchTokenResponse<oauth2::basic::BasicTokenType>,
    oauth2::basic::BasicTokenType,
    oauth2::basic::BasicTokenIntrospectionResponse,
    oauth2::revocation::StandardRevocableToken,
    oauth2::basic::BasicRevocationErrorResponse,
>;

pub fn get_twitch_oauth_client(
    config: &crate::config::Config,
) -> Result<Client, oauth2::url::ParseError> {
    Ok(oauth2::Client::new(
        ClientId::new(config.twitch_client_id.clone()),
        Some(ClientSecret::new(
            config.twitch_client_secret.expose_secret().to_string(),
        )),
        AuthUrl::new("https://id.twitch.tv/oauth2/authorize".to_string())?,
        Some(TokenUrl::new(
            "https://id.twitch.tv/oauth2/token".to_string(),
        )?),
    )
    .set_auth_type(oauth2::AuthType::RequestBody)
    .set_redirect_uri(RedirectUrl::new(config.twitch_redirect_url.clone())?))
}
