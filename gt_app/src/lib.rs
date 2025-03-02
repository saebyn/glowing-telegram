use aws_config::{BehaviorVersion, meta::region::RegionProviderChain};
use figment::{Figment, providers::Env};

pub trait ContextProvider<Config> {
    fn new(config: Config, aws_config: aws_config::SdkConfig) -> Self;
}

/// Initialize the application context with configuration from environment variables.
/// The configuration is extracted using figment.
/// The AWS configuration is loaded using the default provider chain.
///
/// # Arguments
/// None
///
/// # Returns
/// The application context with the configuration and AWS configuration as
/// specified by the trait.
///
/// # Errors
/// If the configuration cannot be extracted from the environment variables
/// or if the AWS configuration cannot be loaded.
///
pub async fn create_app_context<'a, A, Config: serde::Deserialize<'a>>()
-> Result<A, figment::Error>
where
    A: ContextProvider<Config>,
{
    let figment = Figment::new().merge(Env::raw());

    let config: Config = figment.extract()?;

    let region_provider =
        RegionProviderChain::default_provider().or_else("us-east-1");
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    Ok(ContextProvider::new(config, aws_config))
}
