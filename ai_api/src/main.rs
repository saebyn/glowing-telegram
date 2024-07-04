use axum::routing::post;

mod handlers;
mod state;

#[tokio::main]
async fn main() -> Result<(), axum::BoxError> {
    // get path to openai key from env var
    let openai_key_path = std::env::var("OPENAI_KEY_PATH").expect("OPENAI_KEY_PATH not set");

    let openai_model = std::env::var("OPENAI_MODEL").unwrap_or_else(|_| "gpt-4o".to_string());

    let state = state::AppState::new(openai_key_path, openai_model);

    common_api_lib::run(state, |app| {
        app.route("/api/chat", post(handlers::complete_chat::handler))
    })
    .await
}
