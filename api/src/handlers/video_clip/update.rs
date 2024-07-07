use axum::extract::Path;
use axum::response::IntoResponse;
use axum::Json;
use diesel::data_types::PgInterval;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::json;
use tracing;
use tracing::instrument;
use uuid::Uuid;

use crate::db::DbConnection;

use super::structs::UpdateVideoClipRequest;
use super::structs::VideoClipDetailView;
use crate::handlers::utils::parse_duration;
use crate::models::VideoClip;
use crate::schema::{self, video_clips};

#[derive(Debug, AsChangeset)]
#[diesel(table_name = video_clips)]
pub struct UpdateVideoClipChangeset {
    pub title: Option<String>,
    pub description: Option<String>,
    pub uri: Option<String>,
    pub duration: Option<PgInterval>,
    pub start_time: Option<PgInterval>,
    pub stream_id: Option<Uuid>,
}

#[instrument]
pub async fn handler(
    DbConnection(mut db): DbConnection<'_>,
    Path(record_id): Path<Uuid>,
    Json(body): Json<UpdateVideoClipRequest>,
) -> impl IntoResponse {
    use schema::video_clips::dsl::*;

    tracing::info!("update_video_clip");

    let duration_value = match body.duration {
        Some(duration_value) => Some(parse_duration(Some(duration_value))),
        None => None,
    };

    let start_time_value = match body.start_time {
        Some(start_time_value) => Some(parse_duration(Some(start_time_value))),
        None => None,
    };

    let stream_id_value = match body.stream_id {
        Some(other_id) => Some(uuid::Uuid::parse_str(&other_id).unwrap_or(uuid::Uuid::nil())),
        None => None,
    };

    let result: Result<VideoClip, diesel::result::Error> =
        diesel::update(video_clips.filter(id.eq(record_id)))
            .set(&UpdateVideoClipChangeset {
                title: body.title,
                description: body.description,
                uri: body.uri,
                duration: duration_value,
                start_time: start_time_value,
                stream_id: stream_id_value,
            })
            .get_result(&mut db.connection)
            .await;

    match result {
        Ok(result) => (
            [(axum::http::header::CONTENT_TYPE, "application/json")],
            axum::Json(json!(VideoClipDetailView::from(result))),
        )
            .into_response(),

        Err(e) => {
            tracing::error!("Error updating record: {}", e);
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    }
}
