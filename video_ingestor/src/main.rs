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
    silence_detection::Segment,
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

    // The track number of the audio to extract
    speech_track_number: u32,
    // Input audio volume is less or equal to a noise tolerance value
    noise_tolerance: f64,
    // Minimum detected noise duration
    silence_duration: f64,
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

    let input_video_file_path = download_s3_object_to_tempfile(
        &aws_config,
        &config.input_bucket,
        &input_key,
    )
    .await;

    // In parallel, do audio extraction to a temp file, extract keyframes, use ffprobe to get metadata
    // Await the tasks to ensure they complete
    let (audio_result, keyframes_result, metadata_result, silence_result) = tokio::join!(
        do_audio_extraction_task(
            &aws_config,
            config.speech_track_number,
            config.audio_prefix.clone(),
            input_video_file_path.clone(),
            config.output_bucket.clone(),
            input_key.clone()
        ),
        do_keyframes_extraction_task(
            &aws_config,
            config.keyframes_prefix.clone(),
            input_video_file_path.clone(),
            config.output_bucket.clone(),
            input_key.clone()
        ),
        do_metadata_task(input_video_file_path.clone()),
        do_silence_detection_task(
            input_video_file_path.clone(),
            config.speech_track_number,
            config.noise_tolerance,
            config.silence_duration
        )
    );

    let audio_result = audio_result.expect("failed to extract audio");
    let keyframes_result =
        keyframes_result.expect("failed to extract keyframes");
    let metadata_result = metadata_result.expect("failed to get metadata");
    let silence_result = silence_result.expect("failed to extract silence");

    // Insert the metadata into the DynamoDB table
    save_results_to_dynamodb(
        &aws_config,
        &config.dynamodb_table,
        input_key,
        metadata_result,
        audio_result,
        keyframes_result,
        silence_result,
    )
    .await
    .expect("failed to insert metadata into DynamoDB");
}

fn format_object(value: &serde_json::Value) -> AttributeValue {
    match value {
        serde_json::Value::String(s) => AttributeValue::S(s.clone()),
        serde_json::Value::Number(n) => AttributeValue::N(n.to_string()),
        serde_json::Value::Bool(b) => AttributeValue::Bool(*b),
        serde_json::Value::Object(o) => {
            let mut formatted_object = HashMap::new();
            for (k, v) in o {
                formatted_object.insert(k.clone(), format_object(v));
            }
            AttributeValue::M(formatted_object)
        }
        serde_json::Value::Array(a) => AttributeValue::L(
            a.iter().map(format_object).collect::<Vec<AttributeValue>>(),
        ),
        serde_json::Value::Null => AttributeValue::Null(true),
    }
}

fn format_metadata(metadata: &FFProbeOutput) -> AttributeValue {
    let json_metadata: serde_json::Value = serde_json::json!(metadata);
    format_object(&json_metadata)
}

#[tracing::instrument]
async fn save_s3_object_to_file(
    mut object: GetObjectOutput,
    path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Saving object to file: {}", path);

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

    file.flush().await.expect("failed to flush temp file");

    tracing::info!("Saved object to file: {}", path);

    Ok(())
}

#[tracing::instrument]
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

#[tracing::instrument]
async fn download_s3_object_to_tempfile(
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

#[tracing::instrument]
async fn save_results_to_dynamodb(
    aws_config: &SdkConfig,
    table_name: &str,
    input_key: String,
    metadata_result: FFProbeOutput,
    audio_result: String,
    keyframes_result: Vec<String>,
    silence_result: Vec<Segment>,
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
            AttributeValue::Ss(
                keyframes_result
                    .into_iter()
                    .map(|s| s.parse().unwrap())
                    .collect(),
            ),
        )
        .item(
            "silence",
            AttributeValue::L(
                silence_result
                    .into_iter()
                    .map(|segment| {
                        AttributeValue::M(
                            vec![
                                (
                                    "start".to_string(),
                                    AttributeValue::N(
                                        segment.start.as_secs().to_string(),
                                    ),
                                ),
                                (
                                    "end".to_string(),
                                    AttributeValue::N(
                                        segment.end.as_secs().to_string(),
                                    ),
                                ),
                            ]
                            .into_iter()
                            .collect(),
                        )
                    })
                    .collect(),
            ),
        )
        .send()
        .await?;

    Ok(())
}

fn do_audio_extraction_task(
    aws_config: &SdkConfig,
    track_number: u32,
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
        let audio = audio_extraction::extract(&temp_file_path, track_number)
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
        // Create a temporary directory to store the keyframes.
        let temp_dir = tempfile::tempdir().expect("failed to create temp dir");
        // Extract keyframes from the video file
        let keyframe_fns =
            keyframes_extraction::extract(&temp_dir, &temp_file_path, 200, -1)
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

            tracing::info!(
                "Uploading keyframe: {} to {}",
                keyframe_fn,
                keyframe_key
            );

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

fn do_silence_detection_task(
    temp_file_path: String,
    track_number: u32,
    noise: f64,
    duration: f64,
) -> tokio::task::JoinHandle<Vec<Segment>> {
    tokio::spawn(async move {
        // Detect silence in the audio file
        let segments = gt_ffmpeg::silence_detection::extract(
            &temp_file_path,
            track_number,
            noise,
            duration,
        )
        .await
        .expect("failed to extract silence");

        for segment in &segments {
            tracing::trace!(
                "Silence detected from {:?} to {:?}",
                segment.start,
                segment.end
            );
        }

        segments
    })
}
