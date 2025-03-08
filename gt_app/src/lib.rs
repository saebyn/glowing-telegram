use aws_config::{BehaviorVersion, meta::region::RegionProviderChain};
use figment::{Figment, providers::Env};

pub trait ContextProvider<Config> {
    fn new(
        config: Config,
        aws_config: aws_config::SdkConfig,
    ) -> impl Future<Output = Self>;
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
    // https://docs.aws.amazon.com/lambda/latest/dg/rust-logging.html
    // this will also be used in the batch jobs, but that shoud be ok.
    tracing_subscriber::fmt()
        .json()
        // allow log level to be overridden by RUST_LOG env var
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        // this needs to be set to remove duplicated information in the log.
        .with_current_span(false)
        // this needs to be set to false, otherwise ANSI color codes will
        // show up in a confusing manner in CloudWatch logs.
        .with_ansi(false)
        // disabling time is handy because CloudWatch will add the ingestion time.
        .without_time()
        // remove the name of the function from every log entry
        .with_target(false)
        .init();

    let figment = Figment::new().merge(Env::raw());

    let config: Config = figment.extract()?;

    let region_provider =
        RegionProviderChain::default_provider().or_else("us-east-1");
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    let context = A::new(config, aws_config).await;

    Ok(context)
}
