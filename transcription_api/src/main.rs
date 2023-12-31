use axum::{
    extract::State,
    http::{header, status, StatusCode},
    response::IntoResponse,
    routing::post,
    Json,
};
use common_api_lib;
use dotenvy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::process::Stdio;
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
    index: usize,
}

#[derive(Deserialize, Debug)]
struct DetectSegmentInput {
    uris: Vec<String>,
    track: u8,
    cursor: Option<Cursor>,
}

#[instrument]
async fn detect_segment(Json(_body): Json<DetectSegmentInput>) -> impl IntoResponse {
    let mut audio_extraction = match Command::new("ffmpeg")
        .arg("-hide_banner")
        .arg("-i")
        .arg("/obs/2023-12-31 09-06-10.mkv")
        .arg("-map")
        .arg("0:a:2")
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

    let mut whisper_detection = match Command::new("whisper")
        .arg("--model")
        .arg("tiny")
        .arg("--model_dir")
        .arg("/model/")
        .arg("--output_format")
        .arg("json")
        // TODO make a temp dir
        .arg("--output_dir")
        .arg("/obs/transcriptions/")
        .arg("--task")
        .arg("transcribe")
        .arg("--device")
        .arg("cuda")
        .arg("--language")
        .arg("en")
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

    let transcription_path = "/obs/transcriptions/-.json";

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

    axum::Json(json!({ "transcription": transcription })).into_response()
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
