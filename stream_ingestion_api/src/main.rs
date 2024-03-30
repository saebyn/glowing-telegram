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
    rendered_episode_storage_path: String,
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

        rendered_episode_storage_path: dotenvy::var("RENDERED_EPISODE_STORAGE_PATH")
            .expect("RENDERED_EPISODE_STORAGE_PATH must be set"),
    };

    common_api_lib::run(state, |app| {
        app.route("/find_files", get(find_files)).route(
            "/find_rendered_episode_files",
            get(find_rendered_episode_files),
        )
    })
    .await
}

async fn find_files(
    State(state): State<AppState>,
    Query(query): Query<FindFilesQuery>,
) -> impl IntoResponse {
    tracing::info!("find_files: {:?}", query);

    let mut entries = get_entries(&state.video_storage_path, &query).await;

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

async fn find_rendered_episode_files(State(state): State<AppState>) -> impl IntoResponse {
    let mut entries = get_entries(
        &state.rendered_episode_storage_path,
        &FindFilesQuery {
            prefix: "".to_string(),
        },
    )
    .await;

    // sort by filename, ascending
    entries.sort_by(|a, b| a.metadata.filename.cmp(&b.metadata.filename));

    axum::Json(json!(FindFilesResponse { entries: entries })).into_response()
}

async fn get_entries(base_path: &str, query: &FindFilesQuery) -> Vec<Entry> {
    let mut dir_entries = match tokio::fs::read_dir(&base_path).await {
        Ok(dir_entries) => dir_entries,
        Err(_) => {
            tracing::error!("find_files: failed to read_dir: {:?}", base_path);
            return Vec::new();
        }
    };

    let mut entries = Vec::new();

    while let Ok(Some(entry)) = dir_entries.next_entry().await {
        let path = entry.path();

        if path.is_file() {
            let file_name = path.file_name().unwrap().to_str().unwrap();

            if file_name.starts_with(&query.prefix) {
                let file = file_name.to_string();

                tracing::debug!("find_files: file: {:?}", file);

                let path = format!("{}/{}", &base_path, file);

                let metadata = match tokio::fs::metadata(&path).await {
                    Ok(metadata) => metadata,
                    Err(_) => continue,
                };

                tracing::debug!("find_files: metadata: {:?}", metadata);

                let (
                    width,
                    height,
                    duration,
                    frame_rate,
                    video_bitrate,
                    audio_bitrate,
                    audio_track_count,
                ) = match ffprobe::probe(&path).await {
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
                    duration: chrono::Duration::try_milliseconds(
                        duration.map_or(0, |duration| (duration * 1000.0) as i64),
                    )
                    .unwrap_or(chrono::Duration::zero()),
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

                entries.push(Entry { metadata, uri });
            }
        }
    }

    entries
}
