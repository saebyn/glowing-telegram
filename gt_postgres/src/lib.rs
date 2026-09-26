use aws_sdk_secretsmanager::Client as SecretsManagerClient;
use serde::Deserialize;
use thiserror::Error;
use tokio_postgres::{Client, NoTls};

#[derive(Clone, Debug, Deserialize)]
pub struct PostgresConfig {
    pub database_secret_arn: String,
    pub database_endpoint: String,
    #[serde(default = "default_port")]
    pub database_port: u16,
    pub database_name: String,
}

#[derive(Debug, Error)]
pub enum ConnectError {
    #[error("failed to fetch database credentials: {0}")]
    Secret(#[source] Box<dyn std::error::Error + Send + Sync>),
    #[error("database secret does not contain a string")]
    MissingSecretString,
    #[error("database secret is not valid username/password JSON: {0}")]
    InvalidSecret(#[from] serde_json::Error),
    #[error("failed to connect to PostgreSQL: {0}")]
    Postgres(#[from] tokio_postgres::Error),
}

#[derive(Deserialize)]
struct DatabaseCredentials {
    username: String,
    password: String,
}

const fn default_port() -> u16 {
    5432
}

/// Connects to PostgreSQL using credentials stored as a JSON Secrets Manager
/// secret with `username` and `password` fields.
pub async fn connect(
    config: &PostgresConfig,
    aws_config: &aws_config::SdkConfig,
) -> Result<Client, ConnectError> {
    let secret = SecretsManagerClient::new(aws_config)
        .get_secret_value()
        .secret_id(&config.database_secret_arn)
        .send()
        .await
        .map_err(|error| ConnectError::Secret(Box::new(error)))?;
    let credentials: DatabaseCredentials = serde_json::from_str(
        secret
            .secret_string()
            .ok_or(ConnectError::MissingSecretString)?,
    )?;

    let mut postgres = tokio_postgres::Config::new();
    postgres
        .host(&config.database_endpoint)
        .port(config.database_port)
        .dbname(&config.database_name)
        .user(&credentials.username)
        .password(&credentials.password);

    let (client, connection) = postgres.connect(NoTls).await?;
    tokio::spawn(async move {
        if let Err(error) = connection.await {
            tracing::error!(%error, "PostgreSQL connection failed");
        }
    });

    Ok(client)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_port_defaults_to_postgres_port() {
        let config: PostgresConfig =
            serde_json::from_value(serde_json::json!({
                "database_secret_arn": "secret",
                "database_endpoint": "database.example.com",
                "database_name": "streamosaic"
            }))
            .unwrap();

        assert_eq!(config.database_port, 5432);
    }
}
