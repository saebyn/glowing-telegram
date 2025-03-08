use aws_sdk_secretsmanager::client::Client as SecretsManagerClient;
use gt_secrets::UserSecretPathProvider;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize, Clone)]
#[allow(clippy::struct_field_names)]
pub struct Config {
    pub twitch_secret_arn: String,

    pub is_global_refresh_service: bool,

    pub user_secret_path: UserSecretPathProvider,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TwitchCredentials {
    pub id: String,
    pub secret: redact::Secret<String>,
    pub redirect_url: String,
}

#[derive(Debug, Clone)]
pub struct AppContext {
    pub secrets_manager: Arc<SecretsManagerClient>,
    pub twitch_credentials: TwitchCredentials,
    pub config: Config,
}

impl gt_app::ContextProvider<Config> for AppContext {
    async fn new(config: Config, aws_config: aws_config::SdkConfig) -> Self {
        let secrets_manager = SecretsManagerClient::new(&aws_config);

        let twitch_credentials = match secrets_manager
            .get_secret_value()
            .secret_id(&config.twitch_secret_arn)
            .send()
            .await
        {
            Ok(secret) => match serde_json::from_str::<TwitchCredentials>(
                secret.secret_string.as_deref().unwrap_or("{}"),
            ) {
                Ok(credentials) => credentials,
                Err(e) => {
                    tracing::error!("failed to parse Twitch secret: {:?}", e);
                    panic!("failed to parse Twitch secret");
                }
            },
            Err(e) => {
                tracing::error!("failed to get Twitch secret: {:?}", e);
                panic!("failed to get Twitch secret");
            }
        };

        // Create a shared state to pass to the handler
        Self {
            secrets_manager: Arc::new(secrets_manager),
            twitch_credentials,
            config,
        }
    }
}
