use gt_secrets::UserSecretPathProvider;
use serde::Deserialize;
use types::{Episode, YouTubeSessionSecret};
use youtube::UploadStatus;

mod youtube;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub episode_render_bucket: String,
    pub episode_table_name: String,

    pub user_secret_path: UserSecretPathProvider,

    pub max_retry_seconds: u64,
    pub user_agent: String,
}

#[derive(Debug, Clone)]
pub struct AppContext {
    pub config: Config,
    pub dynamodb_client: aws_sdk_dynamodb::Client,
    pub s3_client: aws_sdk_s3::Client,
    pub secrets_manager_client: aws_sdk_secretsmanager::Client,
    pub reqwest_client: reqwest::Client,
}

impl gt_app::ContextProvider<Config> for AppContext {
    fn new(config: Config, aws_config: aws_config::SdkConfig) -> Self {
        let dynamodb_client = aws_sdk_dynamodb::Client::new(&aws_config);
        let s3_client = aws_sdk_s3::Client::new(&aws_config);
        let secrets_manager_client =
            aws_sdk_secretsmanager::Client::new(&aws_config);

        let reqwest_client = reqwest::Client::builder()
            .user_agent(&config.user_agent)
            .build()
            .unwrap();

        Self {
            config,
            dynamodb_client,
            s3_client,
            secrets_manager_client,
            reqwest_client,
        }
    }
}

/**
 * This program will download the video files from S3 and upload them to
 * YouTube.
 *
 * This job will read the following fields from the episode record:
 *   - title
 *   - description
 *   - upload_attempts
 *   - user_id
 *   - tags
 *   - render_uri
 *
 * This job will update the episode record with:
 *   - youtube_video_id
 *   - upload_attempts (incremented each time the upload is attempted)
 *   - upload_status ("FAILED", "SUCCESS", "THROTTLED")
 *   - error_message (if an error occurs)
 *   - retry_after_seconds (if the upload fails and can be retried)
 *   - upload_resume_at_byte (if the upload fails and can be retried)   
 *
 * # Panics
 * This program will panic if the environment variables are not set correctly,
 * or if the episode record id is not provided as a command line argument.
 *
 */
#[tokio::main]
pub async fn main() {
    tracing_subscriber::fmt::init();

    // Read configuration from environment variables with figment
    let app_context = gt_app::create_app_context().await.unwrap();

    // 1. get the record ids from the command line for the projects
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} '<record_id>'", args[0]);
        std::process::exit(1);
    }
    let record_id = &args[1];

    // Upload the video to YouTube
    upload_video(&app_context, record_id).await.unwrap();
}

async fn upload_video(
    context: &AppContext,
    record_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // get the episode record
    tracing::info!("Getting episode record: {}", record_id);
    let episode: Episode = get_episode_record(context, record_id).await?;

    let user_id = episode.user_id.as_ref().ok_or("User ID not found")?;

    // download the video file from S3
    tracing::info!("Downloading video file: {:?}", episode.render_uri);
    let mut video_file: tokio::fs::File =
        download_video_file(context, &episode).await?;

    // get the user's YouTube session secret
    let access_token_path =
        context.config.user_secret_path.secret_path(user_id);

    tracing::info!("Getting YouTube session secret: {}", access_token_path);
    let access_token = gt_secrets::get::<YouTubeSessionSecret>(
        &context.secrets_manager_client,
        &access_token_path,
    )
    .await?
    .access_token
    .ok_or("Access token not found")?;

    let upload_url = if episode.youtube_upload_url.is_some() {
        tracing::info!("Using existing upload URL");
        episode.youtube_upload_url.as_ref().unwrap().to_string()
    } else {
        // create the target video upload URL
        tracing::info!("Creating upload URL");
        youtube::create_upload_url(
            &context.reqwest_client,
            access_token.as_str(),
            &episode,
            &video_file,
        )
        .await?
    };

    // upload the video to YouTube
    tracing::info!("Uploading video to YouTube");
    let mut start_byte = episode.upload_resume_at_byte.unwrap_or(0);
    let mut attempts = episode.upload_attempts.unwrap_or(0);
    loop {
        match youtube::upload_to_youtube(
            &context.reqwest_client,
            &access_token,
            &mut video_file,
            upload_url.as_str(),
            start_byte,
            attempts,
        )
        .await?
        {
            UploadStatus::Success { video_id } => {
                // if the upload was successful, update the episode record with the youtube_video_id
                update_episode_record_success(context, record_id, video_id)
                    .await?;
            }
            UploadStatus::TemporaryFailure {
                start_byte: new_start_byte,
                wait_time_ms,
            } => {
                // if there was a temporary failure, retry the upload if it can be retried
                // in less than the max seconds
                if wait_time_ms < context.config.max_retry_seconds * 1000 {
                    tracing::info!(
                        "Temporary failure, retrying in {} seconds",
                        wait_time_ms / 1000
                    );
                    start_byte = new_start_byte;
                    // wait for the retry_after_seconds
                    tokio::time::sleep(std::time::Duration::from_millis(
                        wait_time_ms,
                    ))
                    .await;

                    attempts += 1;

                    continue;
                }

                // if it would take too long, update the episode record with the retry_after_seconds
                tracing::info!("Temporary failure, returning to queue");
                update_episode_record_with_retry_after(
                    context,
                    record_id,
                    upload_url.as_str(),
                    wait_time_ms,
                    start_byte,
                )
                .await?;
            }
            UploadStatus::PermanentFailure => {
                tracing::error!("Permanent failure");
                // if there was a permanent failure, update the episode record with the error message
                update_episode_record_with_error(context, record_id).await?;
            }
        }
        break;
    }

    Ok(())
}

async fn get_episode_record(
    context: &AppContext,
    record_id: &str,
) -> Result<Episode, Box<dyn std::error::Error>> {
    let query = context
        .dynamodb_client
        .get_item()
        .table_name(&context.config.episode_table_name)
        .key(
            "id",
            aws_sdk_dynamodb::types::AttributeValue::S(record_id.to_string()),
        );

    let episode: Episode = query
        .send()
        .await?
        .item
        .ok_or("Record not found")?
        .try_into()?;

    Ok(episode)
}

async fn download_video_file(
    context: &AppContext,
    episode: &Episode,
) -> Result<tokio::fs::File, Box<dyn std::error::Error>> {
    let Some(render_uri) = &episode.render_uri else {
        return Err("Render URI not found".into());
    };

    let file_path = format!("/tmp/{}", render_uri.split('/').last().unwrap());

    let mut file = tokio::fs::File::create(&file_path).await?;

    let response = context
        .s3_client
        .get_object()
        .bucket(&context.config.episode_render_bucket)
        .key(render_uri)
        .send()
        .await?;

    let mut stream = response.body.into_async_read();

    tokio::io::copy(&mut stream, &mut file).await?;

    let file = tokio::fs::File::open(&file_path).await?;

    Ok(file)
}

async fn update_episode_record_success(
    context: &AppContext,
    record_id: &str,
    video_id: String,
) -> Result<(), Box<dyn std::error::Error>> {
    let update = context
        .dynamodb_client
        .update_item()
        .table_name(&context.config.episode_table_name)
        .key(
            "id",
            aws_sdk_dynamodb::types::AttributeValue::S(record_id.to_string()),
        )
        .update_expression(
            "SET youtube_video_id = :video_id, upload_status = :status",
        )
        .expression_attribute_values(
            ":video_id",
            aws_sdk_dynamodb::types::AttributeValue::S(video_id),
        )
        .expression_attribute_values(
            ":status",
            aws_sdk_dynamodb::types::AttributeValue::S("SUCCESS".to_string()),
        );

    update.send().await?;

    Ok(())
}

async fn update_episode_record_with_retry_after(
    context: &AppContext,
    record_id: &str,
    upload_url: &str,
    wait_time_ms: u64,
    start_byte: i64,
) -> Result<(), Box<dyn std::error::Error>> {
    let update = context
        .dynamodb_client
        .update_item()
        .table_name(&context.config.episode_table_name)
        .key(
            "id",
            aws_sdk_dynamodb::types::AttributeValue::S(record_id.to_string()),
        )
        .update_expression(
            "SET retry_after_seconds = :retry_after_seconds, upload_attempts = :attempts, upload_status = :status, upload_resume_at_byte = :start_byte, youtube_upload_url = :upload_url",
        )
        .expression_attribute_values(
            ":retry_after_seconds",
            aws_sdk_dynamodb::types::AttributeValue::N(
                (wait_time_ms / 1000).to_string(),
            ),
        )
        .expression_attribute_values(
            ":attempts",
            aws_sdk_dynamodb::types::AttributeValue::N("0".to_string()),
        )
        .expression_attribute_values(
            ":status",
            aws_sdk_dynamodb::types::AttributeValue::S("THROTTLED".to_string()),
        )
        .expression_attribute_values(
            ":start_byte",
            aws_sdk_dynamodb::types::AttributeValue::N(start_byte.to_string()),
        )
        .expression_attribute_values(
            ":upload_url",
            aws_sdk_dynamodb::types::AttributeValue::S(upload_url.to_string()),
        );

    update.send().await?;

    Ok(())
}

async fn update_episode_record_with_error(
    context: &AppContext,
    record_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let update = context
        .dynamodb_client
        .update_item()
        .table_name(&context.config.episode_table_name)
        .key(
            "id",
            aws_sdk_dynamodb::types::AttributeValue::S(record_id.to_string()),
        )
        .update_expression(
            "SET upload_status = :status, upload_attempts = :attempts, error_message = :error_message",
        )
        .expression_attribute_values(
            ":status",
            aws_sdk_dynamodb::types::AttributeValue::S("FAILED".to_string()),
        )
        .expression_attribute_values(
            ":attempts",
            aws_sdk_dynamodb::types::AttributeValue::N("0".to_string()),
        );

    update.send().await?;

    Ok(())
}
