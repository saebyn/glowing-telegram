use axum::{
    extract::{Query, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::post,
    Json,
};
use chrono::format;
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
}

#[tokio::main]
async fn main() -> Result<(), axum::BoxError> {
    let state = AppState {
        video_storage_path: dotenvy::var("VIDEO_STORAGE_PATH")
            .expect("VIDEO_STORAGE_PATH must be set"),
    };

    common_api_lib::run(state, |app| {
        app.route("/detect/segment", post(detect_segment))
            .route("/detect", post(detect))
    })
    .await
}

#[derive(Deserialize, Serialize, Debug)]
struct Cursor {
    #[serde(deserialize_with = "common_api_lib::serde::deserialize_duration")]
    #[serde(serialize_with = "common_api_lib::serde::serialize_duration")]
    offset: std::time::Duration,
    index: usize,
}

#[derive(Deserialize, Debug)]
struct DetectSegmentInput {
    uris: Vec<String>,
    track: u8,
    cursor: Option<Cursor>,
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
    let noise = 0.0001;
    let duration = 0.5;

    let track = body.track;

    // get cursor or create a new one
    let cursor = match body.cursor {
        Some(cursor) => cursor,
        None => Cursor {
            offset: std::time::Duration::from_secs(0),
            index: 0,
        },
    };

    let uri = body.uris[cursor.index].clone();
    // extract filename from uri
    let filename = match uri.split('/').last() {
        Some(filename) => filename,
        None => return (StatusCode::BAD_REQUEST, "invalid uri").into_response(),
    };

    let path = format!("{}/{}", state.video_storage_path, filename);

    let output = match Command::new("ffmpeg")
        .arg("-hide_banner")
        .arg("-i")
        .arg(path)
        .arg("-map")
        .arg(format!("0:a:{}", track))
        .arg("-af")
        .arg(format!(
            "silencedetect=noise={},duration={}",
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

    let output = String::from_utf8_lossy(&output.stderr);

    let re = match Regex::new(
        r"silence_end: (?<end>\d+(\.\d+)?) \| silence_duration: (?<duration>\d+(\.\d+)?)",
    ) {
        Ok(re) => re,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "regex error").into_response(),
    };

    let mut segments = Vec::new();

    // TODO find length of the recording
    let length = std::time::Duration::from_secs(900);

    // TODO add the offset to the start and end of each segment

    for cap in re.captures_iter(&output) {
        let end = cap["end"].parse::<f64>().unwrap();
        let duration = cap["duration"].parse::<f64>().unwrap();

        let start = end - duration;

        segments.push(Segment {
            start: std::time::Duration::from_secs_f64(start),
            end: std::time::Duration::from_secs_f64(end),
        });
    }

    // TODO handle case where we are at the end of the list

    let output = DetectSegmentOutput {
        cursor: Some(Cursor {
            offset: cursor.offset + length,
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
    (StatusCode::ACCEPTED, [(header::LOCATION, "test")]).into_response()
}
