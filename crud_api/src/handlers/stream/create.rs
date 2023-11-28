use axum::extract::{Json, State};
use axum::response::IntoResponse;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::json;
use tracing;
use tracing::instrument;

use super::structs::{CreateStreamRequest, StreamDetailView};
use crate::models::Stream;
use crate::state::AppState;

#[instrument]
pub async fn handler(
    // Body contains the data from the client
    State(state): State<AppState>,
    Json(body): Json<CreateStreamRequest>,
) -> impl IntoResponse {
    use crate::schema::streams::dsl::*;

    tracing::info!("create_stream");

    let mut connection = state.pool.get().await.unwrap();

    let record = diesel::insert_into(streams)
        .values((
            title.eq(body.title),
            description.eq(body.description.unwrap_or("".to_string())),
            thumbnail_url.eq(body.thumbnail.unwrap_or("".to_string())),
            prefix.eq(body.prefix),
            speech_audio_url.eq(body.speech_audio_track.unwrap_or("".to_string())),
        ))
        .get_result::<Stream>(&mut connection)
        .await
        .unwrap();

    // TODO: add topic_ids

    axum::Json(json!(StreamDetailView {
        id: record.id.to_string(),
        title: record.title.to_string(),
        created_at: record.created_at.to_string(),
        updated_at: record.updated_at.map(|dt| dt.to_string()),
        thumbnail: record.thumbnail_url.to_string(),
        topic_ids: vec![],
    }))
}
