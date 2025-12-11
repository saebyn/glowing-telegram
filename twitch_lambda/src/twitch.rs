use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use serde::Deserialize;
use sha2::Sha256;

use hmac::{Hmac, Mac};
use reqwest::header::HeaderMap;
use types::{EventSubSubscription, TwitchSessionSecret};

// implement the TokenResponse trait for a custom struct that matches Twitch's response.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct TokenResponse<TT> {
    pub access_token: oauth2::AccessToken,
    pub refresh_token: Option<oauth2::RefreshToken>,
    token_type: TT,

    #[serde(skip_serializing_if = "Option::is_none")]
    expires_in: Option<u64>,

    #[serde(rename = "scope")]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    scopes: Option<Vec<oauth2::Scope>>,
}

impl<TT> oauth2::TokenResponse<TT> for TokenResponse<TT>
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
    TokenResponse<oauth2::basic::BasicTokenType>,
    oauth2::basic::BasicTokenType,
    oauth2::basic::BasicTokenIntrospectionResponse,
    oauth2::revocation::StandardRevocableToken,
    oauth2::basic::BasicRevocationErrorResponse,
>;

pub fn get_oauth_client(
    config: &crate::structs::TwitchCredentials,
) -> Result<Client, oauth2::url::ParseError> {
    let client = oauth2::Client::new(
        ClientId::new(config.id.to_string()),
        Some(ClientSecret::new(config.secret.expose_secret().to_string())),
        AuthUrl::new("https://id.twitch.tv/oauth2/authorize".to_string())?,
        Some(TokenUrl::new(
            "https://id.twitch.tv/oauth2/token".to_string(),
        )?),
    )
    .set_auth_type(oauth2::AuthType::RequestBody)
    .set_redirect_uri(RedirectUrl::new(config.redirect_url.to_string())?);

    Ok(client)
}

#[derive(Deserialize, Debug)]
pub struct ValidationResponse {
    pub user_id: String,
}

pub async fn validate_token(
    token: &str,
) -> Result<ValidationResponse, reqwest::Error> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://id.twitch.tv/oauth2/validate")
        .header("Authorization", format!("OAuth {token}"))
        .send()
        .await?
        .error_for_status()?;

    let body = response.json::<ValidationResponse>().await?;

    Ok(body)
}

/// Obtain an app access token using the client credentials flow.
/// This is required for creating `EventSub` webhook subscriptions.
pub async fn get_app_access_token(
    credentials: &crate::structs::TwitchCredentials,
) -> Result<String, AppAccessTokenError> {
    use oauth2::TokenResponse;

    let client = get_oauth_client(credentials)?;

    let token_response = client
        .exchange_client_credentials()
        .request_async(oauth2::reqwest::async_http_client)
        .await
        .map_err(|e| AppAccessTokenError::TokenRequest(format!("{e:?}")))?;

    Ok(token_response.access_token().secret().clone())
}

#[derive(Debug)]
pub enum AppAccessTokenError {
    UrlParse(oauth2::url::ParseError),
    TokenRequest(String),
}

impl std::fmt::Display for AppAccessTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UrlParse(err) => write!(f, "URL parse error: {err}"),
            Self::TokenRequest(msg) => {
                write!(f, "Token request error: {msg}")
            }
        }
    }
}

impl std::error::Error for AppAccessTokenError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::UrlParse(err) => Some(err),
            Self::TokenRequest(_) => None,
        }
    }
}

impl From<oauth2::url::ParseError> for AppAccessTokenError {
    fn from(err: oauth2::url::ParseError) -> Self {
        Self::UrlParse(err)
    }
}

/// Verify Twitch EventSub webhook signature
pub fn verify_webhook_signature(
    headers: &HeaderMap,
    body: &str,
    secret: &str,
) -> Result<(), String> {
    // Get required headers
    let message_id = headers
        .get("twitch-eventsub-message-id")
        .and_then(|h| h.to_str().ok())
        .ok_or("Missing Twitch-Eventsub-Message-Id header")?;

    let timestamp = headers
        .get("twitch-eventsub-message-timestamp")
        .and_then(|h| h.to_str().ok())
        .ok_or("Missing Twitch-Eventsub-Message-Timestamp header")?;

    let signature = headers
        .get("twitch-eventsub-message-signature")
        .and_then(|h| h.to_str().ok())
        .ok_or("Missing Twitch-Eventsub-Message-Signature header")?;

    // Remove "sha256=" prefix from signature
    let signature = signature
        .strip_prefix("sha256=")
        .ok_or("Invalid signature format")?;

    // Create HMAC message: message_id + timestamp + body
    let message = format!("{}{}{}", message_id, timestamp, body);

    // Create HMAC-SHA256
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| format!("HMAC key error: {}", e))?;
    mac.update(message.as_bytes());

    let signature_bytes = hex::decode(signature)
        .map_err(|e| format!("Hex decode error: {}", e))?;

    // Compare signatures
    if mac.verify_slice(&signature_bytes).is_ok() {
        Ok(())
    } else {
        Err("Signature mismatch".to_string())
    }
}

#[derive(Deserialize)]
struct Pagination {
    pub cursor: Option<String>,
}

#[derive(Deserialize)]
struct EventSubSubscriptionsResponse {
    pub data: Vec<EventSubSubscription>,
    pub pagination: Option<Pagination>,
}

pub async fn get_user_eventsub_subscriptions(
    access_token: &str,
    client_id: &str,
    user_id: &str,
) -> Result<Vec<EventSubSubscription>, reqwest::Error> {
    let client = reqwest::Client::new();
    let mut subscriptions = Vec::new();
    let mut cursor: Option<String> = None;

    loop {
        let mut request = client
            .get("https://api.twitch.tv/helix/eventsub/subscriptions")
            .header("Authorization", format!("Bearer {}", access_token))
            .header("Client-Id", client_id)
            .query(&[("user_id", user_id)]);

        if let Some(ref c) = cursor {
            request = request.query(&[("after", c)]);
        }

        let response = request.send().await?.error_for_status()?;
        let body: EventSubSubscriptionsResponse = response.json().await?;

        subscriptions.extend(body.data);

        if let Some(pagination) = body.pagination {
            if let Some(next_cursor) = pagination.cursor {
                cursor = Some(next_cursor);
            } else {
                break;
            }
        } else {
            break;
        }
    }

    Ok(subscriptions)
}

pub async fn get_twitch_user(
    state: &crate::structs::AppContext,
    cognito_user_id: &str,
) -> Result<ValidationResponse, ()> {
    // Get the user's access token to validate and get broadcaster_id
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
            return Err(());
        }
    };

    let Some(access_token) = secret.access_token else {
        tracing::warn!("access_token not found in secret");
        return Err(());
    };

    // Validate token and get broadcaster_id
    let validation_response = match validate_token(&access_token).await {
        Ok(response) => response,
        Err(e) => {
            tracing::error!("failed to validate token: {:?}", e);
            return Err(());
        }
    };

    Ok(validation_response)
}

pub async fn delete_eventsub_subscription(
    access_token: &str,
    client_id: &str,
    subscription_id: &str,
) -> Result<(), &'static str> {
    let client = reqwest::Client::new();

    let response = match client
        .delete("https://api.twitch.tv/helix/eventsub/subscriptions")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Client-Id", client_id)
        .query(&[("id", subscription_id)])
        .send()
        .await
    {
        Err(e) => {
            tracing::error!("failed to send delete request: {:?}", e);
            return Err("Request error");
        }
        Ok(response) => response,
    };

    if response.status().is_success() {
        Ok(())
    } else {
        tracing::error!(
            "failed to delete subscription: {}",
            response.status()
        );
        Err("Failed to delete subscription")
    }
}
