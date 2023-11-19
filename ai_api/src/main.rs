use axum::routing::post;

mod db;
mod handlers;
mod models;
mod schema;
mod state;

#[tokio::main]
async fn main() -> Result<(), axum::BoxError> {
    // get path to openai key from env var
    let openai_key_path = std::env::var("OPENAI_KEY_PATH").expect("OPENAI_KEY_PATH not set");

    let state = state::AppState::new(
        std::fs::read_to_string(openai_key_path)
            .expect("failed to read openai key from OPENAI_KEY_PATH")
            .trim()
            .to_string(),
        db::create_pool().await,
    );

    common_api::run(state, |app| {
        app.route("/api/chat", post(handlers::complete_chat::handler))
    })
    .await
}
