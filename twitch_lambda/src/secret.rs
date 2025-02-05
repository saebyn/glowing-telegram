use aws_sdk_secretsmanager::client::Client as SecretsManagerClient;
use types::TwitchSessionSecret;

pub const fn new(
    csrf_token: String,
    redirect_url: String,
    scopes: Vec<String>,
) -> TwitchSessionSecret {
    TwitchSessionSecret {
        csrf_token,
        redirect_url,
        scopes,
        access_token: None,
        refresh_token: None,
        valid_until: None,
    }
}

pub async fn create_or_replace(
    secrets_manager: &SecretsManagerClient,
    secret_id: &str,
    secret: &TwitchSessionSecret,
) -> Result<(), String> {
    secrets_manager
        .put_secret_value()
        .secret_id(secret_id)
        .secret_string(
            serde_json::to_string(secret).map_err(|e| e.to_string())?,
        )
        .send()
        .await
        .map_err(|e| {
            tracing::error!("failed to create or replace secret: {:?}", e);
            e.to_string()
        })?;

    Ok(())
}

pub async fn get(
    secrets_manager: &SecretsManagerClient,
    secret_id: &str,
) -> Result<TwitchSessionSecret, String> {
    let secret = secrets_manager
        .get_secret_value()
        .secret_id(secret_id)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("failed to get secret: {:?}", e);
            e.to_string()
        })?;

    let secret_string = secret
        .secret_string
        .ok_or_else(|| "secret string not found".to_string())?;

    serde_json::from_str(&secret_string).map_err(|e| e.to_string())
}

pub async fn set_tokens(
    secrets_manager: &SecretsManagerClient,
    secret_id: &str,
    access_token: &str,
    refresh_token: &str,
    valid_for_duration: Option<std::time::Duration>,
) -> Result<(), String> {
    let secret = get(secrets_manager, secret_id).await?;

    let secret = TwitchSessionSecret {
        access_token: Some(access_token.to_string()),
        refresh_token: Some(refresh_token.to_string()),
        valid_until: valid_for_duration.map(|d| {
            let now = std::time::SystemTime::now();
            (now + d)
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0.0, |d| d.as_secs_f64())
        }),
        ..secret
    };

    create_or_replace(secrets_manager, secret_id, &secret).await
}

pub async fn clear_tokens(
    secrets_manager: &SecretsManagerClient,
    secret_id: &str,
) -> Result<TwitchSessionSecret, String> {
    let secret = get(secrets_manager, secret_id).await?;

    let secret = TwitchSessionSecret {
        access_token: None,
        refresh_token: None,
        valid_until: None,
        ..secret
    };

    create_or_replace(secrets_manager, secret_id, &secret).await?;

    Ok(secret)
}
