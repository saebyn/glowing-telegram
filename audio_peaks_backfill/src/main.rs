use aws_config::{BehaviorVersion, SdkConfig, meta::region::RegionProviderChain};
use aws_sdk_dynamodb::{Client as DynamoClient, types::AttributeValue};
use aws_sdk_s3::{Client as S3Client, primitives::ByteStream};
use clap::Parser;
use figment::{Figment, providers::Env};
use serde::Deserialize;
use tokio::io::AsyncWriteExt;

/// Compute per-second audio peak amplitudes for already-ingested video clips
/// and write `{ peaks, duration }` JSON to S3, then update DynamoDB.
///
/// Reads existing WAV files from the output bucket (stored under `audio/`),
/// so no re-extraction from the raw video archive is needed.
#[derive(Parser, Debug)]
#[command(author, version, about)]
#[clap(group = clap::ArgGroup::new("mode").required(true).args(["item_key", "scan"]))]
struct Args {
    /// Process a single video clip by its DynamoDB partition key.
    #[arg(long)]
    item_key: Option<String>,

    /// Scan all DynamoDB items that are missing the 'peaks' attribute.
    #[arg(long)]
    scan: bool,

    /// Print what would be done without writing anything to S3 or DynamoDB.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Deserialize, Debug)]
struct Config {
    /// Bucket that holds the extracted WAV files (same as output bucket).
    input_bucket: String,
    /// Bucket to write peaks JSON to (same bucket as above in practice).
    output_bucket: String,
    /// S3 key prefix for peaks JSON files, e.g. "peaks".
    peaks_prefix: String,
    /// Name of the DynamoDB videoMetadataTable.
    dynamodb_table: String,
}

fn load_config() -> Config {
    Figment::new()
        .merge(Env::raw())
        .extract()
        .expect("failed to load config from environment variables")
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let config = load_config();

    let region_provider =
        RegionProviderChain::default_provider().or_else("us-east-1");
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    if let Some(ref key) = args.item_key {
        process_single_item(&aws_config, &config, key, args.dry_run).await;
    } else {
        process_all_items(&aws_config, &config, args.dry_run).await;
    }
}

/// Fetch a single DynamoDB item by its partition key and process it.
async fn process_single_item(
    aws_config: &SdkConfig,
    config: &Config,
    item_key: &str,
    dry_run: bool,
) {
    let dynamo = DynamoClient::new(aws_config);

    let response = dynamo
        .get_item()
        .table_name(&config.dynamodb_table)
        .key("key", AttributeValue::S(item_key.to_string()))
        .send()
        .await
        .expect("failed to fetch item from DynamoDB");

    let item = match response.item {
        Some(item) => item,
        None => {
            tracing::warn!("No item found for key: {}", item_key);
            return;
        }
    };

    process_item(aws_config, config, &item, dry_run).await;
}

/// Scan all DynamoDB items missing the `peaks` attribute and process each one.
async fn process_all_items(
    aws_config: &SdkConfig,
    config: &Config,
    dry_run: bool,
) {
    let dynamo = DynamoClient::new(aws_config);

    let mut exclusive_start_key: Option<std::collections::HashMap<String, AttributeValue>> = None;
    let mut total = 0usize;
    let mut processed = 0usize;

    loop {
        let mut req = dynamo
            .scan()
            .table_name(&config.dynamodb_table)
            .filter_expression("attribute_not_exists(peaks)");

        if let Some(ref last_key) = exclusive_start_key {
            req = req.set_exclusive_start_key(Some(last_key.clone()));
        }

        let response = req.send().await.expect("DynamoDB scan failed");

        let items = response.items.unwrap_or_default();
        total += items.len();

        for item in &items {
            let key = item
                .get("key")
                .and_then(|v| v.as_s().ok())
                .map(String::as_str)
                .unwrap_or("<unknown>");

            tracing::info!("Processing item: {}", key);
            process_item(aws_config, config, item, dry_run).await;
            processed += 1;
        }

        exclusive_start_key = response.last_evaluated_key;
        if exclusive_start_key.is_none() {
            break;
        }
    }

    tracing::info!(
        "Scan complete: {} items found, {} processed",
        total,
        processed
    );
}

/// Process one DynamoDB item: download its WAV, compute peaks, upload JSON,
/// and update DynamoDB.
async fn process_item(
    aws_config: &SdkConfig,
    config: &Config,
    item: &std::collections::HashMap<String, AttributeValue>,
    dry_run: bool,
) {
    let item_key = match item.get("key").and_then(|v| v.as_s().ok()) {
        Some(k) => k.clone(),
        None => {
            tracing::warn!("Item is missing 'key' attribute, skipping");
            return;
        }
    };

    let audio_s3_key = match item.get("audio").and_then(|v| v.as_s().ok()) {
        Some(k) => k.clone(),
        None => {
            tracing::warn!(
                "Item '{}' has no 'audio' attribute, skipping",
                item_key
            );
            return;
        }
    };

    tracing::info!(
        "Computing peaks for '{}' from WAV '{}'",
        item_key,
        audio_s3_key
    );

    // Download the WAV to a temp file.
    let wav_path = download_wav(aws_config, &config.input_bucket, &audio_s3_key).await;

    // Compute peaks.
    let audio_peaks = gt_ffmpeg::audio_peaks::extract(&wav_path)
        .await
        .expect("failed to compute audio peaks");

    let peaks_s3_key = format!("{}/{}.json", config.peaks_prefix, audio_s3_key);

    if dry_run {
        tracing::info!(
            "[dry-run] Would upload {} peaks ({:.1}s) to s3://{}/{}",
            audio_peaks.peaks.len(),
            audio_peaks.duration,
            config.output_bucket,
            peaks_s3_key,
        );
        tracing::info!(
            "[dry-run] Would update DynamoDB item '{}' with peaks = '{}'",
            item_key,
            peaks_s3_key,
        );
        return;
    }

    // Serialize and upload JSON.
    let json_bytes = serde_json::to_vec(&serde_json::json!({
        "peaks": audio_peaks.peaks,
        "duration": audio_peaks.duration,
    }))
    .expect("failed to serialize peaks JSON");

    let s3 = S3Client::new(aws_config);
    s3.put_object()
        .bucket(&config.output_bucket)
        .key(&peaks_s3_key)
        .body(ByteStream::from(json_bytes))
        .content_type("application/json")
        .send()
        .await
        .expect("failed to upload peaks JSON to S3");

    tracing::info!("Uploaded peaks to s3://{}/{}", config.output_bucket, peaks_s3_key);

    // Update DynamoDB.
    let dynamo = DynamoClient::new(aws_config);
    dynamo
        .update_item()
        .table_name(&config.dynamodb_table)
        .key("key", AttributeValue::S(item_key.clone()))
        .update_expression("SET peaks = :peaks")
        .expression_attribute_values(":peaks", AttributeValue::S(peaks_s3_key.clone()))
        .send()
        .await
        .expect("failed to update DynamoDB item with peaks key");

    tracing::info!("Updated DynamoDB item '{}' with peaks = '{}'", item_key, peaks_s3_key);
}

/// Download an S3 object to a temp file and return the file path.
async fn download_wav(
    aws_config: &SdkConfig,
    bucket: &str,
    key: &str,
) -> String {
    let s3 = S3Client::new(aws_config);

    let wav_path = tempfile::NamedTempFile::new()
        .expect("failed to create temp file")
        .into_temp_path()
        .to_str()
        .expect("temp path is not valid UTF-8")
        .to_string();

    let mut object = s3
        .get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await
        .expect("failed to download WAV from S3");

    let mut file = tokio::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&wav_path)
        .await
        .expect("failed to open temp file for writing");

    while let Some(chunk) = object
        .body
        .try_next()
        .await
        .expect("error reading S3 object body")
    {
        file.write_all(&chunk)
            .await
            .expect("failed to write chunk to temp file");
    }

    file.flush().await.expect("failed to flush temp file");

    tracing::info!("Downloaded WAV to {}", wav_path);

    wav_path
}
