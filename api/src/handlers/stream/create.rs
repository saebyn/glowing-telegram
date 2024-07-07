use axum::extract::Json;
use axum::response::IntoResponse;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::json;
use tracing;
use tracing::instrument;

use crate::db::DbConnection;

use super::structs::{CreateStreamRequest, StreamDetailView};
use crate::models::Stream;

#[instrument]
pub async fn handler(
    DbConnection(mut db): DbConnection<'_>,
    Json(body): Json<CreateStreamRequest>,
) -> impl IntoResponse {
    use crate::schema::streams::dsl::*;

    tracing::info!("create_stream");

    let record = match diesel::insert_into(streams)
        .values((
            title.eq(body.title),
            description.eq(body.description.unwrap_or("".to_string())),
            thumbnail_url.eq(body.thumbnail.unwrap_or("".to_string())),
            prefix.eq(body.prefix),
            speech_audio_url.eq(body.speech_audio_track.unwrap_or("".to_string())),
            stream_id.eq(body.stream_id),
            stream_platform.eq(body.stream_platform),
            duration.eq(crate::handlers::utils::parse_duration(
                body.duration.clone(),
            )),
            stream_date.eq(match body.stream_date {
                Some(date) => match date.parse() {
                    Ok(date) => date,
                    Err(e) => {
                        tracing::error!("Error parsing date: {}", e);
                        chrono::Utc::now()
                    }
                },
                // default to now
                None => chrono::Utc::now(),
            }),
        ))
        .get_result::<Stream>(&mut db.connection)
        .await
    {
        Ok(record) => record,
        Err(e) => {
            tracing::error!("Error inserting record: {}", e);
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    // TODO: add topic_ids

    axum::Json(json!(StreamDetailView::from((record, vec![])))).into_response()
}
