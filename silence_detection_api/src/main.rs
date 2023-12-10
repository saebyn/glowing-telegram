use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    routing::post,
    Json,
};
use common_api_lib;
use dotenvy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::process::Command;
use tracing::instrument;

#[derive(Clone, Debug)]
struct AppState {
    video_storage_path: String,
    noise: f64,
    duration: f64,
}

#[tokio::main]
async fn main() -> Result<(), axum::BoxError> {
    let state = AppState {
        video_storage_path: dotenvy::var("VIDEO_STORAGE_PATH")
            .expect("VIDEO_STORAGE_PATH must be set"),

        noise: dotenvy::var("NOISE")
            .expect("NOISE must be set")
            .parse::<f64>()
            .expect("NOISE must be a float"),

        duration: dotenvy::var("DURATION")
            .expect("DURATION must be set")
            .parse::<f64>()
            .expect("DURATION must be a float"),
    };

    common_api_lib::run(state, |app| {
        app.route("/detect/segment", post(detect_segment))
            .route("/detect", post(detect))
    })
    .await
}

#[derive(Deserialize, Serialize, Debug)]
struct Cursor {
    index: usize,
}

#[derive(Deserialize, Debug)]
struct DetectSegmentInput {
    uris: Vec<String>,
    track: u8,
    cursor: Option<Cursor>,

    noise: Option<f64>,
    duration: Option<f64>,
}

#[derive(Serialize, Debug)]
struct DetectSegmentOutput {
    cursor: Option<Cursor>,
    segments: Vec<Segment>,
}

#[derive(Serialize, Debug)]
struct Segment {
    #[serde(deserialize_with = "common_api_lib::serde::deserialize_duration")]
    #[serde(serialize_with = "common_api_lib::serde::serialize_duration")]
    start: std::time::Duration,
    #[serde(deserialize_with = "common_api_lib::serde::deserialize_duration")]
    #[serde(serialize_with = "common_api_lib::serde::serialize_duration")]
    end: std::time::Duration,
}

#[instrument]
async fn detect_segment(
    State(state): State<AppState>,
    Json(body): Json<DetectSegmentInput>,
) -> impl IntoResponse {
    let noise = body.noise.unwrap_or(state.noise);
    let duration = body.duration.unwrap_or(state.duration);

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
        None => Cursor { index: 0 },
    };

    // if cursor is out of bounds, return an error
    if cursor.index >= body.uris.len() {
        return (StatusCode::BAD_REQUEST, "invalid cursor").into_response();
    }

    let uri = body.uris[cursor.index].clone();
    // extract filename from uri
    let filename = match uri.split('/').last() {
        Some(filename) => filename,
        None => return (StatusCode::BAD_REQUEST, "invalid uri").into_response(),
    };

    let path = format!("{}/{}", state.video_storage_path, filename);

    let command_output = match Command::new("ffmpeg")
        .arg("-hide_banner")
        .arg("-i")
        .arg(path)
        .arg("-map")
        .arg(format!("0:a:{}", track))
        .arg("-af")
        .arg(format!(
            "silencedetect=noise={}:duration={}",
            noise, duration
        ))
        .arg("-f")
        .arg("null")
        .arg("-")
        .output()
        .await
    {
        Ok(output) => output,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "ffmpeg error").into_response(),
    };

    let command_stderr = String::from_utf8_lossy(&command_output.stderr);

    // handle output status code
    if !command_output.status.success() {
        tracing::error!("ffmpeg error: {}", command_stderr);
        return (StatusCode::INTERNAL_SERVER_ERROR, "ffmpeg error").into_response();
    }

    // trace output
    tracing::trace!("ffmpeg output: {}", command_stderr);

    // detect error in filter by looking for "Conversion failed!" in output
    if command_stderr.contains("Conversion failed!") {
        return (StatusCode::INTERNAL_SERVER_ERROR, "ffmpeg error").into_response();
    }

    let re = match Regex::new(
        r"silence_end: (?<end>\d+(\.\d+)?) \| silence_duration: (?<duration>\d+(\.\d+)?)",
    ) {
        Ok(re) => re,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "regex error").into_response(),
    };

    let mut segments = Vec::new();

    for cap in re.captures_iter(&command_stderr) {
        let end = cap["end"].parse::<f64>().unwrap();
        let duration = cap["duration"].parse::<f64>().unwrap();

        let start = end - duration;

        segments.push(Segment {
            start: std::time::Duration::from_secs_f64(start),
            end: std::time::Duration::from_secs_f64(end),
        });
    }

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
        }),
        segments,
    };

    (StatusCode::OK, axum::Json(json!(output))).into_response()
}

#[derive(Deserialize, Debug)]
struct DetectInput {
    uris: Vec<String>,
    track: u8,
}

#[instrument]
async fn detect(Json(_body): Json<DetectInput>) -> impl IntoResponse {
    // TODO: implement

    (StatusCode::ACCEPTED, [(header::LOCATION, "test")]).into_response()
}
