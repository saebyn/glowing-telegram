use axum::extract::{FromRequestParts, State};

use axum::http::request::Parts;
use axum::routing::post;
use axum::{async_trait, Json};
use axum::{
    http::{header, StatusCode},
    response::IntoResponse,
    routing::get,
};
use common_api_lib::{self, task};
use dotenvy;
use redact::Secret;
use redis;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::io::AsyncSeekExt;
use tracing::instrument;
use url::Url;

// Redis key constants
const ACCESS_TOKEN_KEY: &str = "youtube:access_token";
const REFRESH_TOKEN_KEY: &str = "youtube:refresh_token";

// The state of the application.
#[derive(Clone, Debug)]
struct AppState {
    redis: redis::Client,

    render_storage_path: String,

    youtube_auth_uri: String,
    youtube_token_uri: String,
    youtube_client_id: String,
    youtube_client_secret: Secret<String>,

    redirect_url: String,

    task_api_url: String,
    task_api_external_url: String,

    this_api_base_url: String,

    http_client: reqwest::Client,
}

#[tokio::main]
async fn main() -> Result<(), axum::BoxError> {
    let auth_uri = dotenvy::var("YOUTUBE_AUTH_URI").expect("YOUTUBE_AUTH_URI not set");
    let token_uri = dotenvy::var("YOUTUBE_TOKEN_URI").expect("YOUTUBE_TOKEN_URI not set");
    let youtube_client_id = dotenvy::var("YOUTUBE_CLIENT_ID").expect("YOUTUBE_CLIENT_ID not set");
    let youtube_client_secret_path =
        dotenvy::var("YOUTUBE_CLIENT_SECRET_PATH").expect("YOUTUBE_CLIENT_SECRET_PATH not set");

    let state = AppState {
        redis: redis::Client::open(dotenvy::var("REDIS_URL").expect("REDIS_URL must be set"))?,

        render_storage_path: dotenvy::var("RENDER_STORAGE_PATH")
            .expect("RENDER_STORAGE_PATH must be set"),

        redirect_url: dotenvy::var("REDIRECT_URL").expect("REDIRECT_URL must be set"),

        http_client: reqwest::Client::builder()
            .user_agent("saebyn-youtube-api/0.1")
            .connection_verbose(false)
            .build()
            .expect("failed to create http client"),

        task_api_url: dotenvy::var("TASK_API_URL").expect("TASK_API_URL must be set"),

        task_api_external_url: dotenvy::var("TASK_API_EXTERNAL_URL")
            .expect("TASK_API_EXTERNAL_URL must be set"),

        this_api_base_url: dotenvy::var("THIS_API_BASE_URL")
            .expect("THIS_API_BASE_URL must be set"),

        youtube_auth_uri: auth_uri,
        youtube_token_uri: token_uri,
        youtube_client_id,
        youtube_client_secret: Secret::new(
            std::fs::read_to_string(youtube_client_secret_path)
                .expect("failed to read youtube secret from YOUTUBE_CLIENT_SECRET_PATH")
                .trim()
                .to_string(),
        ),
    };

    common_api_lib::run(state, |app| {
        app.route("/login", get(get_login_handler).post(post_login_handler))
            .route("/upload", post(upload_start_task_handler))
            .route("/upload/task", post(upload_video_handler))
    })
    .await
}

#[derive(Serialize, Debug, Deserialize)]
struct YoutubeUploadRequest {
    title: String,
    description: String,
    tags: Vec<String>,
    category: u8,
    render_uri: String,
    thumbnail_uri: Option<String>,
    recording_date: Option<iso8601::DateTime>,
    playlist_id: Option<String>,
    playlist_position: Option<u32>,
    notify_subscribers: bool,

    task_title: String,
}

#[derive(Serialize, Debug, Deserialize)]
struct YoutubeUploadTaskPayload {
    title: String,
    description: String,
    tags: Vec<String>,
    category: u8,
    render_uri: String,
    thumbnail_uri: Option<String>,
    recording_date: Option<iso8601::DateTime>,
    playlist_id: Option<String>,
    playlist_position: Option<u32>,
    notify_subscribers: bool,
}

#[instrument]
async fn upload_start_task_handler(
    State(state): State<AppState>,
    AccessToken(access_token): AccessToken,
    Json(body): Json<YoutubeUploadRequest>,
) -> impl IntoResponse {
    let task_url = match task::start(
        task::Context {
            http_client: state.http_client.clone(),
            task_api_url: state.task_api_url.clone(),
            task_api_external_url: state.task_api_external_url.clone(),
        },
        task::TaskRequest {
            url: format!("{}/upload/task", state.this_api_base_url),
            title: body.task_title,
            payload: json!(YoutubeUploadTaskPayload {
                title: body.title,
                description: body.description,
                tags: body.tags,
                category: body.category,
                render_uri: body.render_uri,
                thumbnail_uri: body.thumbnail_uri,
                recording_date: body.recording_date,
                playlist_id: body.playlist_id,
                playlist_position: body.playlist_position,
                notify_subscribers: body.notify_subscribers,
            }),
            data_key: "summary".to_string(),
        },
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
    summary: Vec<String>,
}

#[instrument]
async fn upload_video_handler(
    State(state): State<AppState>,
    AccessToken(access_token): AccessToken,
    Json(body): Json<YoutubeUploadTaskPayload>,
) -> impl IntoResponse {
    // extract filename from uri
    let uri = body.render_uri;
    let filename = match uri.split(&['/', ':'][..]).last() {
        Some(filename) => filename,
        None => return (StatusCode::BAD_REQUEST, "invalid uri").into_response(),
    };

    let path = format!("{}/{}", state.render_storage_path, filename);

    // TODO: get the content type
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

    let content_type = "video/mp4";

    // get the upload URL
    let mut url = Url::parse("https://www.googleapis.com/upload/youtube/v3/videos")
        .expect("failed to parse URL");

    url.query_pairs_mut()
        .append_pair("uploadType", "resumable")
        .append_pair("part", "snippet,status,contentDetails")
        .append_pair("notifySubscribers", &body.notify_subscribers.to_string())
        .finish();

    let recording_date_iso8601 = body
        .recording_date
        .map(|dt| dt.into_naive())
        .flatten()
        .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string());

    let response = match state
        .http_client
        .post(url)
        .header("Content-Type", "application/json")
        .header(
            "Authorization",
            format!("Bearer {}", access_token.expose_secret()),
        )
        .header("X-Upload-Content-Length", content_length.to_string())
        .header("X-Upload-Content-Type", content_type)
        .json(&json!({
            "snippet": {
                "title": body.title,
                "description": body.description,
                "tags": body.tags,
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
            "contentDetails": {
                "recordingDate": recording_date_iso8601
            }
        }))
        .send()
        .await
    {
        Ok(response) => response,
        Err(e) => {
            tracing::error!("failed to send request to Youtube API: {:?}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": "failed to send request to Youtube API" })),
            )
                .into_response();
        }
    };

    if !response.status().is_success() {
        tracing::error!("response: {:?}", response);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(json!({ "error": "failed to upload video" })),
        )
            .into_response();
    }

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

    // if the body contains a playlist_id, add the video to the playlist
    if let Some(playlist_id) = body.playlist_id {
        // try to parse the video ID from the response JSON
        let video_id = match response.json::<serde_json::Value>().await {
            Ok(contents) => {
                match contents["id"].clone() {
                    serde_json::Value::String(video_id) => video_id,
                    _ => {
                        tracing::error!("video ID not found in response from Youtube API");
                        return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        axum::Json(json!({ "error": "video ID not found in response from Youtube API" })),
                    )
                        .into_response();
                    }
                }
            }
            Err(e) => {
                tracing::error!("failed to parse response from Youtube API: {:?}", e);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(json!({ "error": "failed to parse response from Youtube API" })),
                )
                    .into_response();
            }
        };

        let response = match state
            .http_client
            .post("https://www.googleapis.com/youtube/v3/playlistItems?part=snippet")
            .header("Content-Type", "application/json")
            .header(
                "Authorization",
                format!("Bearer {}", access_token.expose_secret()),
            )
            .json(&json!({
                "snippet": {
                    "playlistId": playlist_id,
                    "position": body.playlist_position.unwrap_or(0),
                    "resourceId": {
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
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    axum::Json(json!({ "error": "failed to send request to Youtube API" })),
                )
                    .into_response();
            }
        };

        if !response.status().is_success() {
            tracing::error!("response: {:?}", response);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": "failed to add video to playlist" })),
            )
                .into_response();
        }
    }

    match upload(
        &state.http_client,
        &path,
        content_length,
        &upload_url,
        content_type,
        &access_token,
    )
    .await
    {
        Ok(_) => (
            StatusCode::OK,
            axum::Json(json!(UploadVideoTaskOutput {
                cursor: None,
                summary: vec!["video uploaded".to_string()],
            })),
        )
            .into_response(),
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": e })),
            )
                .into_response();
        }
    }
}

#[derive(Debug)]
struct AuthTokens {
    access_token: Secret<String>,
    refresh_token: Secret<String>,
}

/**
 * Extractor for the access token from Redis.
 *
 * This is a simple extractor that gets the access token from Redis
 * and injects it into the request's extensions.
 */
struct AccessToken(Secret<String>);

#[async_trait]
impl FromRequestParts<AppState> for AccessToken {
    type Rejection = (StatusCode, Json<serde_json::Value>);

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let mut con = match state.redis.get_multiplexed_async_connection().await {
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
                let tokens = match update_refresh_token(state).await {
                    Ok(tokens) => tokens,
                    Err(_) => {
                        tracing::error!("failed to update refresh token");

                        return Err((
                            StatusCode::UNAUTHORIZED,
                            Json(json!({ "error": "need to login to YouTube" })),
                        ));
                    }
                };

                Ok(AccessToken(tokens.access_token))
            }
        }
    }
}

#[instrument]
async fn get_login_handler(State(state): State<AppState>) -> impl IntoResponse {
    let mut url = Url::parse(&state.youtube_auth_uri).expect("failed to parse URL");
    url.query_pairs_mut()
        .append_pair("client_id", &state.youtube_client_id)
        .append_pair("redirect_uri", &state.redirect_url)
        .append_pair("response_type", "code")
        .append_pair("access_type", "offline")
        .append_pair("incude_granted_scopes", "true")
        .append_pair("scope", "https://www.googleapis.com/auth/youtube.upload https://www.googleapis.com/auth/youtube.readonly")
        .finish();

    let url: String = url.into();

    (StatusCode::OK, Json(json!({ "url": url })))
}

/**
 * POST /login
 *
 * Completes the OAuth flow by exchanging the code for an access token
 * and refresh token. Stores the tokens in Redis and returns a
 * 202 Accepted response.
 */
#[instrument]
async fn post_login_handler(
    State(state): State<AppState>,
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let code = body["code"].as_str().expect("code not found in body");

    let AuthTokens {
        access_token,
        refresh_token,
    } = match get_token(&state, code).await {
        Ok(tokens) => tokens,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR,);
        }
    };

    let mut con = state
        .redis
        .get_multiplexed_async_connection()
        .await
        .expect("failed to get redis connection");

    let _: () =
        redis::AsyncCommands::set(&mut con, REFRESH_TOKEN_KEY, refresh_token.expose_secret())
            .await
            .expect("failed to set refresh token");

    let _: () = redis::AsyncCommands::set(&mut con, ACCESS_TOKEN_KEY, access_token.expose_secret())
        .await
        .expect("failed to set access token");

    (StatusCode::ACCEPTED,)
}

/**
 * Updates the refresh token in Redis.
 *
 * This function is called when the access token is not found in Redis.
 * It uses the refresh token to get a new access token and refresh token
 * from the Youtube API.
 */
async fn update_refresh_token(state: &AppState) -> Result<AuthTokens, ()> {
    let mut con = match state.redis.get_multiplexed_async_connection().await {
        Ok(con) => con,
        Err(_) => {
            tracing::error!("failed to get redis connection");

            return Err(());
        }
    };

    let refresh_token: Result<String, _> =
        redis::AsyncCommands::get(&mut con, REFRESH_TOKEN_KEY).await;

    let refresh_token = match refresh_token {
        Ok(refresh_token) => refresh_token.to_string(),
        Err(_) => {
            tracing::error!("failed to get refresh token from Redis");

            return Err(());
        }
    };

    get_refresh_token(state, &refresh_token).await
}

/**
 * Gets the access token and refresh token from the Youtube API.
 *
 * This function is called when the access token is not found in Redis.
 * It uses the refresh token to get a new access token and refresh token
 * from the Youtube API.
 */
async fn get_token(state: &AppState, code: &str) -> Result<AuthTokens, ()> {
    let url = state.youtube_token_uri.clone();

    let body = json!({
        "client_id": state.youtube_client_id,
        "client_secret": state.youtube_client_secret.expose_secret(),
        "code": code,
        "grant_type": "authorization_code",
        "redirect_uri": state.redirect_url,
    });

    let response = match state.http_client.post(url).json(&body).send().await {
        Ok(response) => response,
        Err(_) => {
            tracing::error!("failed to send request to Youtube API");

            return Err(());
        }
    };

    let response = match response.json::<serde_json::Value>().await {
        Ok(response) => response,
        Err(_) => {
            tracing::error!("failed to parse response from Youtube API");

            return Err(());
        }
    };

    Ok(AuthTokens {
        access_token: Secret::new(
            response["access_token"]
                .as_str()
                .expect("access_token not found")
                .to_string(),
        ),
        refresh_token: Secret::new(
            response["refresh_token"]
                .as_str()
                .expect("refresh_token not found")
                .to_string(),
        ),
    })
}

/**
 * Gets the access token and refresh token from the Youtube API.
 *
 * This function is called when the access token is not found in Redis.
 * It uses the refresh token to get a new access token and refresh token
 * from the Youtube API.
 */
async fn get_refresh_token(state: &AppState, refresh_token: &str) -> Result<AuthTokens, ()> {
    let url = "https://id.youtube.tv/oauth2/token";

    let body = json!({
        "client_id": state.youtube_client_id,
        "client_secret": state.youtube_client_secret.expose_secret(),
        "refresh_token": refresh_token,
        "grant_type": "refresh_token",
    });

    let response = match state.http_client.post(url).json(&body).send().await {
        Ok(response) => response,
        Err(_) => {
            tracing::error!("failed to send request to Youtube API");

            return Err(());
        }
    };

    let response = match response.json::<serde_json::Value>().await {
        Ok(response) => response,
        Err(_) => {
            tracing::error!("failed to parse response from Youtube API");

            return Err(());
        }
    };

    Ok(AuthTokens {
        access_token: Secret::new(
            response["access_token"]
                .as_str()
                .expect("access_token not found")
                .to_string(),
        ),
        refresh_token: Secret::new(
            response["refresh_token"]
                .as_str()
                .expect("refresh_token not found")
                .to_string(),
        ),
    })
}

enum UploadInnerStatus {
    Success,
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
        UploadInnerStatus::Success
    } else if vec![500, 502, 503, 504].contains(&response.status().as_u16()) {
        UploadInnerStatus::TemporaryFailure
    } else {
        UploadInnerStatus::PermanentFailure
    }
}

enum UploadStatus {
    Success,
    TemporaryFailure { start_byte: u64, wait_time_ms: u64 },
    PermanentFailure,
}

const MAX_ATTEMPTS: u8 = 10;
const BASE_WAIT_TIME: u64 = 1000;

#[instrument]
async fn upload_status(
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
        UploadStatus::Success
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
                            Ok(wait_time) => wait_time.timestamp_millis() as u64,
                            Err(e) => {
                                tracing::error!("failed to parse Retry-After header: {:?}", e);
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
) -> Result<(), String> {
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
            UploadInnerStatus::Success => break,
            UploadInnerStatus::TemporaryFailure => {
                match upload_status(http_client, file_size, upload_url, access_token, attempt).await
                {
                    UploadStatus::Success => break,
                    UploadStatus::TemporaryFailure {
                        start_byte: new_start_byte,
                        wait_time_ms,
                    } => {
                        start_byte = new_start_byte;
                        attempt += 1;

                        tokio::time::sleep(std::time::Duration::from_millis(wait_time_ms)).await;
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

    Ok(())
}
