use axum::extract::{Path, State};
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::json;
use tracing;
use tracing::instrument;
use uuid::Uuid;

use super::structs::StreamDetailView;
use crate::models::Stream;
use crate::schema;
use crate::state::AppState;

#[instrument]
pub async fn handler(
    State(state): State<AppState>,
    Path(record_id): Path<Uuid>,
) -> impl IntoResponse {
    use schema::streams::dsl::*;

    tracing::info!("get_stream");

    let mut connection = match state.pool.get().await {
        Ok(conn) => conn,
        Err(e) => {
            tracing::error!("Error getting connection from pool: {}", e);
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    let result: Result<Stream, _> = streams
        .filter(id.eq(record_id))
        .select(streams::all_columns())
        .first(&mut connection)
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
