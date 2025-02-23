use aws_sdk_secretsmanager::client::Client as SecretsManagerClient;
use figment::Figment;
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

pub fn load_config() -> Result<Config, figment::Error> {
    let figment = Figment::new().merge(figment::providers::Env::raw());

    figment.extract()
}

#[derive(Debug, Clone, Deserialize)]
pub struct TwitchCredentials {
    pub id: String,
    pub secret: redact::Secret<String>,
    pub redirect_url: String,
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub secrets_manager: Arc<SecretsManagerClient>,
    pub twitch_credentials: TwitchCredentials,
    pub config: Config,
}
