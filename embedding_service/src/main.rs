/**
 * This is the main entrypoint for the `embedding_service` application.
 *
 * The service is responsible for:
 * 1. Scanning existing stream data from DynamoDB to create embeddings
 * 2. Processing individual video clips to generate embeddings
 * 3. Storing embeddings in S3 vector bucket for retrieval
 */
use aws_config::{BehaviorVersion, meta::region::RegionProviderChain};
use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_s3::primitives::ByteStream;
use figment::{Figment, providers::Env};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

#[derive(Serialize, Deserialize, Debug)]
struct OpenAIEmbeddingRequest {
    input: String,
    model: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct OpenAIEmbeddingResponse {
    data: Vec<OpenAIEmbeddingData>,
}

#[derive(Serialize, Deserialize, Debug)]
struct OpenAIEmbeddingData {
    embedding: Vec<f32>,
}

#[derive(Deserialize, Debug, Clone)]
struct Config {
    dynamodb_table: String,
    vector_bucket: String,
    openai_secret_arn: String,
    openai_model: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct VectorDocument {
    id: String,
    stream_id: String,
    video_key: String,
    content: String,
    content_type: String, // "transcription", "summary", "keywords"
    embedding: Vec<f32>,
    timestamp: String,
    metadata: HashMap<String, String>,
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

    if args.len() < 2 {
        eprintln!("Usage: embedding_service <command> [args...]");
        eprintln!("Commands:");
        eprintln!(
            "  scan                     - Scan all existing data and create embeddings"
        );
        eprintln!(
            "  process <video_key>      - Process a specific video clip"
        );
        eprintln!(
            "  scan-stream <stream_id>  - Scan all clips for a specific stream"
        );
        std::process::exit(1);
    }

    let command = &args[1];

    match command.as_str() {
        "scan" => {
            if let Err(e) = scan_all_data(&config).await {
                eprintln!("Error scanning data: {:?}", e);
                std::process::exit(1);
            }
        }
        "process" => {
            if args.len() != 3 {
                eprintln!("Usage: embedding_service process <video_key>");
                std::process::exit(1);
            }
            let video_key = &args[2];
            if let Err(e) = process_video_clip(&config, video_key).await {
                eprintln!("Error processing video clip: {:?}", e);
                std::process::exit(1);
            }
        }
        "scan-stream" => {
            if args.len() != 3 {
                eprintln!("Usage: embedding_service scan-stream <stream_id>");
                std::process::exit(1);
            }
            let stream_id = &args[2];
            if let Err(e) = scan_stream_data(&config, stream_id).await {
                eprintln!("Error scanning stream data: {:?}", e);
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            std::process::exit(1);
        }
    }
}

async fn scan_all_data(
    config: &Config,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting scan of all data for embedding generation");

    let region_provider =
        RegionProviderChain::default_provider().or_else("us-east-1");
    let sdk_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    let dynamodb_client = aws_sdk_dynamodb::Client::new(&sdk_config);

    // Scan DynamoDB table for all video clips that have transcriptions
    let mut scan_request = dynamodb_client
        .scan()
        .table_name(&config.dynamodb_table)
        .filter_expression("attribute_exists(transcription)");

    let mut processed_count = 0;
    let mut last_evaluated_key: Option<HashMap<String, AttributeValue>> = None;

    loop {
        if let Some(key) = &last_evaluated_key {
            scan_request =
                scan_request.set_exclusive_start_key(Some(key.clone()));
        }

        let response = scan_request.clone().send().await?;

        if let Some(items) = response.items {
            for item in items {
                if let Some(key_attr) = item.get("key") {
                    if let AttributeValue::S(video_key) = key_attr {
                        match process_video_clip(config, video_key).await {
                            Ok(_) => {
                                processed_count += 1;
                                if processed_count % 10 == 0 {
                                    tracing::info!(
                                        "Processed {} video clips",
                                        processed_count
                                    );
                                }
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to process video clip {}: {:?}",
                                    video_key,
                                    e
                                );
                            }
                        }
                    }
                }
            }
        }

        last_evaluated_key = response.last_evaluated_key;
        if last_evaluated_key.is_none() {
            break;
        }
    }

    tracing::info!(
        "Completed scan. Processed {} video clips",
        processed_count
    );
    Ok(())
}

async fn scan_stream_data(
    config: &Config,
    stream_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Scanning stream {} for embedding generation", stream_id);

    let region_provider =
        RegionProviderChain::default_provider().or_else("us-east-1");
    let sdk_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    let dynamodb_client = aws_sdk_dynamodb::Client::new(&sdk_config);

    // Query DynamoDB for videos in this stream
    let response = dynamodb_client
        .query()
        .table_name(&config.dynamodb_table)
        .index_name("stream_id-index")
        .key_condition_expression("stream_id = :stream_id")
        .expression_attribute_values(
            ":stream_id",
            AttributeValue::S(stream_id.to_string()),
        )
        .filter_expression("attribute_exists(transcription)")
        .send()
        .await?;

    if let Some(items) = response.items {
        let mut processed_count = 0;
        for item in items {
            if let Some(key_attr) = item.get("key") {
                if let AttributeValue::S(video_key) = key_attr {
                    match process_video_clip(config, video_key).await {
                        Ok(_) => processed_count += 1,
                        Err(e) => {
                            tracing::warn!(
                                "Failed to process video clip {}: {:?}",
                                video_key,
                                e
                            );
                        }
                    }
                }
            }
        }
        tracing::info!(
            "Processed {} video clips for stream {}",
            processed_count,
            stream_id
        );
    }

    Ok(())
}

async fn process_video_clip(
    config: &Config,
    video_key: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::debug!("Processing video clip: {}", video_key);

    let region_provider =
        RegionProviderChain::default_provider().or_else("us-east-1");
    let sdk_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    let dynamodb_client = aws_sdk_dynamodb::Client::new(&sdk_config);
    let s3_client = aws_sdk_s3::Client::new(&sdk_config);

    // Check if embedding already exists
    let embedding_key =
        format!("embeddings/{}.json", video_key.replace('/', "_"));
    match s3_client
        .head_object()
        .bucket(&config.vector_bucket)
        .key(&embedding_key)
        .send()
        .await
    {
        Ok(_) => {
            tracing::debug!(
                "Embedding already exists for {}, skipping",
                video_key
            );
            return Ok(());
        }
        Err(_) => {
            // Embedding doesn't exist, continue processing
        }
    }

    // Get video clip data from DynamoDB
    let response = dynamodb_client
        .get_item()
        .table_name(&config.dynamodb_table)
        .key("key", AttributeValue::S(video_key.to_string()))
        .send()
        .await?;

    let item = response.item.ok_or("Video clip not found")?;

    // Extract relevant data
    let stream_id = extract_string_attribute(&item, "stream_id")?;
    let transcription = extract_transcription(&item)?;
    let summary = extract_summary(&item);

    // Get OpenAI API key
    let openai_client = get_openai_client(config, &sdk_config).await?;

    // Generate embeddings for different content types
    let mut documents = Vec::new();

    // Create embedding for transcription text
    if !transcription.is_empty() {
        let embedding =
            generate_embedding(&openai_client, &transcription, config).await?;
        documents.push(VectorDocument {
            id: format!("{}:transcription", video_key),
            stream_id: stream_id.clone(),
            video_key: video_key.to_string(),
            content: transcription,
            content_type: "transcription".to_string(),
            embedding,
            timestamp: chrono::Utc::now().to_rfc3339(),
            metadata: HashMap::new(),
        });
    }

    // Create embedding for summary if available
    if let Some(summary_text) = summary {
        if !summary_text.is_empty() {
            let embedding =
                generate_embedding(&openai_client, &summary_text, config)
                    .await?;
            documents.push(VectorDocument {
                id: format!("{}:summary", video_key),
                stream_id: stream_id.clone(),
                video_key: video_key.to_string(),
                content: summary_text,
                content_type: "summary".to_string(),
                embedding,
                timestamp: chrono::Utc::now().to_rfc3339(),
                metadata: HashMap::new(),
            });
        }
    }

    // Store all embeddings in S3
    if !documents.is_empty() {
        let json_content = serde_json::to_string_pretty(&documents)?;
        s3_client
            .put_object()
            .bucket(&config.vector_bucket)
            .key(&embedding_key)
            .body(ByteStream::from(json_content.into_bytes()))
            .content_type("application/json")
            .send()
            .await?;

        tracing::debug!(
            "Stored {} embeddings for {}",
            documents.len(),
            video_key
        );
    }

    Ok(())
}

async fn get_openai_client(
    config: &Config,
    sdk_config: &aws_config::SdkConfig,
) -> Result<reqwest::Client, Box<dyn std::error::Error>> {
    let secrets_client = aws_sdk_secretsmanager::Client::new(sdk_config);

    let secret_response = secrets_client
        .get_secret_value()
        .secret_id(&config.openai_secret_arn)
        .send()
        .await?;

    let api_key = secret_response
        .secret_string()
        .ok_or("No secret string found")?;

    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()?;

    Ok(client)
}

async fn generate_embedding(
    client: &reqwest::Client,
    text: &str,
    config: &Config,
) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
    let model = config
        .openai_model
        .as_deref()
        .unwrap_or("text-embedding-3-small");

    let request = OpenAIEmbeddingRequest {
        input: text.to_string(),
        model: model.to_string(),
    };

    let response = client
        .post("https://api.openai.com/v1/embeddings")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        let error_text = response.text().await?;
        return Err(format!("OpenAI API error: {}", error_text).into());
    }

    let embedding_response: OpenAIEmbeddingResponse = response.json().await?;

    if let Some(embedding_data) = embedding_response.data.first() {
        Ok(embedding_data.embedding.clone())
    } else {
        Err("No embedding data returned".into())
    }
}

fn extract_string_attribute(
    item: &HashMap<String, AttributeValue>,
    key: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    match item.get(key) {
        Some(AttributeValue::S(value)) => Ok(value.clone()),
        _ => Err(format!("Missing or invalid {} attribute", key).into()),
    }
}

fn extract_transcription(
    item: &HashMap<String, AttributeValue>,
) -> Result<String, Box<dyn std::error::Error>> {
    match item.get("transcription") {
        Some(AttributeValue::M(transcription_map)) => {
            match transcription_map.get("text") {
                Some(AttributeValue::S(text)) => Ok(text.clone()),
                _ => Err("Missing transcription text".into()),
            }
        }
        _ => Err("Missing transcription".into()),
    }
}

fn extract_summary(item: &HashMap<String, AttributeValue>) -> Option<String> {
    if let Some(AttributeValue::M(summary_map)) = item.get("summary") {
        if let Some(AttributeValue::S(summary_text)) =
            summary_map.get("summary_main_discussion")
        {
            return Some(summary_text.clone());
        }
        if let Some(AttributeValue::S(summary_text)) = summary_map.get("title")
        {
            return Some(summary_text.clone());
        }
    }
    None
}
