use axum::{
    Json,
    extract::State,
    http::{StatusCode, header, HeaderMap},
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

#[derive(Debug, Serialize)]
struct SubscribeChatRequest {
    webhook_url: String,
}

#[derive(Debug, Serialize)]
struct SubscribeChatResponse {
    subscription_id: Option<String>,
    status: String,
}

/// Subscribe to Twitch chat events for the authenticated user's channel
#[instrument(skip(state))]
pub async fn subscribe_chat_handler(
    State(state): State<AppContext>,
    CognitoUserId(cognito_user_id): CognitoUserId,
    Json(request): Json<SubscribeChatRequest>,
) -> Json<serde_json::Value> {
    // Get user's Twitch tokens
    let secret_id = state.config.user_secret_path.secret_path(&cognito_user_id);
    
    let Ok(secret) = gt_secrets::get::<TwitchSessionSecret>(
        &state.secrets_manager,
        &secret_id,
    )
    .await
    else {
        tracing::error!("failed to get user's Twitch secret");
        return Json(json!({"error": "missing Twitch secret"}));
    };

    let Some(access_token) = secret.access_token else {
        tracing::error!("user has no Twitch access token");
        return Json(json!({"error": "no access token"}));
    };

    // Validate token and get user info
    let Ok(validation) = twitch::validate_token(&access_token).await else {
        tracing::error!("failed to validate Twitch token");
        return Json(json!({"error": "invalid token"}));
    };

    let broadcaster_id = validation.user_id;

    // Create EventSub subscription
    let subscription = EventSubSubscriptionRequest {
        event_type: "channel.chat.message".to_string(),
        version: "1".to_string(),
        condition: json!({
            "broadcaster_user_id": broadcaster_id,
            "user_id": broadcaster_id
        }),
        transport: EventSubTransport {
            method: "webhook".to_string(),
            callback: request.webhook_url,
            secret: uuid::Uuid::now_v7().to_string(),
        },
    };

    let client = reqwest::Client::new();
    let response = client
        .post("https://api.twitch.tv/helix/eventsub/subscriptions")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Client-Id", &state.twitch_credentials.id)
        .header("Content-Type", "application/json")
        .json(&subscription)
        .send()
        .await;

    match response {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(sub_response) = resp.json::<EventSubSubscriptionResponse>().await {
                if let Some(sub) = sub_response.data.first() {
                    let response = SubscribeChatResponse {
                        subscription_id: Some(sub.id.clone()),
                        status: sub.status.clone(),
                    };
                    
                    Json(json!(response))
                } else {
                    tracing::error!("no subscription data in response");
                    Json(json!({"error": "no subscription data"}))
                }
            } else {
                tracing::error!("failed to parse subscription response");
                Json(json!({"error": "failed to parse response"}))
            }
        }
        Ok(resp) => {
            tracing::error!("Twitch API error: status {}", resp.status());
            Json(json!({"error": "Twitch API error"}))
        }
        Err(e) => {
            tracing::error!("failed to call Twitch API: {:?}", e);
            Json(json!({"error": "API call failed"}))
        }
    }
}

/// Handle incoming Twitch EventSub webhooks
#[instrument(skip(_state))]
pub async fn eventsub_webhook_handler(
    State(_state): State<AppContext>,
    headers: HeaderMap,
    body: String,
) -> impl IntoResponse {
    // Verify the webhook signature
    let signature = headers.get("Twitch-Eventsub-Message-Signature");
    let timestamp = headers.get("Twitch-Eventsub-Message-Timestamp");
    let message_id = headers.get("Twitch-Eventsub-Message-Id");

    if signature.is_none() || timestamp.is_none() || message_id.is_none() {
        tracing::warn!("missing required Twitch headers");
        return (StatusCode::BAD_REQUEST,).into_response();
    }

    // For now, we'll skip signature verification as we need the webhook secret
    // In a real implementation, we'd verify using HMAC-SHA256

    // Parse the request
    let webhook_request: EventSubWebhookRequest = match serde_json::from_str(&body) {
        Ok(req) => req,
        Err(e) => {
            tracing::error!("failed to parse webhook request: {:?}", e);
            return (StatusCode::BAD_REQUEST,).into_response();
        }
    };

    // Handle challenge verification
    if let Some(challenge) = webhook_request.challenge {
        tracing::info!("responding to EventSub challenge");
        return (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "text/plain")],
            challenge,
        ).into_response();
    }

    // Handle actual events
    if let (Some(subscription), Some(event)) = (webhook_request.subscription, webhook_request.event) {
        tracing::info!("received EventSub event: {}", subscription.event_type);
        
        // Send to SQS for processing
        let message_body = json!({
            "subscription": subscription,
            "event": event
        }).to_string();

        // Here we would send to SQS, but for now just log
        tracing::info!("would send to SQS: {}", message_body);
        
        return (StatusCode::NO_CONTENT,).into_response();
    }

    tracing::warn!("unhandled webhook request");
    (StatusCode::BAD_REQUEST,).into_response()
}
