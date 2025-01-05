//! Figment [`figment::Provider`] for AWS Secrets Manager.
//!
//! ```rust
//! use serde::Deserialize;
//! use figment::{Figment, providers::{Env}};
//! use figment_aws_provider::SecretsManager;
//!
//! #[derive(Deserialize)]
//! struct Config {
//!   frobnicate: String,
//!   foo: u64,
//! }
//!
//! let secrets_manager_client = aws_sdk_secrets_manager::Client::new(aws_sdk_secrets_manager::Config::default());
//!
//! let config: Config = Figment::new()
//!    .merge(SecretsManagerAdapter::wrap(
//!       Env::new(),
//!      secrets_manager_client
//!    ).suffix("SECRET_ARN"))
//!    .extract()?;
//! Ok(())
//! ```
//!
//! # Overview
//!
//! This crate provides a [`figment::Provider`] implementation for AWS Secrets
//! Manager. It allows you to load configuration values from a secret in AWS
//! Secrets Manager into your application using the [`figment`] configuration
//! ibrary.

use figment::{
    error::{Kind, Result},
    value::{Dict, Map},
    Profile, Provider,
};

use tokio::runtime::Runtime;

use aws_sdk_secretsmanager::Client;

use std::collections::BTreeMap;

pub struct SecretsManagerAdapter {
    provider: Box<dyn Provider>,
    suffix: String,
    client: Client,
}

impl SecretsManagerAdapter {
    pub fn wrap<T: Provider + 'static>(provider: T, client: Client) -> Self {
        Self {
            provider: Box::new(provider),
            suffix: "SECRET_ARN".to_string(),
            client,
        }
    }

    /// Set the suffix for the environment variable that contains the secret ARN.
    #[must_use]
    pub fn suffix(self, suffix: &str) -> Self {
        Self {
            suffix: suffix.to_string(),
            ..self
        }
    }

    /// Set the AWS Secrets Manager client.
    #[must_use]
    pub fn client(self, client: Client) -> Self {
        Self { client, ..self }
    }
}

impl Provider for SecretsManagerAdapter {
    fn metadata(&self) -> figment::Metadata {
        self.provider.metadata()
    }

    fn data(&self) -> Result<Map<Profile, Dict>> {
        let data = self.provider.data()?;
        let mut secrets: Map<Profile, Dict> = BTreeMap::new();

        let rt = Runtime::new().map_err(|e| {
            Kind::Message(format!("Error creating Tokio runtime: {e:?}"))
        })?;

        for (profile, dict) in data {
            let mut secret = BTreeMap::new();
            for (key, value) in &dict {
                if !key.ends_with(&self.suffix) {
                    continue;
                }

                let key = key.trim_end_matches(&self.suffix).to_string();

                let secret_arn = value.as_str().ok_or_else(|| {
                    Kind::InvalidValue(
                        figment::error::Actual::Str(
                            value
                                .clone()
                                .into_string()
                                .unwrap_or_else(|| "<non-string>".to_string()),
                        ),
                        "Secret ARN must be a string".to_string(),
                    )
                })?;

                let secret_value_future = self
                    .client
                    .get_secret_value()
                    .secret_id(secret_arn)
                    .send();

                let secret_value = rt
                    .block_on(secret_value_future)
                    .map_err(|e| {
                        Kind::Message(format!("Error fetching secret: {e:?}",))
                    })?
                    .secret_string;

                if let Some(secret_value) = secret_value {
                    secret.insert(key.clone(), secret_value.into());
                } else {
                    return Err(Kind::Message(format!(
                        "Secret not found for ARN: {secret_arn}"
                    ))
                    .into());
                }
            }
            secrets.insert(profile.clone(), secret);
        }

        Ok(secrets)
    }
}
