use crate::{media::get_video_duration, state::AppState, task};
use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    Json,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use task_worker::TaskRequest;
use tokio::process::Command;
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
    segments: Vec<Segment>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Segment {
    #[serde(deserialize_with = "crate::serde::deserialize_duration")]
    #[serde(serialize_with = "crate::serde::serialize_duration")]
    start: std::time::Duration,
    #[serde(deserialize_with = "crate::serde::deserialize_duration")]
    #[serde(serialize_with = "crate::serde::serialize_duration")]
    end: std::time::Duration,
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
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "ffmpeg error")
                .into_response()
        }
    };

    let command_stderr = String::from_utf8_lossy(&command_output.stderr);

    // handle output status code
    if !command_output.status.success() {
        tracing::error!("ffmpeg error: {}", command_stderr);
        return (StatusCode::INTERNAL_SERVER_ERROR, "ffmpeg error")
            .into_response();
    }

    // trace output
    tracing::trace!("ffmpeg output: {}", command_stderr);

    // detect error in filter by looking for "Conversion failed!" in output
    if command_stderr.contains("Conversion failed!") {
        return (StatusCode::INTERNAL_SERVER_ERROR, "ffmpeg error")
            .into_response();
    }

    let re = match Regex::new(
        r"silence_end: (?<end>\d+(\.\d+)?) \| silence_duration: (?<duration>\d+(\.\d+)?)",
    ) {
        Ok(re) => re,
        Err(_) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, "regex error")
                .into_response()
        }
    };

    let mut segments = Vec::new();

    for cap in re.captures_iter(&command_stderr) {
        let end = cap["end"].parse::<f64>().unwrap();
        let duration = cap["duration"].parse::<f64>().unwrap();

        let start = end - duration;

        segments.push(Segment {
            start: std::time::Duration::from_secs_f64(start)
                + cursor.start_offset,
            end: std::time::Duration::from_secs_f64(end) + cursor.start_offset,
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
