use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use tracing;
use tracing::instrument;
use uuid::Uuid;

use crate::schema;
use crate::state::AppState;

#[instrument]
pub async fn handler(
    State(state): State<AppState>,
    Path(record_id): Path<Uuid>,
) -> impl IntoResponse {
    use schema::streams::dsl::*;

    tracing::info!("delete_stream");

    let mut connection = match state.pool.get().await {
        Ok(conn) => conn,
        Err(e) => {
            tracing::error!("Error getting connection from pool: {}", e);
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    match diesel::delete(streams.filter(id.eq(record_id)))
        .execute(&mut connection)
        .await
    {
        Ok(_) => StatusCode::NO_CONTENT.into_response(),
        Err(e) => {
            tracing::error!("Error deleting record: {}", e);
            (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response()
        }
    }
}
