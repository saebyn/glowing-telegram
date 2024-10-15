use aws_config::{meta::region::RegionProviderChain, BehaviorVersion};
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_s3::primitives::ByteStream;
use figment::{providers::Env, Figment};
use serde::Deserialize;
use std::env;
use std::process::Stdio;
use tokio::{io::AsyncWriteExt, process::Command};

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

    let options = WhisperOptions {
        model: WhisperModel::Turbo,
        model_dir: "/model/".to_string(),
        initial_prompt,
        language,
        // TODO: make this configurable
        clip_timestamps: "0".to_string(),
        verbose: false,
    };

    tracing::info!("Processing audio with key: {}", input_key);

    // Load AWS configuration
    let region_provider =
        RegionProviderChain::default_provider().or_else("us-east-1");
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    // Create an S3 client
    let client = aws_sdk_s3::Client::new(&aws_config);

    // capture output
    let whisper_output =
        run_whisper_on_s3_object(&client, &config, &input_key, options)
            .await
            .expect("failed to run whisper");

    // write output to dynamodb
    let dynamodb = aws_sdk_dynamodb::Client::new(&aws_config);

    dynamodb
        .update_item()
        .table_name(config.dynamodb_table.clone())
        .key("key", AttributeValue::S(item_key.clone()))
        .update_expression("SET transcription = :transcription")
        .expression_attribute_values(
            ":transcription",
            AttributeValue::from(whisper_output),
        )
        .send()
        .await
        .expect("failed to write to dynamodb");
}

#[derive(Debug, Deserialize)]
struct WhisperSegment {
    pub id: i32,
    pub seek: i32,
    pub start: f32,
    pub end: f32,
    pub text: String,
    pub tokens: Vec<i32>,
    pub temperature: f32,
    pub avg_logprob: f32,
    pub compression_ratio: f32,
    pub no_speech_prob: f32,
}

#[derive(Debug, Deserialize)]
struct WhisperOutput {
    pub text: String,
    pub segments: Vec<WhisperSegment>,
    pub language: String,
}

impl From<WhisperOutput> for AttributeValue {
    fn from(output: WhisperOutput) -> Self {
        let segments = output
            .segments
            .iter()
            .map(|segment| {
                let mut map = std::collections::HashMap::new();
                map.insert(
                    "start".to_string(),
                    Self::N(segment.start.to_string()),
                );
                map.insert(
                    "end".to_string(),
                    Self::N(segment.end.to_string()),
                );
                map.insert("text".to_string(), Self::S(segment.text.clone()));
                map.insert(
                    "tokens".to_string(),
                    Self::Ns(
                        segment
                            .tokens
                            .iter()
                            .map(|token| token.to_string())
                            .collect(),
                    ),
                );
                map.insert(
                    "temperature".to_string(),
                    Self::N(segment.temperature.to_string()),
                );
                map.insert(
                    "avg_logprob".to_string(),
                    Self::N(segment.avg_logprob.to_string()),
                );
                map.insert(
                    "compression_ratio".to_string(),
                    Self::N(segment.compression_ratio.to_string()),
                );
                map.insert(
                    "no_speech_prob".to_string(),
                    Self::N(segment.no_speech_prob.to_string()),
                );

                Self::M(map)
            })
            .collect();

        Self::M(
            vec![
                ("text".to_string(), Self::S(output.text)),
                ("segments".to_string(), Self::L(segments)),
                ("language".to_string(), Self::S(output.language)),
            ]
            .into_iter()
            .collect(),
        )
    }
}

async fn run_whisper_on_s3_object(
    client: &aws_sdk_s3::Client,
    config: &Config,
    input_key: &str,
    options: WhisperOptions,
) -> Result<WhisperOutput, AudioTranscriberError> {
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
) -> Result<WhisperOutput, AudioTranscriberError> {
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
            })
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
            })
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
                })
            }
        };

    tracing::debug!("Transcription JSON: {}", transcription_json);

    serde_json::from_str::<WhisperOutput>(&transcription_json).map_err(|err| {
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
        //.arg("--model_dir")
        //.arg(options.model_dir)
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
