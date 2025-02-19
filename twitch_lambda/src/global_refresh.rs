use aws_sdk_secretsmanager::types::{
    FilterNameStringType, builders::FilterBuilder,
};
use oauth2::TokenResponse;
use types::TwitchSessionSecret;

use crate::{structs::AppState, twitch};

/// Refresh the user's Twitch access token using the refresh token
/// stored in the secrets manager.
/// This endpoint is intended to be called internally via an `EventBridge`
/// scheduled event to keep all users' tokens up to date.
/// The user's Twitch tokens are stored in the secrets manager
/// under the path `user_secret_path`.
pub async fn refresh_user_tokens(state: AppState) -> Result<(), &'static str> {
    // iterate over all the secrets in the secrets manager under the
    // `user_secret_path` prefix, iterating while there are more pages
    // of secrets to fetch.
    let mut next_token = None;
    loop {
        let list_secrets_response = match state
            .secrets_manager
            .list_secrets()
            .set_next_token(next_token)
            .filters(
                FilterBuilder::default()
                    .key(FilterNameStringType::Name)
                    .values(state.config.user_secret_path.0.clone())
                    .build(),
            )
            .send()
            .await
        {
            Ok(response) => response,
            Err(e) => {
                tracing::error!("failed to list secrets: {:?}", e);
                return Err("failed to list secrets");
            }
        };

        let Some(secrets_list) = list_secrets_response.secret_list else {
            tracing::info!("no secrets found");
            break;
        };

        for secret in secrets_list {
            let secret_id = secret.name.as_deref().unwrap_or("");
            let secret = match gt_secrets::get::<TwitchSessionSecret>(
                &state.secrets_manager,
                secret_id,
            )
            .await
            {
                Ok(secret) => secret,
                Err(e) => {
                    tracing::error!("failed to get secret: {:?}", e);
                    continue;
                }
            };

            let Some(access_token) = secret.access_token else {
                tracing::warn!("access_token not found in secret");
                continue;
            };

            let Some(refresh_token) = secret.refresh_token else {
                tracing::warn!("refresh_token not found in secret");
                continue;
            };

            // first, use the validation endpoint to check if the access token is still valid
            // this also satisfies the requirement to check the token every hour
            // (assuming that this function is called at least once an hour)
            if (twitch::validate_token(&access_token).await).is_ok() {
                continue;
            }

            // if the token is invalid, try to refresh it
            match do_refresh(&state, secret_id, refresh_token).await {
                Ok(()) => (),
                Err(e) => {
                    tracing::error!(
                        "failed to store in secrets manager: {:?}",
                        e
                    );
                    continue;
                }
            };
        }

        next_token = list_secrets_response.next_token;
        if next_token.is_none() {
            break;
        }

        // sleep for a short time to avoid throttling
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        tracing::info!("fetching next page of secrets");
    }

    Ok(())
}

async fn do_refresh(
    state: &AppState,
    secret_id: &str,
    refresh_token: String,
) -> Result<(), String> {
    let client = twitch::get_oauth_client(&state.twitch_credentials)
        .map_err(|e| e.to_string())?;

    let token_response = client
        .exchange_refresh_token(&oauth2::RefreshToken::new(refresh_token))
        .request_async(oauth2::reqwest::async_http_client)
        .await
        .map_err(|e| e.to_string())?;

    gt_secrets::set_tokens::<TwitchSessionSecret>(
        &state.secrets_manager,
        secret_id,
        token_response.access_token.secret(),
        token_response
            .refresh_token
            .as_ref()
            .map_or("", |t| t.secret()),
        token_response.expires_in(),
    )
    .await?;

    Ok(())
}
