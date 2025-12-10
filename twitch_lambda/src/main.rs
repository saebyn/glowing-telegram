use axum::{
    Json, Router,
    body::Body,
    http::{Request, StatusCode, header},
    routing::{get, post},
};

use lambda_http::tower;
use lambda_runtime::{LambdaEvent, service_fn};
use serde_json::{Value, json};
use structs::AppContext;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};

mod global_refresh;
mod handlers;
mod structs;
mod twitch;

#[tokio::main]
async fn main() {
    let state = gt_app::create_app_context::<AppContext, structs::Config>()
        .await
        .unwrap();

    if state.config.is_global_refresh_service {
        do_user_token_check(state).await;
    } else {
        initialize_api(state).await;
    }
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
            "/auth/twitch/url",
            post(handlers::obtain_twitch_authorization_url_handler),
        )
        .route(
            "/auth/twitch/callback",
            post(handlers::twitch_callback_handler),
        )
        .route(
            "/auth/twitch/token",
            get(handlers::obtain_twitch_access_token_handler),
        )
        .route(
            "/eventsub/chat/subscribe",
            post(handlers::subscribe_chat_handler)
                .delete(handlers::delete_chat_subscriptions_handler),
        )
        .route(
            "/eventsub/chat/status",
            get(handlers::chat_subscription_status_handler),
        )
        .route(
            "/eventsub/webhook",
            post(handlers::eventsub_webhook_handler),
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

    gt_axum::run_app(app).await.unwrap();
}

async fn do_user_token_check(state: AppContext) {
    lambda_runtime::run(service_fn(|_event: LambdaEvent<Value>| async {
        global_refresh::refresh_user_tokens(state.clone())
            .await
            .unwrap();

        Ok::<serde_json::Value, lambda_runtime::Error>(json!({}))
    }))
    .await
    .unwrap();
}
