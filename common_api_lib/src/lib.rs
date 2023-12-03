use axum::http::header::{HeaderName, AUTHORIZATION};
use axum::http::HeaderValue;
use axum::response::IntoResponse;
use axum::{routing::get, Router};
use serde_json::json;
use std::iter::once;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::sensitive_headers::SetSensitiveRequestHeadersLayer;
use tower_http::trace::TraceLayer;
use tower_http::{compression::CompressionLayer, propagate_header::PropagateHeaderLayer};
use tracing;
use tracing::instrument;
use tracing_subscriber::prelude::*;

pub mod db;
pub mod serde;

pub async fn run<State>(
    state: State,
    add_routes: impl FnOnce(Router<State>) -> Router<State>,
) -> Result<(), axum::BoxError>
where
    State: Clone + Send + Sync + 'static,
{
    init_tracer();

    // build our application with a route
    let app = app(state, add_routes).await;

    let host: std::net::IpAddr = dotenvy::var("HOST")
        .expect("HOST not set")
        .parse()
        .expect("HOST is not a valid IP address");

    let port = dotenvy::var("PORT")
        .expect("PORT not set")
        .parse()
        .expect("PORT is not a valid port number");

    let addr = SocketAddr::from((host, port));
    tracing::debug!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    Ok(())
}

fn init_tracer() {
    let fmt_layer = tracing_subscriber::fmt::layer();

    tracing_subscriber::registry()
        .with(fmt_layer)
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

async fn app<State>(state: State, add_routes: impl FnOnce(Router<State>) -> Router<State>) -> Router
where
    State: Clone + Send + Sync + 'static,
{
    let origins = dotenvy::var("CORS_ALLOWED_ORIGINS")
        .expect("CORS_ALLOWED_ORIGINS not set")
        .split(',')
        .map(|s| s.to_string())
        .map(|s| s.parse::<HeaderValue>().unwrap())
        .collect::<Vec<_>>();

    // build our application with a route
    add_routes(Router::<State>::new())
        .route("/health", get(health))
        .with_state(state)
        .layer(CorsLayer::new().allow_origin(origins))
        // Mark the `Authorization` request header as sensitive so it doesn't show in logs
        .layer(SetSensitiveRequestHeadersLayer::new(once(AUTHORIZATION)))
        // High level logging of requests and responses
        .layer(TraceLayer::new_for_http())
        // Compress responses
        .layer(CompressionLayer::new())
        // Propagate `X-Request-Id`s from requests to responses
        .layer(PropagateHeaderLayer::new(HeaderName::from_static(
            "x-request-id",
        )))
}

#[instrument]
async fn health() -> impl IntoResponse {
    tracing::info!("health check");

    axum::Json(json!({ "status" : "UP" }))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::warn!("signal received, starting graceful shutdown");
}

#[cfg(test)]
mod tests {
    use tokio::runtime::Runtime;

    use super::*;

    #[test]
    fn test_health() {
        let rt = Runtime::new().unwrap();
        let response = rt.block_on(health()).into_response();
        assert_eq!(response.status(), 200);
    }
}
