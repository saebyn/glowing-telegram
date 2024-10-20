use crate::{media::get_video_duration, state::AppState, task};
use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use task_worker::TaskRequest;
use tracing::instrument;

#[derive(Deserialize, Serialize, Debug)]
struct Cursor {
    index: usize,
    start_offset: std::time::Duration,
}

#[derive(Deserialize, Debug)]
pub struct DetectSegmentInput {
    uris: Vec<String>,
    track: u8,
    cursor: Option<Cursor>,

    noise: Option<f64>,
    duration: Option<f64>,
}

#[derive(Serialize, Debug)]
struct DetectSegmentOutput {
    cursor: Option<Cursor>,
    segments: Vec<gt_ffmpeg::silence_detection::Segment>,
}

#[instrument]
pub async fn detect_segment(
    State(state): State<AppState>,
    Json(body): Json<DetectSegmentInput>,
) -> impl IntoResponse {
    let noise = body.noise.unwrap_or(state.config.noise);
    let duration = body.duration.unwrap_or(state.config.duration);

    // if no uris are provided, return an empty list of segments
    if body.uris.is_empty() {
        let output = DetectSegmentOutput {
            cursor: None,
            segments: Vec::new(),
        };

        return (StatusCode::OK, axum::Json(json!(output))).into_response();
    }

    let track = body.track;

    // get cursor or create a new one
    let cursor = match body.cursor {
        Some(cursor) => cursor,
        None => Cursor {
            index: 0,
            start_offset: std::time::Duration::from_secs(0),
        },
    };

    // if cursor is out of bounds, return an error
    if cursor.index >= body.uris.len() {
        return (StatusCode::BAD_REQUEST, "invalid cursor").into_response();
    }

    let uri = body.uris[cursor.index].clone();
    // extract filename from uri
    let filename = match uri.split(&['/', ':'][..]).last() {
        Some(filename) => filename,
        None => {
            return (StatusCode::BAD_REQUEST, "invalid uri").into_response()
        }
    };

    let path = format!("{}/{}", state.config.video_storage_path, filename);

    let video_duration = match get_video_duration(&path).await {
        Ok(video_duration) => video_duration,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": e.to_string() })),
            )
                .into_response();
        }
    };

    let segments = match gt_ffmpeg::silence_detection::extract(
        &path,
        track as u32,
        noise,
        duration,
    )
    .await
    {
        Ok(segments) => segments,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": e.0 })),
            )
                .into_response();
        }
    };

    // handle case where we are at the end of the list
    let length = body.uris.len();
    if cursor.index + 1 >= length {
        let output = DetectSegmentOutput {
            cursor: None,
            segments,
        };

        return (StatusCode::OK, axum::Json(json!(output))).into_response();
    }

    let output = DetectSegmentOutput {
        cursor: Some(Cursor {
            index: cursor.index + 1,
            start_offset: cursor.start_offset + video_duration,
        }),
        segments,
    };

    (StatusCode::OK, axum::Json(json!(output))).into_response()
}

#[derive(Deserialize, Debug)]
pub struct DetectInput {
    task_title: String,

    uris: Vec<String>,
    track: u8,

    noise: Option<f64>,
    duration: Option<f64>,
}

#[instrument]
pub async fn detect(
    State(state): State<AppState>,
    Json(body): Json<DetectInput>,
) -> impl IntoResponse {
    let uris = body.uris.clone();

    let track = body.track;

    let http_client = state.http_client.clone();

    let task_url = match task::start(
        task::Context {
            http_client,
            task_api_url: state.config.task_api_url.clone(),
            task_api_external_url: state.config.task_api_external_url.clone(),
        },
        TaskRequest {
            url: format!(
                "{}/silence_detection/detect/segment",
                state.config.this_api_base_url
            ),
            title: body.task_title,
            payload: json!({
                "uris": uris,
                "track": track,
                "noise": body.noise,
                "duration": body.duration,
            }),
            data_key: "segments".to_string(),

            next_task: None,

            http_method: reqwest::Method::POST,
            payload_transformer: None,
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
