use axum::extract::State;
use axum::http::header;
use axum::response::IntoResponse;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde::Serialize;
use serde_json::json;
use tracing;
use tracing::instrument;

use crate::models::VideoClips;
use crate::schema;
use crate::state::AppState;

#[derive(Debug, Serialize)]
struct VideoClipsView {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: Option<String>,
}

#[instrument]
pub async fn handler(State(state): State<AppState>) -> impl IntoResponse {
    use schema::video_clips::dsl::*;

    tracing::info!("get_video_clips_list");

    let offset = 0;
    let limit = 10;

    let mut connection = state.pool.get().await.unwrap();

    let total: i64 = video_clips
        .count()
        .get_result(&mut connection)
        .await
        .unwrap();

    let results: Vec<VideoClips> = video_clips
        .limit(limit)
        .offset(offset)
        .load(&mut connection)
        .await
        .unwrap();

    let prepared_results = results
        .iter()
        .map(|record| VideoClipsView {
            id: record.id.to_string(),
            title: record.title.to_string(),
            created_at: record.created_at.to_string(),
            updated_at: record.updated_at.map(|dt| dt.to_string()),
        })
        .collect::<Vec<VideoClipsView>>();

    let pagination_info = format!(
        "video_clips {start}-{stop}/{total}",
        start = offset,
        stop = offset + limit,
        total = total
    );

    (
        [(header::CONTENT_RANGE, pagination_info)],
        axum::Json(json!(prepared_results)),
    )
}
