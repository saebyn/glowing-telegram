/**
 * This is the main entrypoint for the summarize_transcription lambda function.
 *
 * The function is responsible for summarizing the transcription of an audio file using the OpenAI API and saving the result to DynamoDB.
 */
use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
use figment::Figment;
use lambda_runtime::{service_fn, Error, LambdaEvent};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
struct Config {
    openai_secret_arn: String,
    metadata_table_name: String,
}

fn load_config() -> Result<Config, figment::Error> {
    let figment = Figment::new().merge(figment::providers::Env::raw());

    figment.extract()
}

#[derive(Debug, Deserialize)]
struct ItemInput {
    key: String,
    audio: String,
}

#[derive(Deserialize, Debug)]
struct Request {
    #[serde(rename = "transcriptionContext")]
    transcription_context: String,

    item: ItemInput,
}

#[derive(Serialize)]
struct Response {}

#[derive(Debug)]
struct SharedResources {
    dynamodb: aws_sdk_dynamodb::Client,
    secrets_manager: aws_sdk_secretsmanager::Client,
    config: Config,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = load_config().expect("failed to load config");
    let region_provider =
        RegionProviderChain::default_provider().or_else("us-east-1");
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    let dynamodb = aws_sdk_dynamodb::Client::new(&aws_config);
    let secrets_manager = aws_sdk_secretsmanager::Client::new(&aws_config);

    let shared_resources = &SharedResources {
        dynamodb,
        secrets_manager,
        config,
    };

    lambda_runtime::run(service_fn(
        move |event: LambdaEvent<Request>| async move {
            handler(shared_resources, event).await
        },
    ))
    .await?;
    Ok(())
}

async fn handler(
    shared_resources: &SharedResources,
    event: LambdaEvent<Request>,
) -> Result<Response, Error> {
    // payload should be the dynamodb fields (key and audio) passed from the step function
    let payload = event.payload;

    print!("Payload: {:?}", payload);

    print!("Shared Resources: {:?}", shared_resources);

    // we need to
    // 1. get the result from the transcription job from dynamodb and the context
    // 2. get the openai api key from secrets manager
    // 3. call the openai api with the transcription result and context
    // 4. save the result to dynamodb (transcription_context)

    Ok(Response {})
}
