use axum::extract::Json;
use axum::response::IntoResponse;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::json;
use tracing;
use tracing::instrument;

use crate::db::DbConnection;

use super::structs::{CreateVideoClipRequest, VideoClipDetailView};
use crate::{handlers::utils::parse_duration, models::VideoClip};

#[instrument]
pub async fn handler(
    DbConnection(mut db): DbConnection<'_>,
    Json(body): Json<CreateVideoClipRequest>,
) -> impl IntoResponse {
    use crate::schema::video_clips::dsl::*;

    tracing::info!("create_video_clip");

    let duration_value = parse_duration(body.duration);

    let start_time_value = parse_duration(body.start_time);

    let record = match diesel::insert_into(video_clips)
        .values((
            title.eq(body.title),
            description.eq(body.description.unwrap_or("".to_string())),
            uri.eq(body.uri.unwrap_or("".to_string())),
            duration.eq(duration_value),
            start_time.eq(start_time_value),
            stream_id.eq(body
                .stream_id
                .map(|other_id| uuid::Uuid::parse_str(&other_id).unwrap_or(uuid::Uuid::nil()))),
        ))
        .get_result::<VideoClip>(&mut db.connection)
        .await
    {
        Ok(record) => record,
        Err(e) => {
            tracing::error!("Error inserting record: {}", e);
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    axum::Json(json!(VideoClipDetailView::from(record))).into_response()
}
