use axum::http::StatusCode;
use axum::response::IntoResponse;

use tracing;
use tracing::instrument;

#[instrument]
pub async fn handler() -> impl IntoResponse {
    tracing::info!("delete");

    StatusCode::NO_CONTENT
}
