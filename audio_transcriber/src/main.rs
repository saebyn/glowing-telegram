use aws_config::{BehaviorVersion, meta::region::RegionProviderChain};
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_s3::primitives::ByteStream;
use figment::{Figment, providers::Env};
use serde::Deserialize;
use std::env;
use std::process::Stdio;
use tokio::{io::AsyncWriteExt, process::Command};
use types::{Silence, Transcription};

#[derive(Deserialize, Debug, Clone)]
struct Config {
    input_bucket: String,

    dynamodb_table: String,
}

fn load_config() -> Result<Config, figment::Error> {
    let figment = Figment::new().merge(Env::raw());

    figment.extract()
}

#[derive(Debug)]
struct AudioTranscriberError {
    pub message: String,
}

#[derive(Debug)]
enum WhisperModel {
    Tiny,
    Base,
    Small,
    Medium,
    Large,
    Turbo,
}

#[derive(Debug)]
struct WhisperOptions {
    pub model: WhisperModel,
    pub model_dir: String,
    pub initial_prompt: String,
    pub language: String,
    pub clip_timestamps: String,
    pub verbose: bool,
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
    let silence_segments = match get_silence_data_from_dynamodb(
        &dynamodb,
        &config.dynamodb_table,
        &item_key,
    )
    .await
    {
        Ok(segments) => {
            tracing::info!("Retrieved {} silence segments", segments.len());
            segments
        }
        Err(e) => {
            tracing::warn!(
                "Failed to retrieve silence data: {}. Using default transcription.",
                e
            );
            Vec::new()
        }
    };

    // Convert silence to clip timestamps
    let clip_timestamps =
        convert_silence_to_clip_timestamps(&silence_segments, None);
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
    let whisper_output =
        run_whisper_on_s3_object(&client, &config, &input_key, options)
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
            convert_transcription_to_attributevalue(whisper_output),
        )
        .send()
        .await
        .expect("failed to write to dynamodb");
}

async fn get_silence_data_from_dynamodb(
    dynamodb: &aws_sdk_dynamodb::Client,
    table_name: &str,
    item_key: &str,
) -> Result<Vec<Silence>, Box<dyn std::error::Error>> {
    let response = dynamodb
        .get_item()
        .table_name(table_name)
        .key("key", AttributeValue::S(item_key.to_string()))
        .send()
        .await?;

    let item = response.item.ok_or("Item not found in DynamoDB")?;

    let silence_attr = item.get("silence").ok_or("No silence data found")?;

    if let AttributeValue::L(silence_list) = silence_attr {
        let mut silence_segments = Vec::new();

        for segment_attr in silence_list {
            if let AttributeValue::M(segment_map) = segment_attr {
                let start = if let Some(AttributeValue::N(start_str)) =
                    segment_map.get("start")
                {
                    start_str.parse::<f64>()?
                } else {
                    continue;
                };

                let end = if let Some(AttributeValue::N(end_str)) =
                    segment_map.get("end")
                {
                    end_str.parse::<f64>()?
                } else {
                    continue;
                };

                silence_segments.push(Silence {
                    start: Some(start),
                    end: Some(end),
                });
            }
        }

        Ok(silence_segments)
    } else {
        Ok(Vec::new())
    }
}

fn convert_silence_to_clip_timestamps(
    silence_segments: &[Silence],
    total_duration: Option<f64>,
) -> String {
    if silence_segments.is_empty() {
        return "0".to_string();
    }

    let mut speaking_segments = Vec::new();
    let mut current_time = 0.0;

    // Sort silence segments by start time
    let mut sorted_silence: Vec<_> = silence_segments.iter().collect();
    sorted_silence.sort_by(|a, b| {
        let a_start = a.start.unwrap_or(0.0);
        let b_start = b.start.unwrap_or(0.0);
        a_start
            .partial_cmp(&b_start)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for silence in sorted_silence {
        let silence_start = silence.start.unwrap_or(0.0);
        let silence_end = silence.end.unwrap_or(0.0);

        // Add speaking segment before this silence
        if current_time < silence_start {
            speaking_segments.push(format!("{current_time},{silence_start}"));
        }

        current_time = silence_end;
    }

    // Add final speaking segment if there's time remaining
    if let Some(duration) = total_duration {
        if current_time < duration {
            speaking_segments.push(format!("{current_time},{duration}"));
        }
    } else if current_time > 0.0 {
        // If we don't have total duration, just add a segment from last silence end to a reasonable end
        speaking_segments.push(format!("{current_time}"));
    }

    if speaking_segments.is_empty() {
        "0".to_string()
    } else {
        speaking_segments.join(",")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_silence_to_clip_timestamps() {
        // Test case 1: No silence - should return "0"
        let no_silence = vec![];
        assert_eq!(
            convert_silence_to_clip_timestamps(&no_silence, Some(100.0)),
            "0"
        );

        // Test case 2: Single silence in middle - should return speaking segments around it
        let single_silence = vec![Silence {
            start: Some(30.0),
            end: Some(40.0),
        }];
        assert_eq!(
            convert_silence_to_clip_timestamps(&single_silence, Some(100.0)),
            "0,30,40,100"
        );

        // Test case 3: Multiple silences
        let multiple_silences = vec![
            Silence {
                start: Some(10.0),
                end: Some(20.0),
            },
            Silence {
                start: Some(50.0),
                end: Some(60.0),
            },
        ];
        assert_eq!(
            convert_silence_to_clip_timestamps(
                &multiple_silences,
                Some(100.0)
            ),
            "0,10,20,50,60,100"
        );

        // Test case 4: Silence at the beginning
        let silence_at_start = vec![Silence {
            start: Some(0.0),
            end: Some(10.0),
        }];
        assert_eq!(
            convert_silence_to_clip_timestamps(&silence_at_start, Some(100.0)),
            "10,100"
        );

        // Test case 5: Silence at the end
        let silence_at_end = vec![Silence {
            start: Some(90.0),
            end: Some(100.0),
        }];
        assert_eq!(
            convert_silence_to_clip_timestamps(&silence_at_end, Some(100.0)),
            "0,90"
        );
    }
}

fn convert_transcription_to_attributevalue(
    transcription: Transcription,
) -> AttributeValue {
    let segments = transcription
        .segments
        .iter()
        .map(|segment| {
            let mut map = std::collections::HashMap::new();

            map.insert(
                "start".to_string(),
                AttributeValue::N(segment.start.to_string()),
            );
            map.insert(
                "end".to_string(),
                AttributeValue::N(segment.end.to_string()),
            );
            map.insert(
                "text".to_string(),
                AttributeValue::S(segment.text.clone()),
            );
            map.insert(
                "tokens".to_string(),
                AttributeValue::L(
                    segment
                        .tokens
                        .iter()
                        .map(|token| AttributeValue::N(token.to_string()))
                        .collect(),
                ),
            );
            map.insert(
                "temperature".to_string(),
                AttributeValue::N(segment.temperature.to_string()),
            );
            map.insert(
                "avg_logprob".to_string(),
                AttributeValue::N(segment.avg_logprob.to_string()),
            );
            map.insert(
                "compression_ratio".to_string(),
                AttributeValue::N(segment.compression_ratio.to_string()),
            );
            map.insert(
                "no_speech_prob".to_string(),
                AttributeValue::N(segment.no_speech_prob.to_string()),
            );

            AttributeValue::M(map)
        })
        .collect();

    AttributeValue::M(
        vec![
            ("text".to_string(), AttributeValue::S(transcription.text)),
            ("segments".to_string(), AttributeValue::L(segments)),
            (
                "language".to_string(),
                AttributeValue::S(transcription.language),
            ),
        ]
        .into_iter()
        .collect(),
    )
}

async fn run_whisper_on_s3_object(
    client: &aws_sdk_s3::Client,
    config: &Config,
    input_key: &str,
    options: WhisperOptions,
) -> Result<Transcription, AudioTranscriberError> {
    // Get the audio file from S3
    let mut object = client
        .get_object()
        .bucket(config.input_bucket.clone())
        .key(input_key)
        .send()
        .await
        .map_err(|err| {
            tracing::error!("Error getting object from S3: {}", err);
            AudioTranscriberError {
                message: "Error getting object from S3".to_string(),
            }
        })?;

    run_whisper_on_bytestream(options, &mut object.body).await
}

async fn run_whisper_on_bytestream(
    options: WhisperOptions,
    bytestream: &mut ByteStream,
) -> Result<Transcription, AudioTranscriberError> {
    let temp_dir: tempfile::TempDir = tempfile::tempdir().map_err(|err| {
        tracing::error!("Error creating temp dir: {}", err);
        AudioTranscriberError {
            message: "Error creating temp dir".to_string(),
        }
    })?;

    let mut whisper_detection = match build_whisper_command(&temp_dir, options)
    {
        Ok(process) => process,
        Err(e) => {
            return Err(AudioTranscriberError {
                message: format!("Error running whisper: {e}"),
            });
        }
    };

    let mut whisper_stdin =
        whisper_detection.stdin.take().expect("failed to get stdin");

    let mut byte_count = 0_usize;
    while let Some(bytes) = bytestream.try_next().await.map_err(|err| {
        tracing::error!("Error reading from bytestream: {}", err);
        AudioTranscriberError {
            message: "Error reading from bytestream".to_string(),
        }
    })? {
        let bytes_len = bytes.len();

        whisper_stdin.write_all(&bytes).await.map_err(|err| {
            tracing::error!("Error writing to stdin: {}", err);

            AudioTranscriberError {
                message: "Error writing to stdin".to_string(),
            }
        })?;

        byte_count += bytes_len;

        tracing::debug!("Wrote {} bytes to stdin", bytes_len);
    }

    drop(whisper_stdin);

    tracing::info!("Wrote a total of {} bytes to stdin", byte_count);

    let whisper_status = match whisper_detection.wait().await {
        Ok(status) => status,
        Err(e) => {
            return Err(AudioTranscriberError {
                message: format!("Error waiting for whisper: {e}"),
            });
        }
    };

    if !whisper_status.success() {
        return Err(AudioTranscriberError {
            message: format!("Whisper failed with status: {whisper_status}"),
        });
    }

    // read the file and parse the json
    let transcription_json =
        match std::fs::read_to_string(temp_dir.path().join("-.json")) {
            Ok(transcription) => transcription,
            Err(e) => {
                return Err(AudioTranscriberError {
                    message: format!("Error reading transcription file: {e}"),
                });
            }
        };

    tracing::debug!("Transcription JSON: {}", transcription_json);

    serde_json::from_str::<Transcription>(&transcription_json).map_err(|err| {
        tracing::error!("Error parsing transcription json: {}", err);
        AudioTranscriberError {
            message: "Error parsing transcription json".to_string(),
        }
    })
}

fn build_whisper_command(
    temp_dir: &tempfile::TempDir,
    options: WhisperOptions,
) -> std::result::Result<tokio::process::Child, std::io::Error> {
    Command::new("whisper")
        .arg("--model")
        .arg(match options.model {
            WhisperModel::Tiny => "tiny",
            WhisperModel::Base => "base",
            WhisperModel::Small => "small",
            WhisperModel::Medium => "medium",
            WhisperModel::Large => "large",
            WhisperModel::Turbo => "turbo",
        })
        .arg("--initial_prompt")
        .arg(options.initial_prompt)
        .arg("--model_dir")
        .arg(options.model_dir)
        .arg("--output_format")
        .arg("json")
        .arg("--output_dir")
        .arg(temp_dir.path())
        .arg("--task")
        .arg("transcribe")
        .arg("--device")
        .arg("cuda")
        .arg("--language")
        .arg(options.language)
        .arg("--clip_timestamps")
        .arg(options.clip_timestamps)
        .arg("--verbose")
        .arg(if options.verbose { "True" } else { "False" })
        .arg("-")
        .stdin(Stdio::piped())
        .spawn()
}
