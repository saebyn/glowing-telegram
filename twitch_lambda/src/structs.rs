use aws_sdk_secretsmanager::client::Client as SecretsManagerClient;
use aws_sdk_sqs::Client as SqsClient;
use gt_secrets::UserSecretPathProvider;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize, Clone)]
#[allow(clippy::struct_field_names)]
pub struct Config {
    pub twitch_secret_arn: String,

    pub is_global_refresh_service: bool,

    pub user_secret_path: UserSecretPathProvider,

    pub chat_queue_url: Option<String>,

    pub eventsub_secret_arn: Option<String>,
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
    pub sqs_client: Arc<SqsClient>,
    pub twitch_credentials: TwitchCredentials,
    pub config: Config,
    pub eventsub_secret: Option<String>,
}

impl gt_app::ContextProvider<Config> for AppContext {
    async fn new(config: Config, aws_config: aws_config::SdkConfig) -> Self {
        let secrets_manager = SecretsManagerClient::new(&aws_config);
        let sqs_client = SqsClient::new(&aws_config);

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

        // Load EventSub secret if configured
        let eventsub_secret = if let Some(eventsub_secret_arn) =
            &config.eventsub_secret_arn
        {
            match secrets_manager
                .get_secret_value()
                .secret_id(eventsub_secret_arn)
                .send()
                .await
            {
                Ok(secret) => {
                    if let Some(secret_string) = secret.secret_string {
                        // Try to parse as JSON first, then fall back to direct string
                        if let Ok(parsed) =
                            serde_json::from_str::<serde_json::Value>(
                                &secret_string,
                            )
                        {
                            if let Some(secret_value) =
                                parsed.get("secret").and_then(|v| v.as_str())
                            {
                                Some(secret_value.to_string())
                            } else {
                                Some(secret_string)
                            }
                        } else {
                            Some(secret_string)
                        }
                    } else {
                        tracing::warn!("EventSub secret value is empty");
                        None
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to get EventSub secret: {:?}", e);
                    None
                }
            }
        } else {
            tracing::warn!("EventSub secret ARN not configured");
            None
        };

        // Create a shared state to pass to the handler
        Self {
            secrets_manager: Arc::new(secrets_manager),
            sqs_client: Arc::new(sqs_client),
            twitch_credentials,
            config,
            eventsub_secret,
        }
    }
}
