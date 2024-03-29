use axum::extract::Json;
use axum::response::IntoResponse;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::json;
use tracing;
use tracing::instrument;

use common_api_lib::db::DbConnection;

use super::structs::{BulkCreateStreamRequest, StreamSimpleView};
use crate::models::Stream;

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
                    (
                        title.eq(&stream.title),
                        description.eq(stream.description.clone().unwrap_or("".to_string())),
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
