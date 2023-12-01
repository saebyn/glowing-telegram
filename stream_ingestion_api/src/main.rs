use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
};
use common_api_lib;
use dotenvy;
use serde::{Deserialize, Serialize};
use serde_json::json;

mod ffprobe;

#[derive(Clone)]
struct AppState {
    video_storage_path: String,
}

#[derive(Deserialize, Debug)]
struct FindFilesQuery {
    prefix: String,
}

pub fn iso8601_chrono_serde<S>(
    duration: &chrono::Duration,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&format!("PT{}S", duration.num_seconds()))
}

#[derive(Serialize)]
struct Metadata {
    filename: String,
    content_type: String,
    size: u64,
    last_modified: chrono::DateTime<chrono::Utc>,
    #[serde(serialize_with = "iso8601_chrono_serde")]
    duration: chrono::Duration,
    #[serde(serialize_with = "iso8601_chrono_serde")]
    start_time: chrono::Duration,
    width: Option<u32>,
    height: Option<u32>,
    frame_rate: Option<f32>,
    video_bitrate: Option<u32>,
    audio_bitrate: Option<u32>,
    audio_track_count: Option<u32>,
}

#[derive(Serialize)]
struct Entry {
    metadata: Metadata,
    uri: String,
}

#[derive(Serialize)]
struct FindFilesResponse {
    entries: Vec<Entry>,
}

#[tokio::main]
async fn main() -> Result<(), axum::BoxError> {
    let state = AppState {
        video_storage_path: dotenvy::var("VIDEO_STORAGE_PATH")
            .expect("VIDEO_STORAGE_PATH must be set"),
    };

    common_api_lib::run(state, |app| app.route("/find_files", get(find_files))).await
}

async fn find_files(
    State(state): State<AppState>,
    Query(query): Query<FindFilesQuery>,
) -> impl IntoResponse {
    tracing::info!("find_files: {:?}", query);

    let mut entries = match tokio::fs::read_dir(&state.video_storage_path).await {
        Ok(entries) => entries,
        Err(_) => return axum::http::StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let mut files = Vec::new();

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();

        if path.is_file() {
            let file_name = path.file_name().unwrap().to_str().unwrap();

            if file_name.starts_with(&query.prefix) {
                files.push(file_name.to_string());
            }
        }
    }

    tracing::debug!("find_files: files: {:?}", files);

    let mut entries = Vec::new();

    for file in files {
        tracing::debug!("find_files: file: {:?}", file);

        let path = format!("{}/{}", &state.video_storage_path, file);

        let metadata = match tokio::fs::metadata(&path).await {
            Ok(metadata) => metadata,
            Err(_) => continue,
        };

        tracing::debug!("find_files: metadata: {:?}", metadata);

        let (width, height, duration, frame_rate, video_bitrate, audio_bitrate, audio_track_count) =
            match ffprobe::probe(&path).await {
                Ok(probe) => {
                    let video_stream = probe
                        .streams
                        .iter()
                        .find(|stream| stream.codec_type == "video");

                    let audio_stream = probe
                        .streams
                        .iter()
                        .find(|stream| stream.codec_type == "audio");

                    let video_stream = match video_stream {
                        Some(video_stream) => video_stream,
                        None => continue,
                    };

                    let audio_stream = match audio_stream {
                        Some(audio_stream) => audio_stream,
                        None => continue,
                    };

                    let audio_stream_count = probe
                        .streams
                        .iter()
                        .filter(|stream| stream.codec_type == "audio")
                        .count();

                    (
                        video_stream.width,
                        video_stream.height,
                        probe.format.duration,
                        video_stream.avg_frame_rate.clone(),
                        probe.format.bit_rate,
                        audio_stream.sample_rate,
                        Some(audio_stream_count as u32),
                    )
                }
                Err(_) => (None, None, None, None, None, None, None),
            };

        let metadata = Metadata {
            filename: format!("{}", file),
            content_type: "video/mp4".to_string(),
            size: metadata.len(),
            last_modified: match metadata.modified() {
                Ok(last_modified) => last_modified.into(),
                Err(_) => continue,
            },
            start_time: chrono::Duration::zero(),
            duration: chrono::Duration::milliseconds(
                duration.map_or(0, |duration| (duration * 1000.0) as i64),
            ),
            width,
            height,
            frame_rate: frame_rate.map(|frame_rate| {
                let mut parts = frame_rate.split('/');

                let numerator = parts.next().unwrap().parse::<f32>().unwrap();
                let denominator = parts.next().unwrap().parse::<f32>().unwrap();

                numerator / denominator
            }),
            video_bitrate,
            audio_bitrate,
            audio_track_count,
        };

        let uri = format!("file:local:{}", file);

        tracing::debug!("find_files: uri: {:?}", uri);

        entries.push(Entry { metadata, uri });
    }

    // sort by filename, ascending
    entries.sort_by(|a, b| a.metadata.filename.cmp(&b.metadata.filename));

    // populate start_time by calculating the cumulative duration of all previous entries
    let mut cumulative_duration = chrono::Duration::zero();

    for entry in &mut entries {
        entry.metadata.start_time = cumulative_duration;
        cumulative_duration = cumulative_duration + entry.metadata.duration;
    }

    axum::Json(json!(FindFilesResponse { entries: entries })).into_response()
}
