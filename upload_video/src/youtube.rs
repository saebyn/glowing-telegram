use std::fmt::{self, Display};

use reqwest::Response;
use reqwest::Url;
use serde_json::json;
use tokio::io::AsyncSeekExt;
use tracing::instrument;
use types::utils::YouTubeCredentials;
use types::{Episode, YouTubeSessionSecret};

pub enum UploadStatus {
    Success { video_id: String },
    TemporaryFailure { start_byte: i64, wait_time_ms: u64 },
    PermanentFailure,
}

struct ResponseAttempt {
    response: Response,
    attempts: i64,
}

#[derive(serde::Deserialize)]
struct YouTubeUploadStatusResponse {
    id: String,
}

impl ResponseAttempt {
    const fn new(response: Response, attempts: i64) -> Self {
        Self { response, attempts }
    }

    async fn determine_upload_status(
        self,
    ) -> Result<UploadStatus, YouTubeError> {
        if self.response.status().is_success() {
            let result: YouTubeUploadStatusResponse =
                self.response.json().await?;
            Ok(UploadStatus::Success {
                video_id: result.id,
            })
        } else if self.response.status().as_u16() == 308 {
            let Some(range) = self
                .response
                .headers()
                .get("Range")
                .and_then(|v| v.to_str().ok())
            else {
                tracing::error!("Range header not found in response");
                return Ok(UploadStatus::PermanentFailure);
            };

            // parse the range header which looks like "bytes=0-12345"
            let range_parts =
                range.split(&['=', '-'][..]).collect::<Vec<&str>>();

            let Some(start_byte) = range_parts.get(1) else {
                tracing::error!("start byte not found in range header");
                return Ok(UploadStatus::PermanentFailure);
            };

            let start_byte = match start_byte.parse::<i64>() {
                Ok(start_byte) => start_byte,
                Err(e) => {
                    tracing::error!("failed to parse start byte: {:?}", e);
                    return Ok(UploadStatus::PermanentFailure);
                }
            };

            let wait_time_ms = match self
                .response
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
            {
                Some(wait_time) => {
                    match wait_time.parse::<u64>() {
                        Ok(wait_time) => wait_time * 1000,
                        Err(_) => {
                            // Retry-After is a date
                            match chrono::DateTime::parse_from_rfc2822(
                                wait_time,
                            ) {
                                Ok(wait_time) => {
                                    wait_time.timestamp_millis() as u64
                                }
                                Err(e) => {
                                    tracing::error!(
                                        "failed to parse Retry-After header: {:?}",
                                        e
                                    );
                                    BASE_WAIT_TIME
                                        * 2u64.pow(self.attempts as u32)
                                }
                            }
                        }
                    }
                }
                None => BASE_WAIT_TIME * 2u64.pow(self.attempts as u32),
            };

            Ok(UploadStatus::TemporaryFailure {
                start_byte,
                wait_time_ms,
            })
        } else {
            Ok(UploadStatus::PermanentFailure)
        }
    }
}

pub enum YouTubeError {
    UploadProcess,
    JsonParsing,
    FileIO,
}

impl From<reqwest::Error> for YouTubeError {
    fn from(_: reqwest::Error) -> Self {
        Self::UploadProcess
    }
}

impl From<std::io::Error> for YouTubeError {
    fn from(_: std::io::Error) -> Self {
        Self::FileIO
    }
}

impl From<serde_json::Error> for YouTubeError {
    fn from(_: serde_json::Error) -> Self {
        Self::JsonParsing
    }
}

impl Display for YouTubeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::UploadProcess => {
                write!(f, "Error uploading video to YouTube")
            }
            Self::JsonParsing => {
                write!(f, "Error parsing response from YouTube")
            }
            Self::FileIO => {
                write!(f, "Error reading file")
            }
        }
    }
}

impl std::fmt::Debug for YouTubeError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl std::error::Error for YouTubeError {}

const BASE_WAIT_TIME: u64 = 1000;

#[instrument]
async fn get_upload_status(
    http_client: &reqwest::Client,
    file_size: u64,
    upload_url: &str,
    access_token: &str,
    attempts: i64,
) -> Result<UploadStatus, YouTubeError> {
    // get the upload status from the Youtube API
    let response = http_client
        .put(upload_url)
        .header("Authorization", format!("Bearer {access_token}"))
        .header("Content-Range", format!("bytes */{file_size}"))
        .send()
        .await?;

    ResponseAttempt::new(response, attempts)
        .determine_upload_status()
        .await
}

#[instrument]
async fn handle_chunked_upload(
    http_client: &reqwest::Client,
    file: &mut tokio::fs::File,
    start_byte: i64,
    upload_url: &str,
    content_type: &str,
    access_token: &str,
) -> Result<UploadStatus, YouTubeError> {
    let mut request = http_client
        .put(upload_url)
        .header("Content-Type", content_type)
        .header("Authorization", format!("Bearer {access_token}"));

    let file_size = file.metadata().await?.len();

    if start_byte > 0 {
        request = request.header(
            "Content-Range",
            format!("bytes {}-{}/{}", start_byte, file_size - 1, file_size),
        );

        file.seek(std::io::SeekFrom::Start(start_byte as u64))
            .await?;
    }

    // upload the contents of the video using the upload URL and
    // chunked upload
    let file = file.try_clone().await?;
    let response = request.body(file).send().await?;

    tracing::trace!("Response: {:?}", response);

    ResponseAttempt::new(response, 0)
        .determine_upload_status()
        .await
}

#[instrument(skip(reqwest_client, access_token, video_file))]
pub async fn upload_to_youtube(
    reqwest_client: &reqwest::Client,
    access_token: &str,
    video_file: &mut tokio::fs::File,
    upload_url: &str,
    start_byte: i64,
    attempts: i64,
) -> Result<UploadStatus, YouTubeError> {
    match handle_chunked_upload(
        reqwest_client,
        video_file,
        start_byte,
        upload_url,
        "video/mp4",
        access_token,
    )
    .await
    {
        Ok(UploadStatus::TemporaryFailure { .. }) => {
            let status = get_upload_status(
                reqwest_client,
                video_file.metadata().await?.len(),
                upload_url,
                access_token,
                attempts,
            )
            .await?;

            Ok(status)
        }
        Ok(other) => Ok(other),
        Err(err) => {
            tracing::error!("Error uploading to YouTube: {:?}", err);
            Ok(UploadStatus::PermanentFailure)
        }
    }
}

pub async fn create_upload_url(
    reqwest_client: &reqwest::Client,
    access_token: &str,
    episode: &Episode,
    video_file: &tokio::fs::File,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut url =
        Url::parse("https://www.googleapis.com/upload/youtube/v3/videos")?;

    // add query parameters to the URL
    url.query_pairs_mut()
        .append_pair("uploadType", "resumable")
        .append_pair("part", "snippet,status")
        .append_pair(
            "notifySubscribers",
            episode
                .notify_subscribers
                .unwrap_or(false)
                .to_string()
                .as_str(),
        )
        .finish();

    tracing::debug!("URL: {:?}", url);

    let response = reqwest_client
        .post(url)
        .header("Authorization", format!("Bearer {access_token}"))
        .header("Content-Type", "application/json")
        .header(
            "X-Upload-Content-Length",
            video_file.metadata().await?.len().to_string(),
        )
        .header("X-Upload-Content-Type", "video/mp4")
        .json(&json!({
            "snippet": {
                "title": episode.title,
                "description": episode.description,
                "tags": episode.tags,
                "categoryId": episode.category,
                "defaultLanguage": "en-US",
                "defaultAudioLanguage": "en-US",
            },
            "status": {
                "privacyStatus": "private",
                "embeddable": true,
                "license": "creativeCommon",
                "selfDeclaredMadeForKids": false,
            },
        }))
        .send()
        .await?;

    tracing::trace!("Response: {:?}", response);

    let location = response
        .headers()
        .get("Location")
        .ok_or("Location header not found")?
        .to_str()?;

    Ok(location.to_string())
}

pub async fn get_access_token(
    secrets_manager_client: &aws_sdk_secretsmanager::Client,
    youtube_secret_arn: &str,
    access_token_path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let credentials = match secrets_manager_client
        .get_secret_value()
        .secret_id(youtube_secret_arn)
        .send()
        .await
    {
        Ok(secret) => {
            match serde_json::from_str::<YouTubeCredentials>(
                secret.secret_string.as_deref().unwrap_or("{}"),
            ) {
                Ok(credentials) => credentials,
                Err(e) => {
                    tracing::error!("failed to parse YouTube secret: {:?}", e);
                    return Err(e.into());
                }
            }
        }
        Err(e) => {
            tracing::error!("failed to get YouTube secret: {:?}", e);
            return Err(e.into());
        }
    };

    tracing::info!("Getting YouTube session secret: {}", access_token_path);
    let session = gt_secrets::get::<YouTubeSessionSecret>(
        secrets_manager_client,
        access_token_path,
    )
    .await?;

    let Some(refresh_token) = session.refresh_token else {
        tracing::error!("Refresh token not found in secret");
        return Err("Refresh token not found".into());
    };

    // use the refresh token to get a new access token
    let response = reqwest::Client::new()
        .post("https://oauth2.googleapis.com/token")
        .form(&[
            ("client_id", credentials.client_id),
            (
                "client_secret",
                credentials.client_secret.expose_secret().clone(),
            ),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token".to_string()),
        ])
        .send()
        .await?;

    let response_json: serde_json::Value = response.json().await?;

    let access_token = response_json
        .get("access_token")
        .ok_or("access_token not found in response")?
        .as_str()
        .ok_or("access_token not a string")?;

    Ok(access_token.to_string())
}
