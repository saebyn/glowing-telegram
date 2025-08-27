/**
 * This is the main entrypoint for the `embedding_service` application.
 *
 * The service is responsible for:
 * 1. Scanning existing stream data from DynamoDB to create embeddings
 * 2. Processing individual video clips to generate embeddings
 * 3. Storing embeddings in Aurora PostgreSQL with pgvector for retrieval
 */
use aws_config::{BehaviorVersion, meta::region::RegionProviderChain};
use aws_sdk_dynamodb::types::AttributeValue;
use figment::{Figment, providers::Env};
use openai_dive::v1::api::Client;
use openai_dive::v1::resources::embedding::{
    Embedding, EmbeddingInput, EmbeddingOutput, EmbeddingParametersBuilder,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use tokio_postgres::{Client as PgClient, NoTls};

#[derive(Deserialize, Debug, Clone)]
struct Config {
    dynamodb_table: String,
    database_secret_arn: String,
    database_endpoint: String,
    database_port: Option<String>,
    database_name: String,
    openai_secret_arn: String,
    openai_model: Option<String>,
    openai_base_url: Option<String>,
    aws_endpoint_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct DatabaseCredentials {
    username: String,
    password: String,
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

    // Load configuration first
    let region_provider =
        RegionProviderChain::default_provider().or_else("us-east-1");
    let mut aws_config_builder =
        aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider);

    if let Some(endpoint) = &config.aws_endpoint_url {
        tracing::info!("Using custom AWS endpoint: {}", endpoint);
        aws_config_builder = aws_config_builder.endpoint_url(endpoint);
    }

    let sdk_config = aws_config_builder.load().await;

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
            let db_client = connect_to_database(&config, &sdk_config)
                .await
                .expect("Failed to connect to database");
            init_database_schema(&db_client)
                .await
                .expect("Failed to initialize database");
            if let Err(e) =
                process_video_clip(&config, video_key, &db_client, &sdk_config)
                    .await
            {
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
    let mut aws_config_builder =
        aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider);

    if let Some(endpoint) = &config.aws_endpoint_url {
        tracing::info!("Using custom AWS endpoint: {}", endpoint);
        aws_config_builder = aws_config_builder.endpoint_url(endpoint);
    }

    let sdk_config = aws_config_builder.load().await;

    let dynamodb_client = aws_sdk_dynamodb::Client::new(&sdk_config);

    // Connect to the database
    let db_client = connect_to_database(config, &sdk_config).await?;

    // Initialize database schema
    init_database_schema(&db_client).await?;

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
                        match process_video_clip(
                            config,
                            video_key,
                            &db_client,
                            &sdk_config,
                        )
                        .await
                        {
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
    let mut aws_config_builder =
        aws_config::defaults(BehaviorVersion::latest())
            .region(region_provider);

    if let Some(endpoint) = &config.aws_endpoint_url {
        tracing::info!("Using custom AWS endpoint: {}", endpoint);
        aws_config_builder = aws_config_builder.endpoint_url(endpoint);
    }

    let sdk_config = aws_config_builder.load().await;

    let dynamodb_client = aws_sdk_dynamodb::Client::new(&sdk_config);

    // Connect to the database
    let db_client = connect_to_database(config, &sdk_config).await?;

    // Initialize database schema
    init_database_schema(&db_client).await?;

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
                    match process_video_clip(
                        config,
                        video_key,
                        &db_client,
                        &sdk_config,
                    )
                    .await
                    {
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

async fn connect_to_database(
    config: &Config,
    sdk_config: &aws_config::SdkConfig,
) -> Result<PgClient, Box<dyn std::error::Error>> {
    let secrets_client = aws_sdk_secretsmanager::Client::new(sdk_config);

    let secret_response = secrets_client
        .get_secret_value()
        .secret_id(&config.database_secret_arn)
        .send()
        .await?;

    let secret_string = secret_response
        .secret_string()
        .ok_or("No secret string found")?;

    let credentials: DatabaseCredentials =
        serde_json::from_str(secret_string)?;

    let port = config.database_port.as_deref().unwrap_or("5432");
    let connection_string = format!(
        "host={} port={} dbname={} user={} password={}",
        config.database_endpoint,
        port,
        config.database_name,
        credentials.username,
        credentials.password
    );

    let (client, connection) =
        tokio_postgres::connect(&connection_string, NoTls).await?;

    // Spawn the connection task
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            tracing::error!("Database connection error: {}", e);
        }
    });

    Ok(client)
}

async fn init_database_schema(
    client: &PgClient,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create pgvector extension if it doesn't exist
    client
        .execute("CREATE EXTENSION IF NOT EXISTS vector", &[])
        .await?;

    // Create embeddings table if it doesn't exist
    client
        .execute(
            r#"
        CREATE TABLE IF NOT EXISTS embeddings (
            id TEXT PRIMARY KEY,
            stream_id TEXT NOT NULL,
            video_key TEXT NOT NULL,
            content TEXT NOT NULL,
            content_type TEXT NOT NULL,
            embedding vector(1536),
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            metadata JSONB DEFAULT '{}'::jsonb
        )
        "#,
            &[],
        )
        .await?;

    // Create indexes for better query performance
    client.execute(
        "CREATE INDEX IF NOT EXISTS idx_embeddings_stream_id ON embeddings (stream_id)",
        &[],
    ).await?;

    client.execute(
        "CREATE INDEX IF NOT EXISTS idx_embeddings_video_key ON embeddings (video_key)",
        &[],
    ).await?;

    client.execute(
        "CREATE INDEX IF NOT EXISTS idx_embeddings_content_type ON embeddings (content_type)",
        &[],
    ).await?;

    // Create an HNSW index on the embedding column for fast similarity search
    client.execute(
        "CREATE INDEX IF NOT EXISTS idx_embeddings_embedding_hnsw ON embeddings USING hnsw (embedding vector_cosine_ops)",
        &[],
    ).await?;

    Ok(())
}

async fn process_video_clip(
    config: &Config,
    video_key: &str,
    db_client: &PgClient,
    sdk_config: &aws_config::SdkConfig,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing::debug!("Processing video clip: {}", video_key);

    let dynamodb_client = aws_sdk_dynamodb::Client::new(sdk_config);

    // Check if embeddings already exist for this video
    let existing_check = db_client
        .query(
            "SELECT COUNT(*) FROM embeddings WHERE video_key = $1",
            &[&video_key],
        )
        .await?;

    if let Some(row) = existing_check.get(0) {
        let count: i64 = row.get(0);
        if count > 0 {
            tracing::debug!(
                "Embeddings already exist for {}, skipping",
                video_key
            );
            return Ok(());
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

    // Get OpenAI API key and client
    let openai_client = get_openai_client(config, sdk_config).await?;

    // Generate embeddings for different content types
    let mut stored_count = 0;

    // Create embedding for transcription text
    if !transcription.is_empty() {
        let embedding =
            generate_embedding(&openai_client, &transcription, config).await?;

        let id = format!("{}:transcription", video_key);

        // Store in database
        db_client
            .execute(
                r#"
                INSERT INTO embeddings (id, stream_id, video_key, content, content_type, embedding, metadata)
                VALUES ($1, $2, $3, $4, $5, $6, $7)
                ON CONFLICT (id) DO UPDATE SET
                    content = EXCLUDED.content,
                    embedding = EXCLUDED.embedding,
                    created_at = NOW()
                "#,
                &[
                    &id,
                    &stream_id,
                    &video_key,
                    &transcription,
                    &"transcription",
                    &embedding,
                    &serde_json::to_string(&serde_json::json!({}))?,
                ],
            )
            .await?;

        stored_count += 1;
    }

    // Create embedding for summary if available
    if let Some(summary_text) = summary {
        if !summary_text.is_empty() {
            let embedding =
                generate_embedding(&openai_client, &summary_text, config)
                    .await?;

            let id = format!("{}:summary", video_key);

            // Store in database
            db_client
                .execute(
                    r#"
                    INSERT INTO embeddings (id, stream_id, video_key, content, content_type, embedding, metadata)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    ON CONFLICT (id) DO UPDATE SET
                        content = EXCLUDED.content,
                        embedding = EXCLUDED.embedding,
                        created_at = NOW()
                    "#,
                    &[
                        &id,
                        &stream_id,
                        &video_key,
                        &summary_text,
                        &"summary",
                        &embedding,
                        &serde_json::to_string(&serde_json::json!({}))?,
                    ],
                )
                .await?;

            stored_count += 1;
        }
    }

    if stored_count > 0 {
        tracing::debug!(
            "Stored {} embeddings for {}",
            stored_count,
            video_key
        );
    }

    Ok(())
}

async fn get_openai_client(
    config: &Config,
    sdk_config: &aws_config::SdkConfig,
) -> Result<Client, Box<dyn std::error::Error>> {
    let secrets_client = aws_sdk_secretsmanager::Client::new(sdk_config);

    let secret_response = secrets_client
        .get_secret_value()
        .secret_id(&config.openai_secret_arn)
        .send()
        .await?;

    let api_key = secret_response
        .secret_string()
        .ok_or("No secret string found")?;

    let mut client = Client::new(api_key.to_string());

    if let Some(base_url) = &config.openai_base_url {
        tracing::info!("Using custom OpenAI base URL: {}", base_url);
        client.base_url = base_url.clone();
    }

    Ok(client)
}

async fn generate_embedding(
    client: &Client,
    text: &str,
    config: &Config,
) -> Result<Vec<f64>, Box<dyn std::error::Error>> {
    let embedding_model: String = config
        .openai_model
        .clone()
        .unwrap_or("text-embedding-3-small".to_string());

    let parameters = EmbeddingParametersBuilder::default()
        .model(embedding_model)
        .input(EmbeddingInput::String(text.to_string()))
        .build()?;

    let response = client.embeddings().create(parameters).await?;

    if let Some(Embedding {
        embedding: EmbeddingOutput::Float(embedding),
        index: _,
        object: _,
    }) = response.data.first()
    {
        Ok(embedding.clone())
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
