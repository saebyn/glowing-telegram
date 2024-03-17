use axum::extract::Json;
use axum::response::IntoResponse;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::json;
use tracing;
use tracing::instrument;

use common_api_lib::db::DbConnection;

use super::structs::{CreateEpisodeRequest, EpisodeDetailView};
use crate::models::Episode;

#[instrument]
pub async fn handler(
    DbConnection(mut db): DbConnection<'_>,
    Json(body): Json<CreateEpisodeRequest>,
) -> impl IntoResponse {
    use crate::schema::episodes::dsl::*;

    tracing::info!("create_episode");

    let record = match diesel::insert_into(episodes)
        .values((
            title.eq(body.title),
            description.eq(body.description.unwrap_or("".to_string())),
            thumbnail_url.eq(body.thumbnail_url.unwrap_or("".to_string())),
        ))
        .get_result::<Episode>(&mut db.connection)
        .await
    {
        Ok(record) => record,
        Err(e) => {
            tracing::error!("Error inserting record: {}", e);
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    axum::Json(json!(EpisodeDetailView::from(record))).into_response()
}
