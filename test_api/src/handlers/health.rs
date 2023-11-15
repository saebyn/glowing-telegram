use axum::extract::State;
use axum::response::IntoResponse;
use serde_json::json;

use crate::state::AppState;

pub async fn handler(State(_state): State<AppState>) -> impl IntoResponse {
    tracing::info!("health check");
    axum::Json(json!({ "status" : "UP" }))
}
