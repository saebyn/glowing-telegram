use reqwest::Response;
use tokio::io::AsyncSeekExt;
use tracing::instrument;

enum UploadStatus {
    Success { video_id: String },
    TemporaryFailure { start_byte: u64, wait_time_ms: u64 },
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
    fn new(response: Response, attempts: i64) -> Self {
        Self { response, attempts }
    }

    fn increment_attempts(&mut self) {
        self.attempts += 1;
    }

    async fn determine_upload_status(
        self,
    ) -> Result<UploadStatus, Box<dyn std::error::Error>> {
        if self.response.status().is_success() {
            let result: YouTubeUploadStatusResponse =
                self.response.json().await?;
            Ok(UploadStatus::Success {
                video_id: result.id,
            })
        } else if self.response.status().as_u16() == 308 {
            let range = match self
                .response
                .headers()
                .get("Range")
                .and_then(|v| v.to_str().ok())
            {
                Some(range) => range,
                None => {
                    tracing::error!("Range header not found in response");
                    return Ok(UploadStatus::PermanentFailure);
                }
            };

            // parse the range header which looks like "bytes=0-12345"
            let range_parts =
                range.split(&['=', '-'][..]).collect::<Vec<&str>>();

            let start_byte = match range_parts.get(1) {
                Some(start_byte) => start_byte,
                None => {
                    tracing::error!("start byte not found in range header");
                    return Ok(UploadStatus::PermanentFailure);
                }
            };

            let start_byte = match start_byte.parse::<u64>() {
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

const BASE_WAIT_TIME: u64 = 1000;

#[instrument]
async fn get_upload_status(
    http_client: &reqwest::Client,
    file_size: u64,
    upload_url: &str,
    access_token: &str,
    attempts: i64,
) -> Result<UploadStatus, Box<dyn std::error::Error>> {
    // get the upload status from the Youtube API
    let response = http_client
        .put(upload_url)
        .header("Authorization", format!("Bearer {}", access_token))
        .header("Content-Range", format!("bytes */{}", file_size))
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
    start_byte: u64,
    upload_url: &str,
    content_type: &str,
    access_token: &str,
) -> Result<UploadStatus, Box<dyn std::error::Error>> {
    let mut request = http_client
        .put(upload_url)
        .header("Content-Type", content_type)
        .header("Authorization", format!("Bearer {}", access_token));

    let file_size = file.metadata().await?.len();

    if start_byte > 0 {
        request = request.header(
            "Content-Range",
            format!("bytes {}-{}/{}", start_byte, file_size - 1, file_size),
        );

        file.seek(std::io::SeekFrom::Start(start_byte)).await?;
    }

    // upload the contents of the video using the upload URL and
    // chunked upload
    let file = file.try_clone().await?;
    let response = request.body(file).send().await?;

    ResponseAttempt::new(response, 0)
        .determine_upload_status()
        .await
}

pub async fn upload_to_youtube(
    reqwest_client: &reqwest::Client,
    access_token: String,
    video_file: &mut tokio::fs::File,
    upload_url: &str,
    start_byte: u64,
    attempts: i64,
) -> Result<UploadStatus, Box<dyn std::error::Error>> {
    match handle_chunked_upload(
        reqwest_client,
        video_file,
        start_byte,
        &upload_url,
        "video/mp4",
        access_token.as_str(),
    )
    .await
    {
        Ok(UploadStatus::TemporaryFailure { .. }) => {
            let status = get_upload_status(
                reqwest_client,
                video_file.metadata().await?.len(),
                &upload_url,
                access_token.as_str(),
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
