use axum::{routing::get, Extension};

mod handlers;
pub mod models;
pub mod schema;
mod state;

#[tokio::main]
async fn main() -> Result<(), axum::BoxError> {
    let state = state::AppState::new();
    let pool = common_api_lib::db::create_pool().await;

    common_api_lib::run(state, |app| {
        // Define routes for ra-data-simple-rest
        app
            // streams resource
            .route(
                "/records/streams",
                get(handlers::stream::get_list::handler)
                    .post(handlers::stream::create::handler)
                    .put(handlers::stream::create_bulk::handler),
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
            .route(
                "/records/video_clips/:record_id",
                get(handlers::video_clip::get_one::handler)
                    .put(handlers::video_clip::update::handler)
                    .delete(handlers::video_clip::delete::handler),
            )
            // episodes resource
            .route(
                "/records/episodes",
                get(handlers::episode::get_list::handler)
                    .post(handlers::episode::create::handler)
                    .put(handlers::episode::create_bulk::handler),
            )
            .route(
                "/records/episodes/:record_id",
                get(handlers::episode::get_one::handler)
                    .put(handlers::episode::update::handler)
                    .delete(handlers::episode::delete::handler),
            )
            // topics resource
            .route(
                "/records/topics",
                get(handlers::topics::get_list::handler).post(handlers::topics::create::handler),
            )
            // series resource
            .route(
                "/records/series",
                get(handlers::series::get_list::handler).post(handlers::series::create::handler),
            )
            .route(
                "/records/series/:record_id",
                get(handlers::series::get_one::handler)
                    .put(handlers::series::update::handler)
                    .delete(handlers::series::delete::handler),
            )
            .layer(Extension(pool))
    })
    .await
}
