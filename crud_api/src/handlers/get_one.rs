use axum::response::IntoResponse;
use serde_json::json;
use tracing;
use tracing::instrument;


#[instrument]
pub async fn handler() -> impl IntoResponse {
    tracing::info!("get_one");

    axum::Json(json!(
        {
          "id": 0,
          "title": "2023-11-12",
          "description": "Description 1",
          "thumbnail": "https://upload.wikimedia.org/wikipedia/commons/b/bd/Test.svg",
          "topic_ids": [0],
          "created_at": "2023-11-12T00:00:00Z",
          "updated_at": null,
          "prefix": "2023-11-12",
          "speech_audio_track": "https://example.invalid/streams/0.mp3"
        }
      ))
}
