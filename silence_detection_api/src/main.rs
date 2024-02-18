use axum::{
    extract::State,
    http::{header, StatusCode},
    response::IntoResponse,
    routing::post,
    Json,
};
use common_api_lib::{self, media::get_video_duration};
use dotenvy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::process::Command;
use tracing::{debug, instrument};

#[derive(Clone, Debug)]
struct AppState {
    video_storage_path: String,
    noise: f64,
    duration: f64,

    task_api_url: String,
    task_api_external_url: String,

    this_api_base_url: String,

    http_client: reqwest::Client,
}

#[derive(Deserialize, Debug)]
struct Task {
    id: String,
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

        task_api_url: dotenvy::var("TASK_API_URL").expect("TASK_API_URL must be set"),

        task_api_external_url: dotenvy::var("TASK_API_EXTERNAL_URL")
            .expect("TASK_API_EXTERNAL_URL must be set"),

        this_api_base_url: dotenvy::var("THIS_API_BASE_URL")
            .expect("THIS_API_BASE_URL must be set"),

        http_client: reqwest::Client::new(),
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
    start_offset: std::time::Duration,
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
        None => return (StatusCode::BAD_REQUEST, "invalid uri").into_response(),
    };

    let path = format!("{}/{}", state.video_storage_path, filename);

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
            start: std::time::Duration::from_secs_f64(start) + cursor.start_offset,
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
struct DetectInput {
    uris: Vec<String>,
    track: u8,
}

#[instrument]
async fn detect(State(state): State<AppState>, Json(body): Json<DetectInput>) -> impl IntoResponse {
    let uris = body.uris.clone();

    let track = body.track;

    let http_client = state.http_client.clone();

    let response = match http_client
        .post(&state.task_api_url)
        .json(&json!({
            "url": format!("{}/detect/segment", state.this_api_base_url),
            "payload": json!({
                "uris": uris,
                "track": track,
            }),
            "data_key": "segments",
        }))
        .send()
        .await
    {
        Ok(response) => response,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };

    debug!("task api response: {:?}", response);

    // if the task api returns an error, then return an error
    if !response.status().is_success() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(json!({ "error": "task api error" })),
        )
            .into_response();
    }

    // log the body of the response
    let response_body = match response.json::<Task>().await {
        Ok(response_body) => response_body,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };

    (
        StatusCode::ACCEPTED,
        [(
            header::LOCATION,
            format!("{}/{}", state.task_api_external_url, response_body.id),
        )],
    )
        .into_response()
}
