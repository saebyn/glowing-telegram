use axum::{
    Json,
    extract::State,
    http::{StatusCode, header},
    response::IntoResponse,
};
use gt_axum::cognito::CognitoUserId;
use oauth2::{AuthorizationCode, CsrfToken, Scope, TokenResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tracing::instrument;
use types::{
    AccessTokenResponse, AuthorizationUrlResponse,
    ChatSubscriptionStatusResponse, EventSubSubscription,
    SubscribeChatRequest, SubscribeChatResponse, TwitchAuthRequest,
    TwitchCallbackRequest, TwitchCallbackResponse, TwitchSessionSecret,
};

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

#[derive(Debug, Deserialize)]
struct EventSubWebhookRequest {
    challenge: Option<String>,
    subscription: Option<EventSubSubscription>,
    event: Option<serde_json::Value>,
}

/// Subscribe to Twitch chat events for the authenticated user's channel
pub async fn subscribe_chat_handler(
    State(state): State<AppContext>,
    CognitoUserId(cognito_user_id): CognitoUserId,
    Json(request): Json<SubscribeChatRequest>,
) -> impl IntoResponse {
    tracing::info!(
        "Subscribe chat handler called with webhook_url: {}",
        request.webhook_url
    );

    // Get the user's access token
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
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!(SubscribeChatResponse {
                    subscription_id: None,
                    status: "error".to_string(),
                })),
            )
                .into_response();
        }
    };

    let Some(access_token) = secret.access_token else {
        tracing::warn!("access_token not found in secret");
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!(SubscribeChatResponse {
                subscription_id: None,
                status: "unauthorized".to_string(),
            })),
        )
            .into_response();
    };

    // Validate token and get broadcaster_id
    let validation_response = match twitch::validate_token(&access_token).await
    {
        Ok(response) => response,
        Err(e) => {
            tracing::error!("Failed to validate access token: {:?}", e);
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!(SubscribeChatResponse {
                    subscription_id: None,
                    status: "invalid_token".to_string(),
                })),
            )
                .into_response();
        }
    };

    // Create EventSub subscription request
    let subscription_request = EventSubSubscriptionRequest {
        event_type: "channel.chat.message".to_string(),
        version: "1".to_string(),
        condition: json!({
            "broadcaster_user_id": validation_response.user_id,
            "user_id": validation_response.user_id
        }),
        transport: EventSubTransport {
            method: "webhook".to_string(),
            callback: request.webhook_url,
            secret: match std::env::var("EVENTSUB_SECRET") {
                Ok(secret) => secret,
                Err(_) => {
                    tracing::error!(
                        "EVENTSUB_SECRET environment variable not set"
                    );
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!(SubscribeChatResponse {
                            subscription_id: None,
                            status: "missing_eventsub_secret".to_string(),
                        })),
                    )
                        .into_response();
                }
            },
        },
    };

    // Make the subscription request to Twitch
    let client = reqwest::Client::new();
    let response = match client
        .post("https://api.twitch.tv/helix/eventsub/subscriptions")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Client-Id", &state.twitch_credentials.id)
        .header("Content-Type", "application/json")
        .json(&subscription_request)
        .send()
        .await
    {
        Ok(response) => response,
        Err(e) => {
            tracing::error!("Failed to make subscription request: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!(SubscribeChatResponse {
                    subscription_id: None,
                    status: "request_failed".to_string(),
                })),
            )
                .into_response();
        }
    };

    if !response.status().is_success() {
        let status_code = response.status();
        let error_body = response.text().await.unwrap_or_default();
        tracing::error!("Twitch API error {}: {}", status_code, error_body);
        return (
            StatusCode::BAD_REQUEST,
            Json(json!(SubscribeChatResponse {
                subscription_id: None,
                status: format!("twitch_error_{}", status_code.as_u16()),
            })),
        )
            .into_response();
    }

    let subscription_response: EventSubSubscriptionResponse = match response
        .json()
        .await
    {
        Ok(response) => response,
        Err(e) => {
            tracing::error!("Failed to parse subscription response: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!(SubscribeChatResponse {
                    subscription_id: None,
                    status: "parse_error".to_string(),
                })),
            )
                .into_response();
        }
    };

    if let Some(subscription) = subscription_response.data.first() {
        tracing::info!(
            "Created subscription: {} with status: {}",
            subscription.id,
            subscription.status
        );
        (
            StatusCode::OK,
            Json(json!(SubscribeChatResponse {
                subscription_id: Some(subscription.id.clone()),
                status: subscription.status.clone(),
            })),
        )
            .into_response()
    } else {
        tracing::error!("No subscription data in response");
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!(SubscribeChatResponse {
                subscription_id: None,
                status: "no_data".to_string(),
            })),
        )
            .into_response()
    }
}

/// Handle incoming Twitch EventSub webhooks
#[instrument(skip(state))]
pub async fn eventsub_webhook_handler(
    State(state): State<AppContext>,
    body: String,
) -> impl IntoResponse {
    tracing::info!("EventSub webhook received: {}", body);

    // Parse the webhook request to handle challenges and events
    let webhook_request: Result<serde_json::Value, _> =
        serde_json::from_str(&body);

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
            if json.get("subscription").is_some()
                && json.get("event").is_some()
            {
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
                            tracing::error!(
                                "Failed to send message to SQS: {:?}",
                                e
                            );
                            return (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Failed to process".to_string(),
                            );
                        }
                    }
                } else {
                    tracing::warn!("No chat queue URL configured");
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "No queue configured".to_string(),
                    );
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

/// Check the status of EventSub chat subscriptions for the authenticated user
#[instrument(skip(state))]
pub async fn chat_subscription_status_handler(
    State(state): State<AppContext>,
    CognitoUserId(cognito_user_id): CognitoUserId,
) -> impl IntoResponse {
    tracing::info!("Chat subscription status handler called");

    // Get the user's access token
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
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!(ChatSubscriptionStatusResponse {
                    has_active_subscription: false,
                    subscriptions: vec![],
                })),
            )
                .into_response();
        }
    };

    let Some(access_token) = secret.access_token else {
        tracing::warn!("access_token not found in secret");
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!(ChatSubscriptionStatusResponse {
                has_active_subscription: false,
                subscriptions: vec![],
            })),
        )
            .into_response();
    };

    // Validate token and get broadcaster_id
    let validation_response = match twitch::validate_token(&access_token).await
    {
        Ok(response) => response,
        Err(e) => {
            tracing::error!("Failed to validate access token: {:?}", e);
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!(ChatSubscriptionStatusResponse {
                    has_active_subscription: false,
                    subscriptions: vec![],
                })),
            )
                .into_response();
        }
    };

    // Get EventSub subscriptions from Twitch
    let client = reqwest::Client::new();
    let response = match client
        .get("https://api.twitch.tv/helix/eventsub/subscriptions")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Client-Id", &state.twitch_credentials.id)
        .query(&[("type", "channel.chat.message")])
        .send()
        .await
    {
        Ok(response) => response,
        Err(e) => {
            tracing::error!("Failed to get subscriptions: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!(ChatSubscriptionStatusResponse {
                    has_active_subscription: false,
                    subscriptions: vec![],
                })),
            )
                .into_response();
        }
    };

    if !response.status().is_success() {
        let status_code = response.status();
        let error_body = response.text().await.unwrap_or_default();
        tracing::error!("Twitch API error {}: {}", status_code, error_body);
        return (
            StatusCode::BAD_REQUEST,
            Json(json!(ChatSubscriptionStatusResponse {
                has_active_subscription: false,
                subscriptions: vec![],
            })),
        )
            .into_response();
    }

    let subscription_response: EventSubSubscriptionResponse = match response
        .json()
        .await
    {
        Ok(response) => response,
        Err(e) => {
            tracing::error!("Failed to parse subscription response: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!(ChatSubscriptionStatusResponse {
                    has_active_subscription: false,
                    subscriptions: vec![],
                })),
            )
                .into_response();
        }
    };

    // Filter subscriptions for this user's channel and enabled status
    let user_id = &validation_response.user_id;
    let active_subscriptions: Vec<EventSubSubscription> =
        subscription_response
            .data
            .into_iter()
            .filter(|sub| {
                // Check if this subscription is for the user's channel
                sub.condition.broadcaster_user_id == Some(user_id)
                    && sub.status == "enabled"
            })
            .collect();

    let has_active = !active_subscriptions.is_empty();

    tracing::info!(
        "Found {} active chat subscriptions for user {}",
        active_subscriptions.len(),
        user_id
    );

    (
        StatusCode::OK,
        Json(json!(ChatSubscriptionStatusResponse {
            has_active_subscription: has_active,
            subscriptions: active_subscriptions,
        })),
    )
        .into_response()
}
