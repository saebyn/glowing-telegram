use axum::{
    Json, Router,
    body::Body,
    http::{Request, StatusCode, header},
    routing::{get, post},
};

use serde_json::json;
use structs::AppContext;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};

mod handlers;
mod structs;
mod youtube;

#[tokio::main]
async fn main() {
    let app_context = gt_app::create_app_context().await.unwrap();

    initialize_api(app_context).await;
}

async fn initialize_api(state: AppContext) {
    // Set up a trace layer
    let trace_layer = TraceLayer::new_for_http().on_request(
        |request: &Request<Body>, _: &tracing::Span| {
            tracing::info!(
                "received request: {method} {uri}",
                method = request.method(),
                uri = request.uri()
            );
        },
    );

    let compression_layer = CompressionLayer::new().gzip(true).deflate(true);

    // Create Axum app
    let app = Router::new()
        .route(
            "/auth/youtube/url",
            post(handlers::obtain_youtube_authorization_url_handler),
        )
        .route(
            "/auth/youtube/callback",
            post(handlers::youtube_callback_handler),
        )
        .route(
            "/auth/youtube/token",
            get(handlers::obtain_youtube_access_token_handler),
        )
        .fallback(|| async {
            (
                StatusCode::NOT_FOUND,
                [(header::CONTENT_TYPE, "application/json")],
                Json(json!({
                    "message": "not found",
                })),
            )
        })
        .layer(trace_layer)
        .layer(compression_layer)
        .with_state(state);

    gt_axum::run_lambda_app(app).await.unwrap();
}
