use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde_json::{json, Value};

#[tokio::main]
async fn main() -> Result<(), Error> {
    let func = service_fn(func);
    lambda_runtime::run(func).await?;
    Ok(())
}

async fn func(event: LambdaEvent<Value>) -> Result<(), Error> {
    let region_provider =
        RegionProviderChain::default_provider().or_else("us-east-1");
    let config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    // payload should be the dynamodb fields (key and audio) passed from the step function

    // we need to
    // 1. get the result from the transcription job from dynamodb and the context
    // 2. get the openai api key from secrets manager
    // 3. call the openai api with the transcription result and context
    // 4. save the result to dynamodb

    println!("Received event: {:?}", event);

    Ok(())
}
