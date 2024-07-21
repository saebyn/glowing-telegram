use crate::media::get_video_duration;
use crate::structs::Segment;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use reqwest::header;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::process::Stdio;
use tokio::process::Command;
use tracing::{debug, instrument};

use crate::state::AppState;

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
    language: Option<String>,
    initial_prompt: Option<String>,
}

#[instrument]
pub async fn detect_segment(
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
        None => {
            return (StatusCode::BAD_REQUEST, "invalid uri").into_response()
        }
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

    let audio_extraction = match Command::new("ffmpeg")
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

    let audio = match audio_extraction.stdout {
        Some(stdout) => {
            let audio: Stdio = match stdout.try_into() {
                Ok(audio) => audio,
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        axum::Json(json!({ "error": e.to_string() })),
                    )
                        .into_response()
                }
            };

            audio
        }
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": "no stdout" })),
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
    let transcription_json = match std::fs::read_to_string(transcription_path)
    {
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
    let transcription =
        match serde_json::from_str::<serde_json::Value>(&transcription_json) {
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
                Some(start) => std::time::Duration::from_micros(
                    (start * 1_000_000.0) as u64,
                ),
                None => return None,
            };

            let end = match raw_segment["end"].as_f64() {
                Some(end) => std::time::Duration::from_micros(
                    (end * 1_000_000.0) as u64,
                ),
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
            if let Some(last_segment_group) = segment_groups.last_mut() {
                // if the last segment group less than 30 seconds long, then add the segment to it
                let last_segment_group_duration = last_segment_group
                    .iter()
                    .map(|segment: &Segment| segment.end - segment.start)
                    .sum::<std::time::Duration>();

                if last_segment_group_duration
                    < std::time::Duration::from_secs(30)
                {
                    last_segment_group.push(segment);
                } else {
                    // otherwise, create a new segment group and add the segment to it
                    segment_groups.push(vec![segment]);
                }
            }

            segment_groups
        })
        // [[segment1,segment2],[segment3,segment4]]
        // combine the segment groups into individual segments
        .iter()
        .map(|segment_group| {
            let start = match segment_group.first() {
                Some(segment) => segment.start,
                None => {
                    tracing::warn!(
                        "no segment in segment group: {:?}",
                        segment_group
                    );

                    std::time::Duration::from_secs(0)
                }
            };

            let end = match segment_group.last() {
                Some(segment) => segment.end,
                None => {
                    tracing::warn!(
                        "no segment in segment group: {:?}",
                        segment_group
                    );

                    std::time::Duration::from_secs(0)
                }
            };

            let text = segment_group
                .iter()
                .map(|segment| segment.text.as_str())
                .collect::<Vec<&str>>()
                .join(" ")
                .to_string();

            Ok(Segment { start, end, text })
        })
        .collect::<Result<Vec<Segment>, ()>>();

    let segments = match segments {
        Ok(segments) => segments,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({ "error": e })),
            )
                .into_response();
        }
    };

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
pub struct DetectInput {
    task_title: String,
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
pub async fn detect(
    State(state): State<AppState>,
    Json(body): Json<DetectInput>,
) -> impl IntoResponse {
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
            "url": format!("{}/transcription/detect/segment", state.this_api_base_url),
            "title": body.task_title,
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
