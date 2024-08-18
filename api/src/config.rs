use serde::Deserialize;

use figment::{providers::Env, Figment};
use figment_file_provider_adapter::FileAdapter;
use redact::Secret;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    // `OPENAI_KEY_PATH` is the name of the environment variable
    pub openai_key: Secret<String>,

    pub openai_model: String,

    pub video_storage_path: String,
    pub rendered_episode_storage_path: String,
    pub noise: f64,
    pub duration: f64,

    pub task_api_url: String,
    pub task_api_external_url: String,

    pub this_api_base_url: String,

    pub twitch_client_id: String,

    // `TWITCH_CLIENT_SECRET_PATH` is the name of the environment variable
    pub twitch_client_secret: Secret<String>,

    pub twitch_user_id: String,
    pub twitch_redirect_url: String,

    pub youtube_auth_uri: String,
    pub youtube_token_uri: String,
    pub youtube_client_id: String,

    // `YOUTUBE_CLIENT_SECRET_PATH` is the name of the environment variable
    pub youtube_client_secret: Secret<String>,
    pub youtube_redirect_url: String,

    pub redis_url: String,

    pub http_client_agent: String,
}

pub fn load_config() -> Result<Config, figment::Error> {
    let figment = Figment::new()
        .merge(FileAdapter::wrap(Env::raw()))
        .join(("openai_model", "gpt-4o"))
        .join(("http_client_agent", "glowing-telegram-api/0.1"));
    figment.extract()
}
