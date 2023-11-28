use axum::{routing::delete, routing::get, routing::put};

mod handlers;
pub mod models;
pub mod schema;
mod state;

#[tokio::main]
async fn main() -> Result<(), axum::BoxError> {
    let state = state::AppState::new(common_api_lib::db::create_pool().await);

    common_api_lib::run(state, |app| {
        app
            // getList
            .route(
                "/records/streams",
                get(handlers::stream::get_list::handler)
                    .post(handlers::stream::create::handler),
            )
            .route(
                "/records/video_clips",
                get(handlers::video_clip::get_list::handler)
                    .post(handlers::video_clip::create::handler),
            )
            // getOne
            .route("/records/:key/:id", get(handlers::get_one::handler))
            // update
            .route("/records/:key/:id", put(handlers::update::handler))
            // delete
            .route("/records/:key/:id", delete(handlers::delete::handler))
    })
    .await
}
