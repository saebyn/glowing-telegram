use aws_config::{
    BehaviorVersion, SdkConfig, meta::region::RegionProviderChain,
};
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_s3::{
    operation::get_object::GetObjectOutput, primitives::ByteStream,
};
use figment::{Figment, providers::Env};
use gt_ffmpeg::{
    audio_extraction,
    ffprobe::{self, FFProbeOutput},
    keyframes_extraction,
    silence_detection::Segment,
    transcode::HLSEntry,
};
use serde::Deserialize;
use std::{collections::HashMap, env};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

const STEP_AUDIO_UPLOAD: &str = "audio_upload";
const STEP_KEYFRAMES: &str = "keyframes";
const STEP_METADATA: &str = "metadata";
const STEP_SILENCE: &str = "silence";
const STEP_TRANSCODE_HLS: &str = "transcode_hls";
const STEP_PEAKS: &str = "peaks";

const STEP_VERSION_AUDIO_UPLOAD: &str = "v1.0.0";
const STEP_VERSION_KEYFRAMES: &str = "v1.0.0";
const STEP_VERSION_METADATA: &str = "v1.0.0";
const STEP_VERSION_SILENCE: &str = "v1.0.0";
const STEP_VERSION_TRANSCODE_HLS: &str = "v1.0.0";
const STEP_VERSION_PEAKS: &str = "v1.0.0";
const LEGACY_GLOBAL_INGESTION_VERSION: &str = "v1.1.0";

#[derive(Deserialize, Debug, Clone)]
struct Config {
    input_bucket: String,

    output_bucket: String,
    keyframes_prefix: String,
    audio_prefix: String,
    peaks_prefix: String,
    transcode_prefix: String,

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

    let input_key = args[1].clone();

    tracing::info!("Processing video with key: {}", input_key);

    let current_versions = current_step_versions();
    let stored_versions = get_stored_ingestion_versions(
        &aws_config,
        &config.dynamodb_table,
        &input_key,
    )
    .await;
    let plan = build_ingestion_version_plan(&stored_versions, &current_versions);

    tracing::info!(
        "Ingestion plan for {}: audio_upload={}, keyframes={}, metadata={}, silence={}, transcode_hls={}, peaks={}",
        input_key,
        plan.should_run_audio_upload,
        plan.should_run_keyframes,
        plan.should_run_metadata,
        plan.should_run_silence,
        plan.should_run_transcode_hls,
        plan.should_run_peaks,
    );

    let should_run_any_step = plan.should_run_audio_upload
        || plan.should_run_keyframes
        || plan.should_run_metadata
        || plan.should_run_silence
        || plan.should_run_transcode_hls
        || plan.should_run_peaks;

    if !should_run_any_step {
        tracing::info!(
            "All ingestion steps are already current for key: {}",
            input_key
        );
        return;
    }

    let input_video_file_path = download_s3_object_to_tempfile(
        &aws_config,
        &config.input_bucket,
        &input_key,
    )
    .await;

    let needs_audio_file =
        plan.should_run_audio_upload || plan.should_run_peaks;
    let audio_temp_file_path = if needs_audio_file {
        // Extract audio to disk first so that the peaks task can read the same
        // temp file concurrently with the S3 upload.
        let audio_temp_file_path = {
            use std::time::{SystemTime, UNIX_EPOCH};

            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time before UNIX_EPOCH")
                .as_millis();

            std::env::temp_dir()
                .join(format!("audiofile-{}-{}.wav", std::process::id(), ts))
                .to_string_lossy()
                .to_string()
        };
        let audio_stdout = audio_extraction::extract(
            &input_video_file_path,
            config.speech_track_number,
        )
        .expect("failed to extract audio");
        save_stdio_to_file(audio_stdout, &audio_temp_file_path)
            .await
            .expect("failed to save audio to file");
        Some(audio_temp_file_path)
    } else {
        None
    };

    let audio_handle = if plan.should_run_audio_upload {
        Some(upload_audio_to_s3(
            &aws_config,
            config.audio_prefix.clone(),
            audio_temp_file_path
                .clone()
                .expect("audio temp file path missing"),
            config.output_bucket.clone(),
            input_key.clone(),
        ))
    } else {
        None
    };

    let keyframes_handle = if plan.should_run_keyframes {
        Some(do_keyframes_extraction_task(
            &aws_config,
            config.keyframes_prefix.clone(),
            input_video_file_path.clone(),
            config.output_bucket.clone(),
            input_key.clone(),
        ))
    } else {
        None
    };

    let metadata_handle = if plan.should_run_metadata {
        Some(do_metadata_task(input_video_file_path.clone()))
    } else {
        None
    };

    let silence_handle = if plan.should_run_silence {
        Some(do_silence_detection_task(
            input_video_file_path.clone(),
            config.speech_track_number,
            config.noise_tolerance,
            config.silence_duration,
        ))
    } else {
        None
    };

    let transcode_handle = if plan.should_run_transcode_hls {
        Some(do_transcode_task(
            &aws_config,
            config.transcode_prefix.clone(),
            input_video_file_path.clone(),
            config.output_bucket.clone(),
            input_key.clone(),
        ))
    } else {
        None
    };

    let peaks_handle = if plan.should_run_peaks {
        Some(do_audio_peaks_task(
            &aws_config,
            config.peaks_prefix.clone(),
            audio_temp_file_path
                .clone()
                .expect("audio temp file path missing"),
            config.output_bucket.clone(),
            input_key.clone(),
        ))
    } else {
        None
    };

    let audio_result = match audio_handle {
        Some(handle) => Some(handle.await.expect("failed to upload audio")),
        None => None,
    };
    let keyframes_result = match keyframes_handle {
        Some(handle) => Some(handle.await.expect("failed to extract keyframes")),
        None => None,
    };
    let metadata_result = match metadata_handle {
        Some(handle) => Some(handle.await.expect("failed to get metadata")),
        None => None,
    };
    let silence_result = match silence_handle {
        Some(handle) => Some(handle.await.expect("failed to extract silence")),
        None => None,
    };
    let transcode_result = match transcode_handle {
        Some(handle) => Some(handle.await.expect("failed to transcode")),
        None => None,
    };
    let peaks_result = match peaks_handle {
        Some(handle) => Some(handle.await.expect("failed to compute audio peaks")),
        None => None,
    };

    let mut updated_versions = HashMap::new();
    if plan.should_run_audio_upload {
        updated_versions.insert(
            STEP_AUDIO_UPLOAD.to_string(),
            STEP_VERSION_AUDIO_UPLOAD.to_string(),
        );
    }
    if plan.should_run_keyframes {
        updated_versions.insert(
            STEP_KEYFRAMES.to_string(),
            STEP_VERSION_KEYFRAMES.to_string(),
        );
    }
    if plan.should_run_metadata {
        updated_versions.insert(
            STEP_METADATA.to_string(),
            STEP_VERSION_METADATA.to_string(),
        );
    }
    if plan.should_run_silence {
        updated_versions.insert(
            STEP_SILENCE.to_string(),
            STEP_VERSION_SILENCE.to_string(),
        );
    }
    if plan.should_run_transcode_hls {
        updated_versions.insert(
            STEP_TRANSCODE_HLS.to_string(),
            STEP_VERSION_TRANSCODE_HLS.to_string(),
        );
    }
    if plan.should_run_peaks {
        updated_versions.insert(
            STEP_PEAKS.to_string(),
            STEP_VERSION_PEAKS.to_string(),
        );
    }

    let results = IngestionResults {
        input_key,
        metadata: metadata_result,
        audio: audio_result,
        keyframes: keyframes_result,
        silence: silence_result,
        transcode: transcode_result,
        peaks: peaks_result,
        updated_versions,
    };

    // Insert the metadata into the DynamoDB table
    save_results_to_dynamodb(&aws_config, &config.dynamodb_table, results)
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
    results: IngestionResults,
) -> Result<(), aws_sdk_dynamodb::Error> {
    let dynamodb_client = aws_sdk_dynamodb::Client::new(aws_config);

    let mut update_expressions: Vec<String> = Vec::new();
    let mut expression_attribute_values: Vec<(String, AttributeValue)> = Vec::new();
    let mut expression_attribute_names: Vec<(String, String)> = Vec::new();

    if let Some(metadata) = results.metadata {
        update_expressions.push("metadata = :metadata".to_string());
        expression_attribute_values
            .push((":metadata".to_string(), format_metadata(&metadata)));
    }

    if let Some(audio) = results.audio {
        update_expressions.push("audio = :audio".to_string());
        expression_attribute_values
            .push((":audio".to_string(), AttributeValue::S(audio)));
    }

    if let Some(keyframes) = results.keyframes {
        update_expressions.push("keyframes = :keyframes".to_string());
        expression_attribute_values.push((
            ":keyframes".to_string(),
            AttributeValue::Ss(keyframes),
        ));
    }

    if let Some(silence) = results.silence {
        update_expressions.push("silence = :silence".to_string());
        expression_attribute_values.push((
            ":silence".to_string(),
            AttributeValue::L(
                silence
                    .into_iter()
                    .map(|segment| {
                        AttributeValue::M(
                            vec![
                                (
                                    "start".to_string(),
                                    AttributeValue::N(segment.start.as_secs().to_string()),
                                ),
                                (
                                    "end".to_string(),
                                    AttributeValue::N(segment.end.as_secs().to_string()),
                                ),
                            ]
                            .into_iter()
                            .collect(),
                        )
                    })
                    .collect(),
            ),
        ));
    }

    if let Some(transcode) = results.transcode {
        update_expressions.push("transcode = :transcode".to_string());
        expression_attribute_values.push((
            ":transcode".to_string(),
            AttributeValue::L(
                transcode
                    .into_iter()
                    .map(|entry| {
                        AttributeValue::M(
                            vec![
                                ("path".to_string(), AttributeValue::S(entry.path)),
                                (
                                    "duration".to_string(),
                                    AttributeValue::N(entry.duration.to_string()),
                                ),
                            ]
                            .into_iter()
                            .collect(),
                        )
                    })
                    .collect(),
            ),
        ));
    }

    if let Some(peaks) = results.peaks {
        update_expressions.push("peaks = :peaks".to_string());
        expression_attribute_values
            .push((":peaks".to_string(), AttributeValue::S(peaks)));
    }

    for (step, version) in results.updated_versions {
        let name_key = format!("#step_{}", step);
        let value_key = format!(":version_{}", step);
        update_expressions.push(format!("ingestion_versions.{} = {}", name_key, value_key));
        expression_attribute_names.push((name_key, step));
        expression_attribute_values
            .push((value_key, AttributeValue::S(version)));
    }

    if update_expressions.is_empty() {
        return Ok(());
    }

    let mut request = dynamodb_client
        .update_item()
        .table_name(table_name)
        .key("key", AttributeValue::S(results.input_key.clone()))
        .update_expression(format!("SET {}", update_expressions.join(", ")));

    for (k, v) in expression_attribute_values {
        request = request.expression_attribute_values(k, v);
    }

    for (k, v) in expression_attribute_names {
        request = request.expression_attribute_names(k, v);
    }

    request.send().await?;

    Ok(())
}

#[derive(Debug)]
struct IngestionResults {
    input_key: String,
    metadata: Option<FFProbeOutput>,
    audio: Option<String>,
    keyframes: Option<Vec<String>>,
    silence: Option<Vec<Segment>>,
    transcode: Option<Vec<HLSEntry>>,
    peaks: Option<String>,
    updated_versions: HashMap<String, String>,
}

#[derive(Debug, Clone)]
struct IngestionVersionPlan {
    should_run_audio_upload: bool,
    should_run_keyframes: bool,
    should_run_metadata: bool,
    should_run_silence: bool,
    should_run_transcode_hls: bool,
    should_run_peaks: bool,
}

fn current_step_versions() -> HashMap<String, String> {
    vec![
        (STEP_AUDIO_UPLOAD.to_string(), STEP_VERSION_AUDIO_UPLOAD.to_string()),
        (STEP_KEYFRAMES.to_string(), STEP_VERSION_KEYFRAMES.to_string()),
        (STEP_METADATA.to_string(), STEP_VERSION_METADATA.to_string()),
        (STEP_SILENCE.to_string(), STEP_VERSION_SILENCE.to_string()),
        (STEP_TRANSCODE_HLS.to_string(), STEP_VERSION_TRANSCODE_HLS.to_string()),
        (STEP_PEAKS.to_string(), STEP_VERSION_PEAKS.to_string()),
    ]
    .into_iter()
    .collect()
}

async fn get_stored_ingestion_versions(
    aws_config: &SdkConfig,
    table_name: &str,
    input_key: &str,
) -> HashMap<String, String> {
    let dynamodb_client = aws_sdk_dynamodb::Client::new(aws_config);
    let item = dynamodb_client
        .get_item()
        .table_name(table_name)
        .key("key", AttributeValue::S(input_key.to_string()))
        .projection_expression("ingestion_versions, ingestion_version")
        .send()
        .await
        .expect("failed to get item from dynamodb");

    let mut versions = HashMap::new();
    if let Some(item_map) = item.item {
        if let Some(AttributeValue::M(map)) = item_map.get("ingestion_versions") {
            for (step, value) in map {
                if let AttributeValue::S(v) = value {
                    versions.insert(step.clone(), v.clone());
                }
            }

            if !versions.is_empty() {
                return versions;
            }
        }

        if let Some(AttributeValue::S(legacy_version)) =
            item_map.get("ingestion_version")
        {
            if legacy_version == LEGACY_GLOBAL_INGESTION_VERSION {
                versions.insert(
                    STEP_AUDIO_UPLOAD.to_string(),
                    STEP_VERSION_AUDIO_UPLOAD.to_string(),
                );
                versions.insert(
                    STEP_KEYFRAMES.to_string(),
                    STEP_VERSION_KEYFRAMES.to_string(),
                );
                versions.insert(
                    STEP_METADATA.to_string(),
                    STEP_VERSION_METADATA.to_string(),
                );
                versions.insert(
                    STEP_SILENCE.to_string(),
                    STEP_VERSION_SILENCE.to_string(),
                );
                versions.insert(
                    STEP_TRANSCODE_HLS.to_string(),
                    STEP_VERSION_TRANSCODE_HLS.to_string(),
                );
                versions.insert(
                    STEP_PEAKS.to_string(),
                    STEP_VERSION_PEAKS.to_string(),
                );
            }
        }
    }

    versions
}

fn build_ingestion_version_plan(
    stored_versions: &HashMap<String, String>,
    current_versions: &HashMap<String, String>,
) -> IngestionVersionPlan {
    let should_run_step = |step: &str| -> bool {
        match (stored_versions.get(step), current_versions.get(step)) {
            (Some(stored), Some(current)) => stored != current,
            _ => true,
        }
    };

    IngestionVersionPlan {
        should_run_audio_upload: should_run_step(STEP_AUDIO_UPLOAD),
        should_run_keyframes: should_run_step(STEP_KEYFRAMES),
        should_run_metadata: should_run_step(STEP_METADATA),
        should_run_silence: should_run_step(STEP_SILENCE),
        should_run_transcode_hls: should_run_step(STEP_TRANSCODE_HLS),
        should_run_peaks: should_run_step(STEP_PEAKS),
    }
}

/// Upload the already-extracted WAV file to S3 and return the S3 key.
fn upload_audio_to_s3(
    aws_config: &SdkConfig,
    audio_prefix: String,
    audio_temp_file_path: String,
    output_bucket: String,
    input_key: String,
) -> tokio::task::JoinHandle<String> {
    let s3_client = aws_sdk_s3::Client::new(aws_config);

    tokio::spawn(async move {
        let output_key = format!("{audio_prefix}/{input_key}");

        s3_client
            .put_object()
            .bucket(output_bucket)
            .key(output_key.as_str())
            .body(
                ByteStream::from_path(audio_temp_file_path)
                    .await
                    .unwrap(),
            )
            .send()
            .await
            .expect("failed to upload audio");

        output_key
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

fn do_transcode_task(
    aws_config: &SdkConfig,
    transcode_prefix: String,
    input_video_file_path: String,
    output_bucket: String,
    input_key: String,
) -> tokio::task::JoinHandle<Vec<gt_ffmpeg::transcode::HLSEntry>> {
    let s3_client = aws_sdk_s3::Client::new(aws_config);

    tokio::spawn(async move {
        // Create a temporary directory to store the transcode segments.
        let transcode_temp_dir =
            tempfile::tempdir().expect("failed to create temp dir");

        // Transcode the video file into HLS .ts segments
        let transcode_files = gt_ffmpeg::transcode::hls(
            transcode_temp_dir.path().to_str().unwrap(),
            &input_video_file_path,
            None,
        )
        .await
        .expect("failed to transcode");

        let mut transcode_keys = Vec::new();

        // Upload the transcoded files to an S3 bucket
        for transcode_file in transcode_files {
            let transcode_path = std::path::Path::new(&transcode_file.path);
            let transcode_basename = transcode_path
                .file_name()
                .expect("failed to get transcode filename")
                .to_str()
                .expect("failed to convert transcode filename to string")
                .to_string();

            let transcode_key =
                format!("{transcode_prefix}/{input_key}/{transcode_basename}");

            tracing::info!(
                "Uploading transcode: {} to {}",
                transcode_file.path,
                transcode_key
            );

            s3_client
                .put_object()
                .bucket(&output_bucket)
                .key(&transcode_key)
                .body(
                    ByteStream::from_path(transcode_file.path).await.unwrap(),
                )
                .metadata("duration", transcode_file.duration.to_string())
                .metadata("Content-Type", "video/MP2T")
                .send()
                .await
                .expect("failed to upload transcode");

            transcode_keys.push(HLSEntry {
                path: transcode_key.clone(),
                duration: transcode_file.duration,
            });
        }

        // Return the S3 keys of the transcoded files
        transcode_keys
    })
}

/// Compute per-second peak amplitudes from the WAV file, serialize to JSON,
/// upload to S3, and return the S3 key.
fn do_audio_peaks_task(
    aws_config: &SdkConfig,
    peaks_prefix: String,
    audio_temp_file_path: String,
    output_bucket: String,
    input_key: String,
) -> tokio::task::JoinHandle<String> {
    let s3_client = aws_sdk_s3::Client::new(aws_config);

    tokio::spawn(async move {
        let audio_peaks = gt_ffmpeg::audio_peaks::extract(&audio_temp_file_path)
            .await
            .expect("failed to compute audio peaks");

        let json = serde_json::json!({
            "peaks": audio_peaks.peaks,
            "duration": audio_peaks.duration,
        });

        let json_bytes = serde_json::to_vec(&json).expect("failed to serialize peaks");

        let output_key = format!("{peaks_prefix}/{input_key}.json");

        tracing::info!("Uploading audio peaks to {}", output_key);

        s3_client
            .put_object()
            .bucket(output_bucket)
            .key(&output_key)
            .body(ByteStream::from(json_bytes))
            .content_type("application/json")
            .send()
            .await
            .expect("failed to upload audio peaks");

        output_key
    })
}
