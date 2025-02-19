use aws_sdk_secretsmanager::client::Client as SecretsManagerClient;
use serde::{Deserialize, Serialize};
use types::TwitchSessionSecret;

/// Trait for session secrets
/// Trait for managing session secrets, providing methods to create, set, and clear tokens.
///
/// # Usage
/// Implement this trait for any type that needs to manage session secrets.
/// The type must be serializable and deserializable using serde.
///
/// # Methods
/// - `new`: Creates a new session secret with the given CSRF token, redirect URL, and scopes.
/// - `set_tokens`: Sets the access token, refresh token, and validity duration for the session secret.
/// - `clear_tokens`: Clears the access token, refresh token, and validity duration for the session secret.
pub trait SessionSecret: Serialize + for<'de> Deserialize<'de> {
    fn new(
        csrf_token: String,
        redirect_url: String,
        scopes: Vec<String>,
    ) -> Self;
    fn set_tokens(
        &mut self,
        access_token: String,
        refresh_token: String,
        valid_until: Option<f64>,
    );
    fn clear_tokens(&mut self);
}

impl SessionSecret for TwitchSessionSecret {
    fn new(
        csrf_token: String,
        redirect_url: String,
        scopes: Vec<String>,
    ) -> Self {
        Self {
            csrf_token,
            redirect_url,
            scopes,
            access_token: None,
            refresh_token: None,
            valid_until: None,
        }
    }

    fn set_tokens(
        &mut self,
        access_token: String,
        refresh_token: String,
        valid_until: Option<f64>,
    ) {
        self.access_token = Some(access_token);
        self.refresh_token = Some(refresh_token);
        self.valid_until = valid_until;
    }

    fn clear_tokens(&mut self) {
        self.access_token = None;
        self.refresh_token = None;
        self.valid_until = None;
    }
}

/// Create or replace a secret in the secrets manager.
///
/// This function asynchronously creates or replaces a secret in the AWS Secrets Manager.
/// It serializes the provided secret and stores it under the given secret ID.
///
/// # Arguments
///
/// * `secrets_manager` - The secrets manager client
/// * `secret_id` - The secret id
/// * `secret` - The secret to store
///
/// # Returns
///
/// * `Result<(), String>` - The result of the operation
///
/// # Errors
///
/// * `String` - The error message if the operation fails. This error may provide additional context on the failure.
///
pub async fn create_or_replace<T: SessionSecret + Send + Sync>(
    secrets_manager: &SecretsManagerClient,
    secret_id: &str,
    secret: &T,
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

/// Get a secret from the secrets manager.
///
/// This function asynchronously retrieves a secret from the AWS Secrets Manager.
/// It deserializes the secret string into the specified type `T`.
///
/// # Arguments
///
/// * `secrets_manager` - The secrets manager client
/// * `secret_id` - The secret id
///
/// # Returns
///
/// * `Result<T, String>` - The secret
///
/// # Errors
///
/// * `String` - The error message if the operation fails. This error may provide additional context on the failure.
///
pub async fn get<T: SessionSecret>(
    secrets_manager: &SecretsManagerClient,
    secret_id: &str,
) -> Result<T, String> {
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

/// Set tokens in a secret.
///
/// This function asynchronously sets the access and refresh tokens in the
/// specified secret. It retrieves the current secret, updates the tokens,
/// and then stores the updated secret back in the AWS Secrets Manager. The
/// function also calculates the validity duration for the tokens if provided.
///
/// # Arguments
///
/// * `secrets_manager` - The secrets manager client.
/// * `secret_id` - The secret id.
/// * `access_token` - The access token.
/// * `refresh_token` - The refresh token.
/// * `valid_for_duration` - The duration for which the tokens are valid.
///
/// # Returns
///
/// * `Result<(), String>` - The result of the operation.
///
/// # Errors
///
/// * `String` - The error message if the operation fails. This error may
///   provide additional context on the failure.
///
pub async fn set_tokens<T: SessionSecret + Send + Sync>(
    secrets_manager: &SecretsManagerClient,
    secret_id: &str,
    access_token: &str,
    refresh_token: &str,
    valid_for_duration: Option<std::time::Duration>,
) -> Result<(), String> {
    let mut secret = get::<T>(secrets_manager, secret_id).await?;

    let valid_until = calculate_valid_until(valid_for_duration);

    secret.set_tokens(
        access_token.to_string(),
        refresh_token.to_string(),
        valid_until,
    );

    create_or_replace(secrets_manager, secret_id, &secret).await
}

/// Clear tokens in a secret.
///
/// This function asynchronously clears the tokens in the specified secret
/// stored in the AWS Secrets Manager. It retrieves the secret, clears the
/// tokens, and then updates the secret in the secrets manager.
///
/// # Arguments
///
/// * `secrets_manager` - The secrets manager client
/// * `secret_id` - The secret id
///
/// # Returns
///
/// * `Result<T, String>` - The secret with cleared tokens
///
/// # Errors
///
/// * `String` - The error message if the operation fails. This error
///   may provide additional context on the failure.
///
pub async fn clear_tokens<T: SessionSecret + Send + Sync>(
    secrets_manager: &SecretsManagerClient,
    secret_id: &str,
) -> Result<T, String> {
    let mut secret = get::<T>(secrets_manager, secret_id).await?;

    secret.clear_tokens();

    create_or_replace(secrets_manager, secret_id, &secret).await?;

    Ok(secret)
}

/// Calculate the valid until timestamp.
///
/// This function calculates the valid until timestamp based on the provided duration.
///
/// # Arguments
///
/// * `valid_for_duration` - The duration for which the tokens are valid.
///
/// # Returns
///
/// * `Option<f64>` - The valid until timestamp as a floating-point number.
fn calculate_valid_until(
    valid_for_duration: Option<std::time::Duration>,
) -> Option<f64> {
    valid_for_duration.map(|d| {
        let now = std::time::SystemTime::now();
        (now + d)
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_else(|_| std::time::Duration::from_secs(0))
            .as_secs_f64()
    })
}
