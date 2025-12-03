use oauth2::{AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use serde::Deserialize;

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
