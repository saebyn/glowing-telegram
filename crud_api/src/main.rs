use axum::{routing::delete, routing::get, routing::put};

mod handlers;
pub mod models;
pub mod schema;
mod state;

#[tokio::main]
async fn main() -> Result<(), axum::BoxError> {
    let state = state::AppState::new(common_api_lib::db::create_pool().await);

    common_api_lib::run(state, |app| {
        // Define routes for ra-data-simple-rest
        app
            // streams resource
            .route(
                "/records/streams",
                get(handlers::stream::get_list::handler).post(handlers::stream::create::handler),
            )
            .route(
                "/records/streams/:record_id",
                get(handlers::stream::get_one::handler)
                    .put(handlers::stream::update::handler)
                    .delete(handlers::stream::delete::handler),
            )
            // video_clips resource
            .route(
                "/records/video_clips",
                get(handlers::video_clip::get_list::handler)
                    .post(handlers::video_clip::create::handler),
            )
    })
    .await
}
