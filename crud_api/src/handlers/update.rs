use axum::response::IntoResponse;
use serde_json::json;
use tracing;
use tracing::instrument;

#[instrument]
pub async fn handler() -> impl IntoResponse {
    tracing::info!("update");

    axum::Json(json!({}))
}
