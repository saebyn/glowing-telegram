
use axum::{routing::get, routing::post, routing::put, routing::delete};

mod handlers;
mod state;
pub mod models;
pub mod schema;

#[tokio::main]
async fn main() -> Result<(), axum::BoxError> {
    let state = state::AppState::new(
        common_api_lib::db::create_pool().await,
    );

    common_api_lib::run(state, |app| {
        app
            // getList
            .route("/records/:key", get(handlers::get_list::handler))
            // getOne
            .route("/records/:key/:id", get(handlers::get_one::handler))
            // create
            .route("/records/:key", post(handlers::create::handler))
            // update
            .route("/records/:key/:id", put(handlers::update::handler))
            // delete
            .route("/records/:key/:id", delete(handlers::delete::handler))
    })
    .await
}
