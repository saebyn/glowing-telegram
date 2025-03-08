use aws_sdk_secretsmanager::client::Client as SecretsManagerClient;
use gt_secrets::UserSecretPathProvider;
use serde::Deserialize;
use std::sync::Arc;
use types::utils::YouTubeCredentials;

#[derive(Debug, Deserialize, Clone)]
#[allow(clippy::struct_field_names)]
pub struct Config {
    pub youtube_secret_arn: String,

    pub user_secret_path: UserSecretPathProvider,
}

#[derive(Debug, Clone)]
pub struct AppContext {
    pub secrets_manager: Arc<SecretsManagerClient>,
    pub youtube_credentials: YouTubeCredentials,
    pub config: Config,
}

impl gt_app::ContextProvider<Config> for AppContext {
    async fn new(config: Config, aws_config: aws_config::SdkConfig) -> Self {
        let secrets_manager = SecretsManagerClient::new(&aws_config);

        let youtube_credentials = match secrets_manager
            .get_secret_value()
            .secret_id(&config.youtube_secret_arn)
            .send()
            .await
        {
            Ok(secret) => match serde_json::from_str::<YouTubeCredentials>(
                secret.secret_string.as_deref().unwrap_or("{}"),
            ) {
                Ok(credentials) => credentials,
                Err(e) => {
                    tracing::error!("failed to parse YouTube secret: {:?}", e);
                    panic!("failed to parse YouTube secret");
                }
            },
            Err(e) => {
                tracing::error!("failed to get YouTube secret: {:?}", e);
                panic!("failed to get YouTube secret");
            }
        };

        // Create a shared state to pass to the handler
        Self {
            secrets_manager: Arc::new(secrets_manager),
            youtube_credentials,
            config,
        }
    }
}
