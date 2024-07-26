use serde::Deserialize;

use figment::{
    providers::{Env, Format, Toml},
    Figment,
};
use figment_file_provider_adapter::FileAdapter;
use redact::Secret;

#[derive(Deserialize)]
pub struct Config {
    pub openai_key: Secret<String>,
}

pub fn load_config() -> Result<Config, figment::Error> {
    let figment =
        Figment::new().merge(FileAdapter::wrap(Env::prefixed("APP_")));
    figment.extract()
}
