use aws_config::{BehaviorVersion, meta::region::RegionProviderChain};
use aws_sdk_dynamodb::types::AttributeValue;
use figment::{Figment, providers::Env};
use serde::Deserialize;
use std::env;
use types::{Silence, Transcription};

mod dynamodb;
mod whisper;

use whisper::{WhisperModel, WhisperOptions};

#[derive(Deserialize, Debug, Clone)]
struct Config {
    input_bucket: String,

    dynamodb_table: String,
}

fn load_config() -> Result<Config, figment::Error> {
    let figment = Figment::new().merge(Env::raw());

    figment.extract()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    // Read configuration from environment variables with figment
    let config = load_config().expect("failed to load config");

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() != 5 {
        eprintln!(
            "Usage: audio_transcriber <item_key> <input_key> <initial_prompt> <language>",
        );
        std::process::exit(1);
    }

    let item_key = args[1].clone();
    let input_key = args[2].clone();
    let initial_prompt = args[3].clone();
    let language = args[4].clone();

    tracing::info!("Processing audio with key: {}", input_key);

    // Load AWS configuration
    let region_provider =
        RegionProviderChain::default_provider().or_else("us-east-1");
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    // Create clients
    let client = aws_sdk_s3::Client::new(&aws_config);
    let dynamodb = aws_sdk_dynamodb::Client::new(&aws_config);

    // Get silence data from DynamoDB
    let (silence_segments, duration) = match dynamodb::get_item_from_dynamodb(
        &dynamodb,
        &config.dynamodb_table,
        &item_key,
    )
    .await
    {
        Ok(item) => {
            let segments = match dynamodb::get_silence_data_from_item(&item)
                .await
            {
                Ok(segments) => {
                    tracing::info!(
                        "Retrieved {} silence segments",
                        segments.len()
                    );
                    segments
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to extract silence data: {}. Using default transcription.",
                        e
                    );
                    Vec::new()
                }
            };

            let duration = match dynamodb::get_duration_from_item(&item).await
            {
                Ok(duration) => {
                    tracing::info!("Retrieved duration: {} seconds", duration);
                    Some(duration)
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to extract duration: {}. Using no duration limit.",
                        e
                    );
                    None
                }
            };

            (segments, duration)
        }
        Err(e) => {
            tracing::warn!(
                "Failed to retrieve item from DynamoDB: {}. Using default transcription.",
                e
            );
            (Vec::new(), None)
        }
    };

    // Convert silence to clip timestamps
    let clip_timestamps = whisper::convert_silence_to_clip_timestamps(
        &silence_segments,
        duration,
    );
    tracing::info!("Using clip_timestamps: {}", clip_timestamps);

    let options = WhisperOptions {
        model: WhisperModel::Turbo,
        model_dir: "/model/".to_string(),
        initial_prompt,
        language,
        clip_timestamps,
        verbose: false,
    };

    // capture output
    let whisper_output = whisper::run_whisper_on_s3_object(
        &client,
        &config.input_bucket,
        &input_key,
        options,
    )
    .await
    .expect("failed to run whisper");

    // write output to dynamodb
    dynamodb
        .update_item()
        .table_name(config.dynamodb_table.clone())
        .key("key", AttributeValue::S(item_key.clone()))
        .update_expression("SET transcription = :t")
        .expression_attribute_values(
            ":t",
            dynamodb::convert_transcription_to_attributevalue(whisper_output),
        )
        .send()
        .await
        .expect("failed to write to dynamodb");
}
