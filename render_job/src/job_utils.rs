use aws_config::{BehaviorVersion, meta::region::RegionProviderChain};
use aws_sdk_s3::primitives::ByteStream;
use figment::{Figment, providers::Env};
use gt_ffmpeg::edit::build_ffmpeg_command;
use serde::Deserialize;
use std::path::Path;
use tokio::task::JoinSet;
use types::{CutList, Episode};

const FRAME_RATE: f32 = 60.0;
const RESOLUTION: (u32, u32) = (2560, 1440);

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub input_bucket: String,
    pub output_bucket: String,
    pub dynamodb_table: String,
}

#[derive(Debug, Clone)]
pub struct AppContext {
    pub config: Config,
    pub dynamodb_client: aws_sdk_dynamodb::Client,
    pub s3_client: aws_sdk_s3::Client,
}

pub async fn initialize_app_context() -> Result<AppContext, figment::Error> {
    let figment = Figment::new().merge(Env::raw());

    let config = figment.extract()?;

    let region_provider =
        RegionProviderChain::default_provider().or_else("us-east-1");
    let aws_config = aws_config::defaults(BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    Ok(AppContext {
        config,
        dynamodb_client: aws_sdk_dynamodb::Client::new(&aws_config),
        s3_client: aws_sdk_s3::Client::new(&aws_config),
    })
}

pub async fn get_cut_list(
    context: &AppContext,
    record_id: &str,
) -> Result<CutList, Box<dyn std::error::Error>> {
    let item_output = context
        .dynamodb_client
        .get_item()
        .table_name(&context.config.dynamodb_table)
        .key(
            "id",
            aws_sdk_dynamodb::types::AttributeValue::S(record_id.to_string()),
        )
        .send()
        .await?;

    match item_output.item {
        Some(item) => {
            let episode: Episode = serde_dynamo::from_item(item)?;

            match episode.cut_list {
                Some(cut_list) => Ok(cut_list.into()),
                None => Err("Cut list not found".into()),
            }
        }
        None => Err("Record not found".into()),
    }
}

pub async fn download_input_files(
    context: &AppContext,
    cut_lists: &[CutList],
    temp_input_dir: &std::path::Path,
) -> Result<(), String> {
    let mut input_paths = Vec::new();
    for cut_list in cut_lists {
        for input_media in &cut_list.input_media {
            input_paths.push(input_media.s3_location.clone());
        }
    }

    // check that none of the input files have absolute paths or contain ".."
    if input_paths.iter().any(|path_string| {
        let path = Path::new(path_string);
        path.is_absolute()
            || path.components().any(|component| {
                matches!(component, std::path::Component::ParentDir)
            })
    }) {
        return Err(format!("Invalid input file paths: {:?}", input_paths));
    }

    // download the input files in parallel and wait for them all to finish
    let mut set = JoinSet::new();
    for s3_object_key in input_paths {
        let context = context.clone();
        let s3_object_key = s3_object_key.clone();
        let temp_input_dir = temp_input_dir.to_path_buf();

        let fut = async move {
            download_file(&context, s3_object_key, &temp_input_dir).await
        };
        set.spawn(fut);
    }

    while let Some(res) = set.join_next().await {
        match res {
            Ok(_) => {}
            Err(_) => {
                tracing::error!("Error downloading input files");
                return Err("Error downloading input files".to_string());
            }
        };
    }

    Ok(())
}

pub async fn download_file(
    context: &AppContext,
    s3_object_key: String,
    temp_input_dir: &Path,
) -> Result<(), String> {
    let path = Path::new(&s3_object_key);
    let temp_input_dir = temp_input_dir.to_path_buf();
    let temp_file_path = temp_input_dir.join(path);
    let download_output = context
        .s3_client
        .get_object()
        .bucket(context.config.input_bucket.clone())
        .key(s3_object_key)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("Error downloading file: {:?}", e);
            "Error downloading file"
        })?;

    let mut reader = download_output.body.into_async_read();

    let containing_dir = match temp_file_path.parent() {
        Some(dir) => dir,
        None => {
            return Err(format!("Error getting parent directory for path: {:?}", temp_file_path));
        }
    };

    tokio::fs::create_dir_all(containing_dir)
        .await
        .map_err(|e| {
            tracing::error!("Error creating directory {:?}: {:?}", containing_dir, e);
            format!("Error creating directory {:?}", containing_dir)
        })?;

    let mut file =
        tokio::fs::File::create(temp_file_path).await.map_err(|e| {
            tracing::error!("Error creating file: {:?}", e);
            "Error creating file"
        })?;
    tokio::io::copy(&mut reader, &mut file).await.map_err(|e| {
        tracing::error!("Error writing file: {:?}", e);
        "Error writing file"
    })?;

    Ok(())
}

pub async fn run_command(
    cut_list: &CutList,
    temp_input_dir: &std::path::Path,
    output_file: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut command = build_ffmpeg_command(
        cut_list,
        FRAME_RATE,
        output_file.to_string_lossy().as_ref(),
        RESOLUTION,
    );

    command.current_dir(temp_input_dir);

    let output = command.spawn()?.wait_with_output().await?;

    if !output.status.success() {
        tracing::error!(
            "ffmpeg failed with status: {}",
            output.status.code().unwrap_or(-1)
        );
        return Err("ffmpeg failed".into());
    }

    Ok(())
}

pub async fn upload_output_file(
    context: &AppContext,
    output_file: &std::path::Path,
) -> Result<String, Box<dyn std::error::Error>> {
    // generate a unique key for the output file using the current time
    let output_key = format!(
        "renders/{}.mp4",
        chrono::Utc::now().format("%Y/%m/%d-%H-%M-%S")
    );

    context
        .s3_client
        .put_object()
        .bucket(context.config.output_bucket.clone())
        .key(output_key.clone())
        .body(ByteStream::from_path(output_file).await?)
        .send()
        .await?;

    tracing::info!("Uploaded output file to: {}", output_key);
    Ok(output_key.clone())
}

pub async fn update_job_record(
    context: &AppContext,
    record_id: &str,
    output_location: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let update_item_output = context
        .dynamodb_client
        .update_item()
        .table_name(&context.config.dynamodb_table)
        .key(
            "id",
            aws_sdk_dynamodb::types::AttributeValue::S(record_id.to_string()),
        )
        .update_expression("SET render_uri = :output_location")
        .expression_attribute_values(
            ":output_location",
            aws_sdk_dynamodb::types::AttributeValue::S(output_location),
        )
        .send()
        .await?;

    tracing::info!("Updated job record: {:?}", update_item_output);
    Ok(())
}
