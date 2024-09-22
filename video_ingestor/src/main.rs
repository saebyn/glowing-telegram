use aws_config::{
    meta::region::RegionProviderChain, BehaviorVersion, SdkConfig,
};
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_s3::{
    operation::get_object::GetObjectOutput, primitives::ByteStream,
};
use figment::{providers::Env, Figment};
use gt_ffmpeg::{
    audio_extraction,
    ffprobe::{self, FFProbeOutput},
    keyframes_extraction,
};
use serde::Deserialize;
use std::{collections::HashMap, env};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(Deserialize, Debug, Clone)]
struct Config {
    input_bucket: String,

    output_bucket: String,
    keyframes_prefix: String,
    audio_prefix: String,

    dynamodb_table: String,
}

fn load_config() -> Result<Config, figment::Error> {
    let figment = Figment::new().merge(Env::raw());

    figment.extract()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let region_provider =
        RegionProviderChain::default_provider().or_else("us-east-1");
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    // Read configuration from environment variables with figment
    let config = load_config().expect("failed to load config");

    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    let input_key = args[2].clone();

    tracing::info!("Processing video with key: {}", input_key);

    let temp_file_path =
        download_video(&aws_config, &config.input_bucket, &input_key).await;

    // In parallel, do audio extraction to a temp file, extract keyframes, use ffprobe to get metadata
    // Await the tasks to ensure they complete
    let (audio_result, keyframes_result, metadata_result) = tokio::join!(
        do_audio_extraction_task(
            &aws_config,
            config.audio_prefix.clone(),
            temp_file_path.clone(),
            config.output_bucket.clone(),
            input_key.clone()
        ),
        do_keyframes_extraction_task(
            &aws_config,
            config.keyframes_prefix.clone(),
            temp_file_path.clone(),
            config.output_bucket.clone(),
            input_key.clone()
        ),
        do_metadata_task(temp_file_path.clone())
    );

    let audio_result = audio_result.expect("failed to extract audio");
    let keyframes_result =
        keyframes_result.expect("failed to extract keyframes");
    let metadata_result = metadata_result.expect("failed to get metadata");

    // Insert the metadata into the DynamoDB table
    save_results_to_dynamodb(
        &aws_config,
        &config.dynamodb_table,
        input_key,
        metadata_result,
        audio_result,
        keyframes_result,
    )
    .await
    .expect("failed to insert metadata into DynamoDB");
}

fn format_metadata(metadata: &FFProbeOutput) -> AttributeValue {
    let json_metadata: serde_json::Value = serde_json::json!(metadata);

    let mut formatted_metadata = HashMap::new();

    for (key, value) in json_metadata.as_object().unwrap() {
        let attribute_value = match value {
            serde_json::Value::String(s) => AttributeValue::S(s.clone()),
            serde_json::Value::Number(n) => AttributeValue::N(n.to_string()),
            serde_json::Value::Bool(b) => AttributeValue::Bool(*b),
            _ => AttributeValue::Null(true),
        };

        formatted_metadata.insert(key.clone(), attribute_value);
    }

    AttributeValue::M(formatted_metadata)
}

async fn save_s3_object_to_file(
    mut object: GetObjectOutput,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .await
        .expect("failed to open temp file");

    while let Some(bytes) = object.body.try_next().await? {
        file.write_all(&bytes)
            .await
            .expect("failed to write to temp file");
    }

    Ok(())
}

async fn save_stdio_to_file(
    mut stdio: tokio::process::ChildStdout,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .await
        .expect("failed to open temp file");

    let mut buffer = [0; 1024];
    loop {
        let n = stdio.read(&mut buffer).await?;
        if n == 0 {
            break;
        }
        file.write_all(&buffer[..n]).await?;
    }

    Ok(())
}

async fn download_video(
    aws_config: &SdkConfig,
    input_bucket: &str,
    input_key: &str,
) -> String {
    let s3_client = aws_sdk_s3::Client::new(aws_config);

    let temp_file_path = std::env::temp_dir()
        .join("videofile")
        .to_str()
        .unwrap()
        .to_string();

    {
        // Get the object from the bucket
        let object = s3_client
            .get_object()
            .bucket(input_bucket)
            .key(input_key)
            .send()
            .await
            .expect("failed to get object");

        // Write the object to a temp file

        save_s3_object_to_file(object, &temp_file_path)
            .await
            .expect("failed to save object to file");
    }

    temp_file_path
}

async fn save_results_to_dynamodb(
    aws_config: &SdkConfig,
    table_name: &str,
    input_key: String,
    metadata_result: FFProbeOutput,
    audio_result: String,
    keyframes_result: Vec<String>,
) -> Result<(), aws_sdk_dynamodb::Error> {
    let dynamodb_client = aws_sdk_dynamodb::Client::new(aws_config);

    dynamodb_client
        .put_item()
        .table_name(table_name)
        .item("key", AttributeValue::S(input_key.to_string()))
        .item("metadata", format_metadata(&metadata_result))
        .item("audio", AttributeValue::S(audio_result.to_string()))
        .item(
            "keyframes",
            AttributeValue::Ns(
                keyframes_result
                    .into_iter()
                    .map(|s| s.parse().unwrap())
                    .collect(),
            ),
        )
        .send()
        .await?;

    Ok(())
}

fn do_audio_extraction_task(
    aws_config: &SdkConfig,
    audio_prefix: String,
    temp_file_path: String,
    output_bucket: String,
    input_key: String,
) -> tokio::task::JoinHandle<String> {
    let s3_client = aws_sdk_s3::Client::new(aws_config);

    tokio::spawn(async move {
        // Extract audio from the video file
        let audio_temp_file_path =
            std::env::temp_dir().join("audiofile").to_str().unwrap()[..]
                .to_string();
        let audio = audio_extraction::extract(&temp_file_path, 1)
            .expect("failed to extract audio");

        save_stdio_to_file(audio, &audio_temp_file_path)
            .await
            .expect("failed to save audio to file");

        let output_key = format!("{audio_prefix}/{input_key}");

        // Upload the audio to an S3 bucket
        s3_client
            .put_object()
            .bucket(output_bucket)
            .key(output_key.as_str())
            .body(
                ByteStream::from_path(audio_temp_file_path.clone())
                    .await
                    .unwrap(),
            )
            .send()
            .await
            .expect("failed to upload audio");

        output_key.to_string()
    })
}

fn do_metadata_task(
    temp_file_path: String,
) -> tokio::task::JoinHandle<FFProbeOutput> {
    tokio::spawn(async move {
        // Use ffprobe to get metadata about the video file
        ffprobe::probe(&temp_file_path)
            .await
            .expect("failed to get metadata")
    })
}

fn do_keyframes_extraction_task(
    aws_config: &SdkConfig,
    keyframes_prefix: String,
    temp_file_path: String,
    output_bucket: String,
    input_key: String,
) -> tokio::task::JoinHandle<Vec<String>> {
    let s3_client = aws_sdk_s3::Client::new(aws_config);
    tokio::spawn(async move {
        // Extract keyframes from the video file
        let keyframe_fns =
            keyframes_extraction::extract(&temp_file_path, 200, -1)
                .await
                .expect("failed to extract keyframes");

        let mut keyframe_keys = Vec::new();

        // Upload the keyframes to an S3 bucket
        for keyframe_fn in keyframe_fns {
            let keyframe_path = std::path::Path::new(&keyframe_fn);
            let keyframe_basename = keyframe_path
                .file_name()
                .expect("failed to get keyframe filename")
                .to_str()
                .expect("failed to convert keyframe filename to string")
                .to_string();

            let keyframe_key =
                format!("{keyframes_prefix}/{input_key}/{keyframe_basename}");

            s3_client
                .put_object()
                .bucket(&output_bucket)
                .key(&keyframe_key)
                .body(ByteStream::from_path(keyframe_fn).await.unwrap())
                .send()
                .await
                .expect("failed to upload keyframe");

            keyframe_keys.push(keyframe_key.clone());
        }

        // Return the S3 keys of the keyframes
        keyframe_keys
    })
}
