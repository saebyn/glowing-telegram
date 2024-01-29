use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Json};
use common_api_lib;
use common_api_lib::structs::Segment;
use dotenvy;
use reqwest::header;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, instrument};

#[derive(Clone, Debug)]
struct AppState {
    video_storage_path: String,

    task_api_url: String,
    task_api_external_url: String,

    this_api_base_url: String,

    http_client: reqwest::Client,
}

#[tokio::main]
async fn main() -> Result<(), axum::BoxError> {
    let state = AppState {
        video_storage_path: dotenvy::var("VIDEO_STORAGE_PATH")
            .expect("VIDEO_STORAGE_PATH must be set"),

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
    language: Option<String>,
    initial_prompt: Option<String>,
}

#[instrument]
async fn detect_segment(
    State(state): State<AppState>,
    Json(body): Json<DetectSegmentInput>,
) -> impl IntoResponse {
    let language = match body.language {
        Some(language) => language,
        None => "en".to_string(),
    };

    let initial_prompt = match body.initial_prompt {
        Some(initial_prompt) => initial_prompt,
        None => "".to_string(),
    };

    let cursor = match body.cursor {
        Some(cursor) => cursor,
        None => Cursor {
            index: 0,
            start_offset: std::time::Duration::from_secs(0),
        },
    };

    let uri = &body.uris[cursor.index];

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

    let mut audio_extraction = match Command::new("ffmpeg")
        .arg("-hide_banner")
        .arg("-i")
        .arg(path)
        .arg("-map")
        .arg(format!("0:a:{}", body.track))
        .arg("-acodec")
        .arg("pcm_s16le")
        .arg("-ac")
        .arg("1")
        .arg("-ar")
        .arg("16000")
        .arg("-f")
        .arg("wav")
        .arg("-")
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(process) => process,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };

    let audio: Stdio = match audio_extraction.stdout.take().unwrap().try_into() {
        Ok(audio) => audio,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };

    // make a temp dir for the transcription
    let temp_dir = match tempfile::tempdir() {
        Ok(temp_dir) => temp_dir,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };

    let mut whisper_detection = match Command::new("whisper")
        .arg("--model")
        .arg("tiny")
        .arg("--initial_prompt")
        .arg(initial_prompt)
        .arg("--model_dir")
        .arg("/model/")
        .arg("--output_format")
        .arg("json")
        .arg("--output_dir")
        .arg(temp_dir.path())
        .arg("--task")
        .arg("transcribe")
        .arg("--device")
        .arg("cuda")
        .arg("--language")
        .arg(language)
        .arg("-")
        .stdin(audio)
        .spawn()
    {
        Ok(process) => process,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };

    let whisper_status = match whisper_detection.wait().await {
        Ok(status) => status,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };

    if !whisper_status.success() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            axum::Json(json!({ "error": "whisper failed" })),
        )
            .into_response();
    }

    let transcription_path = temp_dir.path().join("-.json");

    // read the file and parse the json
    let transcription_json = match std::fs::read_to_string(transcription_path) {
        Ok(transcription) => transcription,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };

    // use a struct to parse the json
    let transcription = match serde_json::from_str::<serde_json::Value>(&transcription_json) {
        Ok(transcription) => transcription,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": e.to_string() })),
            )
                .into_response()
        }
    };

    let segments = match transcription["segments"].as_array() {
        Some(segments) => segments,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": "invalid json" })),
            )
                .into_response();
        }
    };

    // combine the segments into groups where each group is 30 seconds long
    // convert the segments to a vector of Segment structs
    let segments = segments
        .iter()
        .map(|raw_segment| {
            let start = match raw_segment["start"].as_f64() {
                Some(start) => std::time::Duration::from_micros((start * 1_000_000.0) as u64),
                None => return None,
            };

            let end = match raw_segment["end"].as_f64() {
                Some(end) => std::time::Duration::from_micros((end * 1_000_000.0) as u64),
                None => return None,
            };

            let text = match raw_segment["text"].as_str() {
                Some(text) => text,
                None => return None,
            };

            Some(Segment {
                start: start + cursor.start_offset,
                end: end + cursor.start_offset,
                text: text.to_string(),
            })
        })
        .filter_map(|segment: Option<Segment>| segment)
        .fold(vec![vec![]], |mut segment_groups, segment| {
            let last_segment_group = segment_groups.last_mut().unwrap();

            // if the last segment group less than 30 seconds long, then add the segment to it
            let last_segment_group_duration = last_segment_group
                .iter()
                .map(|segment: &Segment| segment.end - segment.start)
                .sum::<std::time::Duration>();

            if last_segment_group_duration < std::time::Duration::from_secs(30) {
                last_segment_group.push(segment);
            } else {
                // otherwise, create a new segment group and add the segment to it
                segment_groups.push(vec![segment]);
            }

            segment_groups
        })
        // [[segment1,segment2],[segment3,segment4]]
        // combine the segment groups into individual segments
        .iter()
        .map(|segment_group| {
            let start = segment_group.first().unwrap().start;

            let end = segment_group.last().unwrap().end;

            let text = segment_group
                .iter()
                .map(|segment| segment.text.as_str())
                .collect::<Vec<&str>>()
                .join(" ")
                .to_string();

            Segment { start, end, text }
        })
        .collect::<Vec<Segment>>();

    // if this was the last item in the list, then return None for the cursor
    // return the segments
    axum::Json(json!(   {
        "segments": segments,
        "cursor": if cursor.index + 1 >= body.uris.len() {
            None
        } else {
            Some(Cursor { index: cursor.index + 1, start_offset: cursor.start_offset + video_duration })
        },
    }))
    .into_response()
}

#[derive(Deserialize, Debug)]
struct DetectInput {
    uris: Vec<String>,
    track: u8,
    language: Option<String>,
    initial_prompt: Option<String>,
}

#[derive(Deserialize, Debug)]
struct Task {
    id: String,
}

#[instrument]
async fn detect(State(state): State<AppState>, Json(body): Json<DetectInput>) -> impl IntoResponse {
    // call task api to find the segments in the background asynchrnously,
    // and return a 202 Accepted response with the URL of the task

    let language = match body.language {
        Some(language) => language,
        None => "en".to_string(),
    };

    let initial_prompt = match body.initial_prompt {
        Some(initial_prompt) => initial_prompt,
        None => "".to_string(),
    };

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
                "language": language,
                "initial_prompt": initial_prompt,
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

async fn get_video_duration(path: &str) -> Result<std::time::Duration, String> {
    let output = match Command::new("ffprobe")
        .arg("-v")
        .arg("error")
        .arg("-show_entries")
        .arg("format=duration")
        .arg("-of")
        .arg("default=noprint_wrappers=1:nokey=1")
        .arg(path)
        .output()
        .await
    {
        Ok(output) => output,
        Err(e) => return Err(e.to_string()),
    };

    let output = match String::from_utf8(output.stdout) {
        Ok(output) => output,
        Err(e) => return Err(e.to_string()),
    };

    let output = match output.trim().parse::<f64>() {
        Ok(output) => output,
        Err(e) => return Err(e.to_string()),
    };

    Ok(std::time::Duration::from_secs_f64(output))
}
