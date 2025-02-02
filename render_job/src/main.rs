use std::path::Path;

use aws_config::{BehaviorVersion, meta::region::RegionProviderChain};
use aws_sdk_s3::primitives::ByteStream;
use figment::{Figment, providers::Env};
use gt_ffmpeg::edit::build_ffmpeg_command;
use serde::Deserialize;
use tokio::task::JoinSet;
use types::{CutList, Episode};

const FRAME_RATE: f32 = 60.0;
const RESOLUTION: (u32, u32) = (2560, 1440);

#[derive(Debug, Clone, Deserialize)]
struct Config {
    input_bucket: String,
    output_bucket: String,
    dynamodb_table: String,
}

#[derive(Debug, Clone)]
struct AppContext {
    config: Config,
    dynamodb_client: aws_sdk_dynamodb::Client,
    s3_client: aws_sdk_s3::Client,
}

async fn initialize_app_context() -> Result<AppContext, figment::Error> {
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

/**
 * This program will take a project that describes a video editing job and
 * and will run the job using ffmpeg and upload the result to S3 and update
 * the project record with the output file path.
 */
#[tokio::main]
pub async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Read configuration from environment variables with figment
    let app_context = initialize_app_context().await?;

    // 1. get the record id from the command line for the project
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <record_id>", args[0]);
        std::process::exit(1);
    }
    let record_id = &args[1];
    // 2. transform the cut job record into the type `CutList`
    let cut_list = get_cut_list(&app_context, record_id).await?;
    // 3. download the input files from the input into a temporary directory
    let temp_input_dir = tempfile::tempdir()?;
    download_input_files(&app_context, &cut_list, temp_input_dir.path())
        .await?;
    // 4. build and run the command
    let temp_output_file = tempfile::NamedTempFile::new()?;
    run_command(&cut_list, temp_input_dir.path(), temp_output_file.path())
        .await?;
    // 5. upload the output file to the output bucket
    let output_location =
        upload_output_file(&app_context, temp_output_file.path()).await?;
    // 6. update the job record with the output file location
    update_job_record(&app_context, record_id, output_location).await?;
    Ok(())
}

async fn get_cut_list(
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

async fn download_input_files(
    context: &AppContext,
    cut_list: &CutList,
    temp_input_dir: &std::path::Path,
) -> Result<(), String> {
    let input_paths = cut_list
        .input_media
        .iter()
        .map(|input_media| input_media.s3_location.clone())
        .collect::<Vec<String>>();

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

async fn download_file(
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
            return Err("Error getting parent directory".to_string());
        }
    };

    tokio::fs::create_dir_all(containing_dir)
        .await
        .map_err(|e| {
            tracing::error!("Error creating directory: {:?}", e);
            "Error creating directory"
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

async fn run_command(
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

async fn upload_output_file(
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

async fn update_job_record(
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
