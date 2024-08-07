use axum::extract::Path;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tracing;
use tracing::instrument;
use uuid::Uuid;

use crate::db::DbConnection;

use crate::schema;

#[instrument]
pub async fn handler(
    DbConnection(mut db): DbConnection<'_>,
    Path(record_id): Path<Uuid>,
) -> impl IntoResponse {
    use schema::video_clips::dsl::*;

    tracing::info!("delete_video_clip");

    match diesel::delete(video_clips.filter(id.eq(record_id)))
        .execute(&mut db.connection)
        .await
    {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            tracing::error!("Error deleting record: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}
