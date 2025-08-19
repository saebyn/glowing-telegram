use axum::{
    Json,
    extract::State,
    http::{StatusCode, header},
    response::IntoResponse,
};
use gt_axum::cognito::CognitoUserId;
use oauth2::{AuthorizationCode, CsrfToken, Scope, TokenResponse};
use serde_json::json;
use tracing::instrument;
use types::{
    AccessTokenResponse, AuthorizationUrlResponse, TwitchAuthRequest,
    TwitchCallbackRequest, TwitchCallbackResponse, TwitchSessionSecret,
};
use serde::{Deserialize, Serialize};

use crate::{structs::AppContext, twitch};

pub async fn obtain_twitch_authorization_url_handler(
    State(state): State<AppContext>,
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
    State(state): State<AppContext>,
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
        Ok(()) => {
            tracing::info!("Tokens stored successfully in secrets manager");
        }
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
    State(state): State<AppContext>,
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

// EventSub subscription request/response structures
#[derive(Debug, Serialize)]
struct EventSubSubscriptionRequest {
    #[serde(rename = "type")]
    event_type: String,
    version: String,
    condition: serde_json::Value,
    transport: EventSubTransport,
}

#[derive(Debug, Serialize)]
struct EventSubTransport {
    method: String,
    callback: String,
    secret: String,
}

#[derive(Debug, Deserialize)]
struct EventSubSubscriptionResponse {
    data: Vec<EventSubSubscription>,
}

#[derive(Debug, Deserialize, Serialize)]
struct EventSubSubscription {
    id: String,
    status: String,
    #[serde(rename = "type")]
    event_type: String,
    version: String,
    condition: serde_json::Value,
    transport: serde_json::Value,
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct EventSubWebhookRequest {
    challenge: Option<String>,
    subscription: Option<EventSubSubscription>,
    event: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubscribeChatRequest {
    webhook_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubscribeChatResponse {
    subscription_id: Option<String>,
    status: String,
}

/// Subscribe to Twitch chat events for the authenticated user's channel
pub async fn subscribe_chat_handler(
    State(_state): State<AppContext>,
    Json(request): Json<SubscribeChatRequest>,
) -> impl IntoResponse {
    tracing::info!("Subscribe chat handler called with webhook_url: {}", request.webhook_url);
    "not implemented yet"
}

/// Handle incoming Twitch EventSub webhooks
#[instrument(skip(state))]
pub async fn eventsub_webhook_handler(
    State(state): State<AppContext>,
    body: String,
) -> impl IntoResponse {
    tracing::info!("EventSub webhook received: {}", body);
    
    // Parse the webhook request to handle challenges and events
    let webhook_request: Result<serde_json::Value, _> = serde_json::from_str(&body);
    
    match webhook_request {
        Ok(json) => {
            // Check if this is a challenge verification
            if let Some(challenge) = json.get("challenge") {
                if let Some(challenge_str) = challenge.as_str() {
                    tracing::info!("Responding to EventSub challenge");
                    return (StatusCode::OK, challenge_str.to_string());
                }
            }
            
            // Check if this is an actual event
            if json.get("subscription").is_some() && json.get("event").is_some() {
                // Send the entire message to SQS for processing
                if let Some(queue_url) = &state.config.chat_queue_url {
                    match state
                        .sqs_client
                        .send_message()
                        .queue_url(queue_url)
                        .message_body(&body)
                        .send()
                        .await
                    {
                        Ok(_) => {
                            tracing::info!("Successfully sent message to SQS");
                            return (StatusCode::NO_CONTENT, "".to_string());
                        }
                        Err(e) => {
                            tracing::error!("Failed to send message to SQS: {:?}", e);
                            return (StatusCode::INTERNAL_SERVER_ERROR, "Failed to process".to_string());
                        }
                    }
                } else {
                    tracing::warn!("No chat queue URL configured");
                    return (StatusCode::INTERNAL_SERVER_ERROR, "No queue configured".to_string());
                }
            }
            
            tracing::warn!("Unhandled webhook request format");
            (StatusCode::BAD_REQUEST, "Invalid request".to_string())
        }
        Err(e) => {
            tracing::error!("Failed to parse webhook request: {:?}", e);
            (StatusCode::BAD_REQUEST, "Invalid JSON".to_string())
        }
    }
}
