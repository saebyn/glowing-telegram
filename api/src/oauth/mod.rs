use oauth2::{RefreshToken, TokenResponse};
use redact::Secret;

use crate::config::Config;

pub mod twitch;
pub mod youtube;

pub struct RedisTokenStorageKeys<'a> {
    pub access_token_key: &'a str,
    pub refresh_token_key: &'a str,
}

pub async fn save_tokens_to_redis<TT>(
    redis_client: &redis::Client,
    token_response: impl TokenResponse<TT>,
    keys: RedisTokenStorageKeys<'_>,
) -> Result<(), String>
where
    TT: oauth2::TokenType,
{
    let access_token = token_response.access_token().secret().to_string();
    let refresh_token = token_response
        .refresh_token()
        .map(|t| t.secret().to_string());

    let mut con = match redis_client.get_multiplexed_async_connection().await {
        Ok(con) => con,
        Err(e) => {
            tracing::error!(
                "failed to get redis connection for login: {:?}",
                e
            );

            return Err("Failed to get redis connection".to_string());
        }
    };

    if let Some(refresh_token) = refresh_token {
        match redis::AsyncCommands::set(
            &mut con,
            keys.refresh_token_key,
            refresh_token,
        )
        .await
        {
            Ok(()) => (),
            Err(e) => {
                tracing::error!(
                    "failed to set refresh token in Redis: {:?}",
                    e
                );

                return Err("Failed to set refresh token in Redis".to_string());
            }
        };
    }

    let access_token_ttl = calculate_access_token_ttl(token_response);

    let set_options = redis::SetOptions::default()
        .with_expiration(redis::SetExpiry::EX(access_token_ttl.as_secs()));

    match redis::AsyncCommands::set_options(
        &mut con,
        keys.access_token_key,
        access_token,
        set_options,
    )
    .await
    {
        Ok(()) => (),
        Err(e) => {
            tracing::error!("failed to set access token in Redis: {:?}", e);

            return Err("Failed to set access token in Redis".to_string());
        }
    };

    Ok(())
}

pub fn calculate_access_token_ttl<TR, TT>(
    token_response: TR,
) -> std::time::Duration
where
    TR: TokenResponse<TT>,
    TT: oauth2::TokenType,
{
    (match token_response.expires_in() {
        Some(duration) => duration,
        None => {
            tracing::debug!("access token duration not found in google oauth response, using 1 hour");
            std::time::Duration::from_secs(3600)
        }
    } - std::time::Duration::from_secs(5))
}

/**
 * Get new access token with the refresh token.
 *
 * This function is called when the access token is not found in Redis.
 * It uses the refresh token to get a new access token and refresh token
 * from the Youtube API.
 */
pub async fn refresh_access_token<TE, TR, TT, TIR, RT, TRE>(
    redis_client: &redis::Client,
    oauth2_client: &oauth2::Client<TE, TR, TT, TIR, RT, TRE>,
    keys: RedisTokenStorageKeys<'_>,
) -> Result<Secret<String>, String>
where
    TE: oauth2::ErrorResponse + 'static,
    TR: oauth2::TokenResponse<TT>,
    TT: oauth2::TokenType,
    TIR: oauth2::TokenIntrospectionResponse<TT>,
    RT: oauth2::RevocableToken,
    TRE: oauth2::ErrorResponse + 'static,
{
    let mut con = match redis_client.get_multiplexed_async_connection().await {
        Ok(con) => con,
        Err(_) => {
            tracing::error!("failed to get redis connection");

            return Err("Failed to get redis connection".to_string());
        }
    };

    let refresh_token: String = match redis::AsyncCommands::get(
        &mut con,
        keys.refresh_token_key,
    )
    .await
    {
        Ok(refresh_token) => refresh_token,
        Err(_) => {
            tracing::error!("failed to get refresh token from Redis");

            return Err("Failed to get refresh token from Redis".to_string());
        }
    };

    let token_response = match oauth2_client
        .exchange_refresh_token(&RefreshToken::new(refresh_token.to_string()))
        .request_async(oauth2::reqwest::async_http_client)
        .await
    {
        Ok(token_response) => token_response,
        Err(_) => {
            tracing::error!("failed to refresh access token");

            return Err("Failed to refresh access token".to_string());
        }
    };

    let access_token = token_response.access_token().secret().to_string();
    let access_token_ttl =
        crate::oauth::calculate_access_token_ttl(token_response);

    let set_options = redis::SetOptions::default()
        .with_expiration(redis::SetExpiry::EX(access_token_ttl.as_secs()));

    match redis::AsyncCommands::set_options(
        &mut con,
        keys.access_token_key,
        access_token.clone(),
        set_options,
    )
    .await
    {
        Ok(()) => (),
        Err(_) => {
            tracing::error!("failed to set access token in Redis");

            return Err("Failed to set access token in Redis".to_string());
        }
    };

    Ok(Secret::new(access_token))
}

pub async fn get_access_token<F, FE, TE, TR, TT, TIR, RT, TRE>(
    redis_client: &redis::Client,
    config: &Config,
    oauth2_client_builder: F,
    keys: RedisTokenStorageKeys<'_>,
) -> Result<Secret<String>, String>
where
    F: FnOnce(&Config) -> Result<oauth2::Client<TE, TR, TT, TIR, RT, TRE>, FE>,
    TE: oauth2::ErrorResponse + 'static,
    TR: oauth2::TokenResponse<TT>,
    TT: oauth2::TokenType,
    TIR: oauth2::TokenIntrospectionResponse<TT>,
    RT: oauth2::RevocableToken,
    TRE: oauth2::ErrorResponse + 'static,
{
    let mut con = match redis_client.get_multiplexed_async_connection().await {
        Ok(con) => con,
        Err(_) => {
            tracing::error!("failed to get redis connection");

            return Err("Failed to get redis connection".to_string());
        }
    };

    let access_token: Result<String, _> =
        redis::AsyncCommands::get(&mut con, keys.access_token_key).await;

    match access_token {
        Ok(access_token) => Ok(Secret::new(access_token)),
        Err(_) => {
            let oauth2_client = match oauth2_client_builder(config) {
                Ok(client) => client,
                Err(_) => {
                    tracing::error!("failed to create OAuth2 client");

                    return Err("Failed to create OAuth2 client".to_string());
                }
            };
            let access_token = match crate::oauth::refresh_access_token(
                &redis_client,
                &oauth2_client,
                keys,
            )
            .await
            {
                Ok(access_token) => access_token,
                Err(_) => {
                    tracing::error!("failed to update refresh token");

                    return Err("Failed to update refresh token".to_string());
                }
            };

            Ok(access_token)
        }
    }
}
