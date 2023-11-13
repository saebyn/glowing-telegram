use axum::http::header::{HeaderName, AUTHORIZATION};
use axum::{routing::get, routing::post, Router};
use std::iter::once;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::sensitive_headers::SetSensitiveRequestHeadersLayer;
use tower_http::trace::TraceLayer;
use tower_http::{compression::CompressionLayer, propagate_header::PropagateHeaderLayer};

mod db;
mod handlers;
mod models;
mod schema;
mod state;

#[tokio::main]
async fn main() {
    // initialize tracing
    tracing_subscriber::fmt::init();

    let state = state::AppState::new(
        std::fs::read_to_string("../openai_key.txt")
            .expect("failed to read openai_key.txt")
            .trim()
            .to_string(),
        db::create_pool().await,
    );

    state.pool().get().await.unwrap();

    // build our application with a route
    let app = Router::new()
        .route("/", get(handlers::health::handler))
        // `POST /api/chat` goes to `complete_chat`
        .route("/api/chat", post(handlers::complete_chat::handler))
        .with_state(state)
        // TODO make this only allow requests from our frontend?
        .layer(CorsLayer::permissive())
        // Mark the `Authorization` request header as sensitive so it doesn't show in logs
        .layer(SetSensitiveRequestHeadersLayer::new(once(AUTHORIZATION)))
        // High level logging of requests and responses
        .layer(TraceLayer::new_for_http())
        // Compress responses
        .layer(CompressionLayer::new())
        // Propagate `X-Request-Id`s from requests to responses
        .layer(PropagateHeaderLayer::new(HeaderName::from_static(
            "x-request-id",
        )));

    // run our app with hyper
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
