use axum::extract::State;
use axum::response::IntoResponse;
use serde_json::json;
use tracing;
use tracing::instrument;

use crate::state::AppState;

#[instrument(skip(_state))]
pub async fn handler(State(_state): State<AppState>) -> impl IntoResponse {
    tracing::info!("health check");

    axum::Json(json!({ "status" : "UP" }))
}
