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
    AccessTokenResponse, AuthorizationUrlResponse, YouTubeAuthRequest,
    YouTubeCallbackRequest, YouTubeCallbackResponse, YouTubeSessionSecret,
};

use crate::{structs::AppState, youtube};

#[instrument(skip(state))]
pub async fn obtain_youtube_authorization_url_handler(
    State(state): State<AppState>,
    CognitoUserId(cognito_user_id): CognitoUserId,
    Json(request): Json<YouTubeAuthRequest>,
) -> impl IntoResponse {
    let Ok(client) = youtube::get_oauth_client(&state.youtube_credentials)
    else {
        return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
    };

    let (authorize_url, csrf_state) = client
        .authorize_url(CsrfToken::new_random)
        .add_scopes(
            request.scopes.iter().map(|scope| Scope::new(scope.clone())),
        )
        .add_extra_param("access_type", "offline")
        .add_extra_param("prompt", "consent")
        .url();

    // store csrf_state and redirect_url in secrets manager
    match gt_secrets::create_or_replace::<YouTubeSessionSecret>(
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

#[instrument(skip(state))]
pub async fn youtube_callback_handler(
    State(state): State<AppState>,
    CognitoUserId(cognito_user_id): CognitoUserId,
    Json(request): Json<YouTubeCallbackRequest>,
) -> impl IntoResponse {
    let secret_id =
        state.config.user_secret_path.secret_path(&cognito_user_id);

    let Ok(secret) = gt_secrets::get::<YouTubeSessionSecret>(
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

    let client = match youtube::get_oauth_client(&state.youtube_credentials) {
        Ok(client) => client,
        Err(e) => {
            tracing::error!("failed to get YouTube OAuth client: {:?}", e);
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

    match gt_secrets::set_tokens::<YouTubeSessionSecret>(
        &state.secrets_manager,
        &secret_id,
        token_response.access_token().secret(),
        token_response
            .refresh_token()
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

    let response_body = YouTubeCallbackResponse {
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
pub async fn obtain_youtube_access_token_handler(
    State(state): State<AppState>,
    CognitoUserId(cognito_user_id): CognitoUserId,
) -> impl IntoResponse {
    let secret_id =
        state.config.user_secret_path.secret_path(&cognito_user_id);

    let secret = match gt_secrets::get::<YouTubeSessionSecret>(
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

    let response_body = AccessTokenResponse {
        access_token,
        broadcaster_id: String::new(),
    };

    (
        StatusCode::OK,
        [(header::CONTENT_TYPE, "application/json")],
        Json(json!(response_body)),
    )
        .into_response()
}
