use axum::extract::Json;
use axum::response::IntoResponse;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::json;
use tracing;
use tracing::instrument;

use crate::db::DbConnection;

use super::structs::{BulkCreateStreamRequest, StreamSimpleView};
use crate::{handlers::utils::parse_duration, models::Stream};

#[instrument]
pub async fn handler(
    DbConnection(mut db): DbConnection<'_>,
    Json(body): Json<BulkCreateStreamRequest>,
) -> impl IntoResponse {
    use crate::schema::streams::dsl::*;

    tracing::info!("create_bulk_stream");

    let records = match diesel::insert_into(streams)
        .values(
            body.records
                .iter()
                .map(|stream| {
                    let duration_value = parse_duration(stream.duration.clone());
                    let stream_date_value: chrono::DateTime<chrono::Utc> = match &stream.stream_date
                    {
                        Some(date) => match date.parse() {
                            Ok(date) => date,
                            Err(e) => {
                                tracing::error!("Error parsing date: {}", e);
                                chrono::Utc::now()
                            }
                        },
                        // default to now
                        None => chrono::Utc::now(),
                    };

                    (
                        title.eq(&stream.title),
                        description.eq(stream.description.clone().unwrap_or("".to_string())),
                        thumbnail_url.eq(stream.thumbnail.clone().unwrap_or("".to_string())),
                        speech_audio_url
                            .eq(stream.speech_audio_track.clone().unwrap_or("".to_string())),
                        prefix.eq(stream.prefix.clone()),
                        stream_id.eq(stream.stream_id.clone()),
                        stream_platform.eq(stream.stream_platform.clone()),
                        duration.eq(duration_value),
                        stream_date.eq(stream_date_value),
                    )
                })
                .collect::<Vec<_>>(),
        )
        .get_results::<Stream>(&mut db.connection)
        .await
    {
        Ok(records) => records,
        Err(e) => {
            tracing::error!("Error inserting records: {}", e);
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    axum::Json(json!(records
        .into_iter()
        .map(|record| StreamSimpleView::from(record))
        .collect::<Vec<_>>()))
    .into_response()
}
