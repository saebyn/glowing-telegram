use aws_sdk_secretsmanager::client::Client as SecretsManagerClient;
use figment::Figment;
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

pub fn load_config() -> Result<Config, figment::Error> {
    let figment = Figment::new().merge(figment::providers::Env::raw());

    figment.extract()
}

#[derive(Debug, Clone)]
pub struct AppState {
    pub secrets_manager: Arc<SecretsManagerClient>,
    pub youtube_credentials: YouTubeCredentials,
    pub config: Config,
}
