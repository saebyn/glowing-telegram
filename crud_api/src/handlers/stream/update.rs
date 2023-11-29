use axum::extract::Path;
use axum::response::IntoResponse;
use axum::Json;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::json;
use tracing;
use tracing::instrument;
use uuid::Uuid;

use common_api_lib::db::DbConnection;

use super::structs::StreamDetailView;
use super::structs::UpdateStreamRequest;
use crate::models::Stream;
use crate::schema::{self, streams};

#[derive(Debug, AsChangeset)]
#[diesel(table_name = streams)]
pub struct UpdateStreamChangeset {
    pub title: Option<String>,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub prefix: Option<String>,
    pub speech_audio_url: Option<String>,
}

#[instrument]
pub async fn handler(
    DbConnection(mut db): DbConnection<'_>,
    Path(record_id): Path<Uuid>,
    Json(body): Json<UpdateStreamRequest>,
) -> impl IntoResponse {
    use schema::streams::dsl::*;

    tracing::info!("update_stream");

    let result: Result<Stream, diesel::result::Error> =
        diesel::update(streams.filter(id.eq(record_id)))
            .set(&UpdateStreamChangeset {
                title: body.title,
                description: body.description,
                thumbnail_url: body.thumbnail,
                prefix: body.prefix,
                speech_audio_url: body.speech_audio_track,
            })
            .get_result(&mut db.connection)
            .await;

    match result {
        Ok(result) => (
            [(axum::http::header::CONTENT_TYPE, "application/json")],
            axum::Json(json!(StreamDetailView::from(result))),
        )
            .into_response(),

        Err(e) => {
            tracing::error!("Error updating record: {}", e);
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    }
}
