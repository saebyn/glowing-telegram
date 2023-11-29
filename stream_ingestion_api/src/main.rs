use common_api_lib;
use dotenvy;

#[derive(Clone)]
struct AppState {
    video_storage_path: String,
}

#[tokio::main]
async fn main() -> Result<(), axum::BoxError> {
    let state = AppState {
        video_storage_path: dotenvy::var("VIDEO_STORAGE_PATH")
            .expect("VIDEO_STORAGE_PATH must be set"),
    };

    common_api_lib::run(state, |app| {
        // TODO: Add routes here
        app
    })
    .await
}
