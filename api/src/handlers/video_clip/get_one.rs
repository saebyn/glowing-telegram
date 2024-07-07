use axum::extract::Path;
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::json;
use tracing;
use tracing::instrument;
use uuid::Uuid;

use crate::db::DbConnection;

use super::structs::VideoClipDetailView;
use crate::models::VideoClip;
use crate::schema;

#[instrument]
pub async fn handler(
    DbConnection(mut db): DbConnection<'_>,
    Path(record_id): Path<Uuid>,
) -> impl IntoResponse {
    use schema::video_clips::dsl::*;

    tracing::info!("get_video_clip");

    let result: Result<VideoClip, _> = video_clips
        .filter(id.eq(record_id))
        .select(video_clips::all_columns())
        .first(&mut db.connection)
        .await;

    match result {
        Ok(result) => {
            let video_clip_view = VideoClipDetailView::from(result);

            (
                [(header::CONTENT_TYPE, "application/json")],
                axum::Json(json!(video_clip_view)),
            )
                .into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND).into_response(),
    }
}
