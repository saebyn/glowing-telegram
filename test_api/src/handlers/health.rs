use axum::extract::State;
use axum::{http::StatusCode, response::IntoResponse};

use crate::state::AppState;

pub async fn handler(State(_state): State<AppState>) -> impl IntoResponse {
    tracing::info!("health check");
    (StatusCode::OK, "ok")
}
