use axum::extract::Path;
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::json;
use tracing;
use tracing::instrument;
use uuid::Uuid;

use common_api_lib::db::DbConnection;

use super::structs::StreamDetailView;
use crate::models::Stream;
use crate::schema;

#[instrument]
pub async fn handler(
    DbConnection(mut db): DbConnection<'_>,
    Path(record_id): Path<Uuid>,
) -> impl IntoResponse {
    use schema::streams::dsl::*;

    tracing::info!("get_stream");

    let result: Result<Stream, _> = streams
        .filter(id.eq(record_id))
        .select(streams::all_columns())
        .first(&mut db.connection)
        .await;

    match result {
        Ok(result) => {
            let stream_view = StreamDetailView::from(result);

            (
                [(header::CONTENT_TYPE, "application/json")],
                axum::Json(json!(stream_view)),
            )
                .into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND).into_response(),
    }
}
