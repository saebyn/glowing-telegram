use axum::extract::{FromRequestParts, State};
use task_worker::{PayloadTransform, TaskRequest, TaskTemplate};

use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    PkceCodeChallenge, RedirectUrl, RefreshToken, Scope, TokenUrl,
};
use oauth2::{PkceCodeVerifier, TokenResponse};

use crate::config;
use crate::state::AppState;
use crate::task::{self};
use axum::http::request::Parts;
use axum::{async_trait, Json};
use axum::{
    http::{header, StatusCode},
    response::IntoResponse,
};
use redact::Secret;
use redis;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::io::AsyncSeekExt;
use tracing::instrument;
use url::Url;
use validator::Validate;

// Redis key constants
const ACCESS_TOKEN_KEY: &str = "youtube:access_token";
const REFRESH_TOKEN_KEY: &str = "youtube:refresh_token";

const YOUTUBE_API_QUOTA_RETRY_AFTER: u64 = 24 /* hours */ * 60 /* minutes */ * 60 /* seconds */;

#[derive(Serialize, Debug, Deserialize, Validate)]
pub struct YoutubeUploadRequest {
    episode_id: String,
    #[validate(length(min = 1, max = 100))]
    pub title: String,
    #[validate(length(min = 1, max = 5000))]
    pub description: String,
    #[serde(default = "default_language")]
    language: String,
    tags: Vec<String>,
    category: u8,
    render_uri: String,
    #[serde(default = "default_mime_type")]
    mime_type: String,
    thumbnail_uri: Option<String>,
    recording_date: Option<iso8601::DateTime>,
    playlist_id: Option<String>,
    playlist_position: Option<u32>,
    notify_subscribers: bool,

    task_title: String,
}

#[derive(Serialize, Debug, Deserialize, Validate)]
pub struct YoutubeUploadTaskPayload {
    #[validate(length(min = 1, max = 100))]
    pub title: String,
    #[validate(length(min = 1, max = 5000))]
    pub description: String,
    pub language: String,
    pub mime_type: String,
    pub tags: Vec<String>,
    pub category: u8,
    pub render_uri: String,
    pub thumbnail_uri: Option<String>,
    pub recording_date: Option<iso8601::DateTime>,
    pub notify_subscribers: bool,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct YoutubeUploadResponse {
    video_id: String,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct YoutubePlaylistAddTaskPayload {
    playlist_id: String,
    playlist_position: Option<u32>,
    #[serde(rename = "@previous_task_data")]
    previous_task_data: Vec<YoutubeUploadResponse>,
}

impl From<&YoutubeUploadRequest> for YoutubeUploadTaskPayload {
    fn from(request: &YoutubeUploadRequest) -> Self {
        YoutubeUploadTaskPayload {
            title: request.title.clone(),
            description: request.description.clone(),
            language: request.language.clone(),
            mime_type: request.mime_type.clone(),
            tags: request.tags.clone(),
            category: request.category,
            render_uri: request.render_uri.clone(),
            thumbnail_uri: request.thumbnail_uri.clone(),
            recording_date: request.recording_date,
            notify_subscribers: request.notify_subscribers,
        }
    }
}

// Create a way to transform a YoutubeUploadRequest and an AppState into a TaskRequest, but TaskRequest is not defined in this file.
impl YoutubeUploadRequest {
    pub fn to_task_request(&self, app_state: &AppState) -> TaskRequest {
        let sync_video_id_task = TaskTemplate {
            title: "Save video ID".to_string(),
            payload: json!(YoutubeUploadTaskPayload::from(self)),
            data_key: "summary".to_string(),
            next_task: None,

            http_method: reqwest::Method::PUT,
            url: format!(
                "{}/records/episodes/{}",
                app_state.config.this_api_base_url, self.episode_id
            ),

            payload_transformer: Some(vec![PayloadTransform {
                source_pointer: "/@previous_task_data/0/youtube_video_id"
                    .to_string(),
                destination_key: "youtube_video_id".to_string(),
            }]),
        };

        let after_upload_task = match &self.playlist_id {
            None => sync_video_id_task,

            Some(playlist_id) => TaskTemplate {
                url: format!(
                    "{}/youtube/playlist/add/task",
                    app_state.config.this_api_base_url
                ),
                title: "Add video to playlist".to_string(),
                payload: json!(YoutubePlaylistAddTaskPayload {
                    playlist_id: playlist_id.to_string(),
                    playlist_position: self.playlist_position,
                    previous_task_data: vec![]
                }),
                data_key: "summary".to_string(),
                next_task: Some(Box::new(sync_video_id_task)),

                http_method: reqwest::Method::POST,
                payload_transformer: None,
            },
        };

        TaskRequest {
            url: format!(
                "{}/youtube/upload/task",
                app_state.config.this_api_base_url
            ),
            title: self.task_title.clone(),
            payload: json!(YoutubeUploadTaskPayload::from(self)),
            http_method: reqwest::Method::POST,
            payload_transformer: None,
            data_key: "summary".to_string(),

            next_task: Some(after_upload_task),
        }
    }
}

fn default_mime_type() -> String {
    "video/mp4".to_string()
}

fn default_language() -> String {
    "en-US".to_string()
}

#[instrument]
pub async fn upload_start_task_handler(
    State(state): State<AppState>,
    AccessToken(access_token): AccessToken,
    Json(body): Json<YoutubeUploadRequest>,
) -> impl IntoResponse {
    let task_url = match task::start(
        task::Context {
            http_client: state.http_client.clone(),
            task_api_url: state.config.task_api_url.clone(),
            task_api_external_url: state.config.task_api_external_url.clone(),
        },
        YoutubeUploadRequest::to_task_request(&body, &state),
    )
    .await
    {
        Ok(task_url) => task_url,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": e })),
            )
                .into_response()
        }
    };

    (StatusCode::ACCEPTED, [(header::LOCATION, task_url)]).into_response()
}

#[derive(Serialize, Debug)]
struct UploadVideoTaskOutput {
    cursor: Option<()>,
    summary: serde_json::Value,
}

#[instrument]
pub async fn upload_video_handler(
    State(state): State<AppState>,
    AccessToken(access_token): AccessToken,
    Json(body): Json<YoutubeUploadTaskPayload>,
) -> impl IntoResponse {
    // extract filename from uri
    let uri = body.render_uri.clone();
    let filename = match uri.split(&['/', ':'][..]).last() {
        Some(filename) => filename,
        None => {
            return (StatusCode::BAD_REQUEST, "invalid uri").into_response()
        }
    };

    // get the full path to the file on disk
    let path = format!(
        "{}/{}",
        state.config.rendered_episode_storage_path, filename
    );

    // get the length of the file in bytes for the Content-Length header
    let content_length = match tokio::fs::metadata(&path).await {
        Ok(metadata) => metadata.len(),
        Err(e) => {
            tracing::error!("failed to get file metadata: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "failed to get file metadata",
            )
                .into_response();
        }
    };

    // get the upload URL
    let mut url =
        Url::parse("https://www.googleapis.com/upload/youtube/v3/videos")
            .expect("failed to parse URL");

    // add query parameters to the URL
    url.query_pairs_mut()
        .append_pair("uploadType", "resumable")
        .append_pair("part", "snippet,status,recordingDetails")
        .append_pair("notifySubscribers", &body.notify_subscribers.to_string())
        .finish();

    // convert the recording date to an ISO8601 string
    let recording_date_iso8601 = body
        .recording_date
        .and_then(|dt| dt.into_naive())
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string());

    // send the request to the Youtube API
    let response = match state
        .http_client
        .post(url)
        .header("Content-Type", "application/json")
        .header(
            "Authorization",
            format!("Bearer {}", access_token.expose_secret()),
        )
        .header("X-Upload-Content-Length", content_length.to_string())
        .header("X-Upload-Content-Type", &body.mime_type)
        .json(&json!({
            "snippet": {
                "title": body.title.clone(),
                "description": body.description.clone(),
                "tags": body.tags.clone(),
                "categoryId": body.category,
                "defaultLanguage": "en-US",
                "defaultAudioLanguage": "en-US",
            },
            "status": {
                "privacyStatus": "private",
                "embeddable": true,
                "selfDeclaredMadeForKids": false,
                "license": "creativeCommon"
            },
            "recordingDetails": {
                "recordingDate": recording_date_iso8601
            }
        }))
        .send()
        .await
    {
        // if the request was successful, return the response
        Ok(response) => response,
        // if the request failed due to a network error, return an error response
        Err(e) => {
            tracing::error!("failed to send request to Youtube API: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": "failed to send request to Youtube API" })),
            )
                .into_response();
        }
    };

    // if the response is not successful, return an error response
    if !response.status().is_success() {
        tracing::error!("response status: {:?}", response.status());

        // find if there was a quota error
        let error_body = match response.json::<serde_json::Value>().await {
            Ok(error_body) => error_body,
            Err(_) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(json!({ "error": "failed to upload video" })),
                )
                    .into_response();
            }
        };

        if error_body["error"]["errors"][0]["reason"] == "quotaExceeded" {
            // return a 503 Service Unavailable response and include a Retry-After header
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                [
                    (
                        header::RETRY_AFTER,
                        YOUTUBE_API_QUOTA_RETRY_AFTER.to_string(),
                    ),
                    (header::CONTENT_TYPE, "application/json".to_string()),
                ],
                axum::Json(json!({ "error": "quota exceeded" })),
            )
                .into_response();
        }

        tracing::error!("response: {:?}", error_body);

        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(json!({ "error": "failed to upload video" })),
        )
            .into_response();
    }

    // get the upload URL from the Location header
    let upload_url = match response
        .headers()
        .get("Location")
        .and_then(|v| v.to_str().ok())
    {
        Some(upload_url) => upload_url.to_string(),
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": "upload URL not found" })),
            )
                .into_response();
        }
    };

    tracing::info!("upload_url: {}", upload_url);

    // upload the video to Youtube
    let response = match upload(
        &state.http_client,
        &path,
        content_length,
        &upload_url,
        &body.mime_type.clone(),
        &access_token,
    )
    .await
    {
        Ok(response) => response,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": e })),
            )
                .into_response();
        }
    };
    // try to parse the video ID from the response JSON
    let video_id = match response.json::<serde_json::Value>().await {
        Ok(contents) => match contents["id"].clone() {
            serde_json::Value::String(video_id) => video_id,
            _ => {
                tracing::error!(
                    "video ID not found in response from Youtube API"
                );
                return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        axum::Json(json!({ "error": "video ID not found in response from Youtube API" })),
                    )
                        .into_response();
            }
        },
        Err(e) => {
            tracing::error!(
                "failed to parse response from Youtube API: {:?}",
                e
            );
            return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(json!({ "error": "failed to parse response from Youtube API" })),
                )
                    .into_response();
        }
    };

    (
        StatusCode::OK,
        axum::Json(json!(UploadVideoTaskOutput {
            cursor: None,
            summary: json!([{
                "video_id": video_id,
            }]),
        })),
    )
        .into_response()
}

#[instrument]
pub async fn add_to_playlist_task_handler(
    State(state): State<AppState>,
    AccessToken(access_token): AccessToken,
    Json(body): Json<YoutubePlaylistAddTaskPayload>,
) -> impl IntoResponse {
    let video_id = match body.previous_task_data.first() {
        Some(response) => response.video_id.clone(),
        None => {
            tracing::error!("video ID not found");
            return (
                StatusCode::BAD_REQUEST,
                axum::Json(json!({ "error": "video ID not found" })),
            )
                .into_response();
        }
    };

    let result = add_video_to_playlist(
        &state,
        &access_token,
        &body.playlist_id,
        &video_id,
        body.playlist_position,
    )
    .await;
    if let Err(e) = result {
        return e;
    }

    (
        StatusCode::OK,
        axum::Json(json!(UploadVideoTaskOutput {
            cursor: None,
            summary: json!([{
                "video_id": video_id,
            }]),
        })),
    )
        .into_response()
}

/**
 * Extractor for the access token from Redis.
 *
 * This is a simple extractor that gets the access token from Redis
 * and injects it into the request's extensions.
 */
pub struct AccessToken(Secret<String>);

#[async_trait]
impl FromRequestParts<AppState> for AccessToken {
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let mut con = match state
            .redis_client
            .get_multiplexed_async_connection()
            .await
        {
            Ok(con) => con,
            Err(_) => {
                tracing::error!("failed to get redis connection");

                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "internal server error" })),
                ));
            }
        };

        let access_token: Result<String, _> =
            redis::AsyncCommands::get(&mut con, ACCESS_TOKEN_KEY).await;

        match access_token {
            Ok(access_token) => Ok(AccessToken(Secret::new(access_token))),
            Err(_) => {
                let access_token = match refresh_access_token(state).await {
                    Ok(access_token) => access_token,
                    Err(_) => {
                        tracing::error!("failed to update refresh token");

                        return Err((
                            StatusCode::UNAUTHORIZED,
                            Json(
                                json!({ "error": "need to login to YouTube" }),
                            ),
                        ));
                    }
                };

                Ok(AccessToken(access_token))
            }
        }
    }
}

fn get_google_oauth_client(
    config: &config::Config,
) -> Result<BasicClient, oauth2::url::ParseError> {
    Ok(BasicClient::new(
        ClientId::new(config.youtube_client_id.clone()),
        Some(ClientSecret::new(
            config.youtube_client_secret.expose_secret().to_string(),
        )),
        AuthUrl::new(config.youtube_auth_uri.clone())?,
        Some(TokenUrl::new(config.youtube_token_uri.clone())?),
    )
    .set_redirect_uri(RedirectUrl::new(config.youtube_redirect_url.clone())?))
}

#[derive(Serialize, Debug)]
struct AuthRequest {
    authorize_url: Url,
    csrf_state: CsrfToken,
    pkce_code_verifier: String,
}

#[instrument]
pub async fn get_login_handler(
    State(state): State<AppState>,
) -> impl IntoResponse {
    // using oauth2 crate
    let client = match get_google_oauth_client(&state.config) {
        Ok(client) => client,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR,).into_response(),
    };

    let (pkce_code_challenge, pkce_code_verifier) =
        PkceCodeChallenge::new_random_sha256();

    let (authorize_url, csrf_state) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/youtube.upload".to_string(),
        ))
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/youtube.readonly".to_string(),
        ))
        .add_scope(Scope::new(
            "https://www.googleapis.com/auth/youtube".to_string(),
        ))
        .set_pkce_challenge(pkce_code_challenge)
        .url();

    let response = AuthRequest {
        authorize_url,
        csrf_state,
        pkce_code_verifier: pkce_code_verifier.secret().to_string(),
    };

    (StatusCode::OK, Json(json!(response))).into_response()
}

#[derive(Deserialize, Debug)]
pub struct AuthCode {
    code: AuthorizationCode,
    state: CsrfToken,
    csrf_state: CsrfToken,
    pkce_code_verifier: PkceCodeVerifier,
}

/**
 * POST /login
 *
 * Completes the OAuth flow by exchanging the code for an access token
 * and refresh token. Stores the tokens in Redis and returns a
 * 202 Accepted response.
 */
#[instrument]
pub async fn post_login_handler(
    State(state): State<AppState>,
    Json(body): Json<AuthCode>,
) -> impl IntoResponse {
    let oauth2_client = match get_google_oauth_client(&state.config) {
        Ok(client) => client,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR,).into_response(),
    };

    // check the CSRF state
    if body.state.secret() != body.csrf_state.secret() {
        return (StatusCode::UNAUTHORIZED,).into_response();
    }

    // exchange the code for an access token and refresh token
    let token_response = match oauth2_client
        .exchange_code(body.code)
        .set_pkce_verifier(body.pkce_code_verifier)
        .request_async(oauth2::reqwest::async_http_client)
        .await
    {
        Ok(token_response) => token_response,
        Err(_) => {
            return (StatusCode::UNAUTHORIZED,).into_response();
        }
    };

    let access_token = token_response.access_token().secret().to_string();
    let refresh_token = token_response
        .refresh_token()
        .map(|t| t.secret().to_string());

    let mut con =
        match state.redis_client.get_multiplexed_async_connection().await {
            Ok(con) => con,
            Err(e) => {
                tracing::error!(
                    "failed to get redis connection for login: {:?}",
                    e
                );

                return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
            }
        };

    if let Some(refresh_token) = refresh_token {
        match redis::AsyncCommands::set(
            &mut con,
            REFRESH_TOKEN_KEY,
            refresh_token,
        )
        .await
        {
            Ok(()) => (),
            Err(e) => {
                tracing::error!(
                    "failed to set refresh token in Redis: {:?}",
                    e
                );

                return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
            }
        };
    }

    let access_token_ttl = calculate_access_token_ttl(token_response);

    let set_options = redis::SetOptions::default()
        .with_expiration(redis::SetExpiry::EX(access_token_ttl.as_secs()));

    match redis::AsyncCommands::set_options(
        &mut con,
        ACCESS_TOKEN_KEY,
        access_token,
        set_options,
    )
    .await
    {
        Ok(()) => (),
        Err(e) => {
            tracing::error!("failed to set access token in Redis: {:?}", e);

            return (StatusCode::INTERNAL_SERVER_ERROR,).into_response();
        }
    };

    (StatusCode::ACCEPTED,).into_response()
}

/**
 * Get new access token with the refresh token.
 *
 * This function is called when the access token is not found in Redis.
 * It uses the refresh token to get a new access token and refresh token
 * from the Youtube API.
 */
async fn refresh_access_token(state: &AppState) -> Result<Secret<String>, ()> {
    let oauth2_client = match get_google_oauth_client(&state.config) {
        Ok(client) => client,
        Err(_) => return Err(()),
    };

    let mut con =
        match state.redis_client.get_multiplexed_async_connection().await {
            Ok(con) => con,
            Err(_) => {
                tracing::error!("failed to get redis connection");

                return Err(());
            }
        };

    let refresh_token: String =
        match redis::AsyncCommands::get(&mut con, REFRESH_TOKEN_KEY).await {
            Ok(refresh_token) => refresh_token,
            Err(_) => {
                tracing::error!("failed to get refresh token from Redis");

                return Err(());
            }
        };

    let token_response = match oauth2_client
        .exchange_refresh_token(&RefreshToken::new(refresh_token.to_string()))
        .request_async(oauth2::reqwest::async_http_client)
        .await
    {
        Ok(token_response) => token_response,
        Err(_) => {
            tracing::error!("failed to refresh access token");

            return Err(());
        }
    };

    let access_token = token_response.access_token().secret().to_string();
    let access_token_ttl = calculate_access_token_ttl(token_response);

    let set_options = redis::SetOptions::default()
        .with_expiration(redis::SetExpiry::EX(access_token_ttl.as_secs()));

    match redis::AsyncCommands::set_options(
        &mut con,
        ACCESS_TOKEN_KEY,
        access_token.clone(),
        set_options,
    )
    .await
    {
        Ok(()) => (),
        Err(_) => {
            tracing::error!("failed to set access token in Redis");

            return Err(());
        }
    };

    Ok(Secret::new(access_token))
}

fn calculate_access_token_ttl<EF, TT>(
    token_response: oauth2::StandardTokenResponse<EF, TT>,
) -> std::time::Duration
where
    EF: oauth2::ExtraTokenFields,
    TT: oauth2::TokenType,
{
    (match token_response.expires_in() {
        Some(duration) => duration,
        None => {
            tracing::debug!("access token duration not found in google oauth response, using 1 hour");
            std::time::Duration::from_secs(3600)
        }
    } - std::time::Duration::from_secs(5))
}

enum UploadInnerStatus {
    Success(reqwest::Response),
    TemporaryFailure,
    PermanentFailure,
}

#[instrument]
async fn upload_inner(
    http_client: &reqwest::Client,
    path: &str,
    start_byte: u64,
    file_size: u64,
    upload_url: &str,
    content_type: &str,
    access_token: &Secret<String>,
) -> UploadInnerStatus {
    // get an async file handle
    let mut file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(e) => {
            tracing::error!("failed to open file: {:?}", e);
            return UploadInnerStatus::PermanentFailure;
        }
    };

    let mut request = http_client
        .put(upload_url)
        .header("Content-Type", content_type)
        .header(
            "Authorization",
            format!("Bearer {}", access_token.expose_secret()),
        );

    if start_byte > 0 {
        request = request.header(
            "Content-Range",
            format!("bytes {}-{}/{}", start_byte, file_size - 1, file_size),
        );

        file.seek(std::io::SeekFrom::Start(start_byte))
            .await
            .expect("failed to seek to start of file");
    }

    // upload the contents of the video using the upload URL and
    // chunked upload
    let response = match request.body(file).send().await {
        Ok(response) => response,
        Err(e) => {
            tracing::error!("failed to send request to Youtube API: {:?}", e);
            return UploadInnerStatus::PermanentFailure;
        }
    };

    if response.status().is_success() {
        UploadInnerStatus::Success(response)
    } else {
        let status_code = response.status().as_u16();
        tracing::error!(
            "Youtube API error response: {:?} {:?}",
            response.status(),
            response.text().await
        );
        if [500, 502, 503, 504].contains(&status_code) {
            UploadInnerStatus::TemporaryFailure
        } else {
            UploadInnerStatus::PermanentFailure
        }
    }
}

enum UploadStatus {
    Success(reqwest::Response),
    TemporaryFailure { start_byte: u64, wait_time_ms: u64 },
    PermanentFailure,
}

const MAX_ATTEMPTS: u8 = 10;
const BASE_WAIT_TIME: u64 = 1000;

#[instrument]
async fn get_upload_status(
    http_client: &reqwest::Client,
    file_size: u64,
    upload_url: &str,
    access_token: &Secret<String>,
    attempts: u8,
) -> UploadStatus {
    if attempts >= MAX_ATTEMPTS {
        return UploadStatus::PermanentFailure;
    }

    // get the upload status from the Youtube API
    let response = match http_client
        .put(upload_url)
        .header(
            "Authorization",
            format!("Bearer {}", access_token.expose_secret()),
        )
        .header("Content-Range", format!("bytes */{}", file_size))
        .send()
        .await
    {
        Ok(response) => response,
        Err(e) => {
            tracing::error!("failed to send request to Youtube API: {:?}", e);

            return UploadStatus::PermanentFailure;
        }
    };

    if response.status().is_success() {
        UploadStatus::Success(response)
    } else if response.status().as_u16() == 308 {
        let range = match response
            .headers()
            .get("Range")
            .and_then(|v| v.to_str().ok())
        {
            Some(range) => range,
            None => {
                tracing::error!("Range header not found in response");
                return UploadStatus::PermanentFailure;
            }
        };

        // parse the range header which looks like "bytes=0-12345"
        let range_parts = range.split(&['=', '-'][..]).collect::<Vec<&str>>();

        let start_byte = match range_parts.get(1) {
            Some(start_byte) => start_byte,
            None => {
                tracing::error!("start byte not found in range header");
                return UploadStatus::PermanentFailure;
            }
        };

        let start_byte = match start_byte.parse::<u64>() {
            Ok(start_byte) => start_byte,
            Err(e) => {
                tracing::error!("failed to parse start byte: {:?}", e);
                return UploadStatus::PermanentFailure;
            }
        };

        let wait_time_ms = match response
            .headers()
            .get("Retry-After")
            .and_then(|v| v.to_str().ok())
        {
            Some(wait_time) => {
                match wait_time.parse::<u64>() {
                    Ok(wait_time) => wait_time * 1000,
                    Err(_) => {
                        // Retry-After is a date
                        match chrono::DateTime::parse_from_rfc2822(wait_time) {
                            Ok(wait_time) => {
                                wait_time.timestamp_millis() as u64
                            }
                            Err(e) => {
                                tracing::error!(
                                    "failed to parse Retry-After header: {:?}",
                                    e
                                );
                                BASE_WAIT_TIME * 2u64.pow(attempts as u32)
                            }
                        }
                    }
                }
            }
            None => BASE_WAIT_TIME * 2u64.pow(attempts as u32),
        };

        UploadStatus::TemporaryFailure {
            start_byte,
            wait_time_ms,
        }
    } else {
        UploadStatus::PermanentFailure
    }
}

#[instrument]
async fn upload(
    http_client: &reqwest::Client,
    path: &str,
    file_size: u64,
    upload_url: &str,
    content_type: &str,
    access_token: &Secret<String>,
) -> Result<reqwest::Response, String> {
    let mut attempt = 0;
    let mut start_byte = 0;

    loop {
        match upload_inner(
            http_client,
            path,
            start_byte,
            file_size,
            upload_url,
            content_type,
            access_token,
        )
        .await
        {
            UploadInnerStatus::Success(response) => return Ok(response),
            UploadInnerStatus::TemporaryFailure => {
                match get_upload_status(
                    http_client,
                    file_size,
                    upload_url,
                    access_token,
                    attempt,
                )
                .await
                {
                    UploadStatus::Success(response) => return Ok(response),
                    UploadStatus::TemporaryFailure {
                        start_byte: new_start_byte,
                        wait_time_ms,
                    } => {
                        start_byte = new_start_byte;
                        attempt += 1;

                        tokio::time::sleep(std::time::Duration::from_millis(
                            wait_time_ms,
                        ))
                        .await;
                    }
                    UploadStatus::PermanentFailure => {
                        tracing::error!("failed to get upload status");
                        return Err("failed to get upload status".to_string());
                    }
                }
            }
            UploadInnerStatus::PermanentFailure => {
                tracing::error!("failed to upload video");
                return Err("failed to upload video".to_string());
            }
        }
    }
}

async fn add_video_to_playlist(
    state: &AppState,
    access_token: &Secret<String>,
    playlist_id: &str,
    video_id: &str,
    playlist_position: Option<u32>,
) -> Result<(), axum::http::Response<axum::body::Body>> {
    let response = match state
        .http_client
        .post(
            "https://www.googleapis.com/youtube/v3/playlistItems?part=snippet",
        )
        .header("Content-Type", "application/json")
        .header(
            "Authorization",
            format!("Bearer {}", access_token.expose_secret()),
        )
        .json(&json!({
            "snippet": {
                "playlistId": playlist_id,
                "position": playlist_position.unwrap_or(1) - 1,
                "resourceId": {
                    "kind": "youtube#video",
                    "videoId": video_id
                }
            }
        }))
        .send()
        .await
    {
        Ok(response) => response,
        Err(e) => {
            tracing::error!("failed to send request to Youtube API: {:?}", e);
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": "failed to send request to Youtube API" })),
            )
                .into_response());
        }
    };

    if !response.status().is_success() {
        tracing::error!(
            "response: {:?} {:?}",
            response.status(),
            response.text().await
        );

        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(json!({ "error": "failed to add video to playlist" })),
        )
            .into_response());
    }

    Ok(())
}
