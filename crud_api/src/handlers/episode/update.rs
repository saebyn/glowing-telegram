use axum::extract::Path;
use axum::response::IntoResponse;
use axum::Json;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::json;
use tracing;
use tracing::instrument;
use uuid::Uuid;

use common_api_lib::db::DbConnection;

use super::structs::EpisodeDetailView;
use super::structs::UpdateEpisodeRequest;
use crate::models::Episode;
use crate::schema::{self, episodes};

#[derive(Debug, AsChangeset)]
#[diesel(table_name = episodes)]
pub struct UpdateEpisodeChangeset {
    pub title: Option<String>,
    pub description: Option<String>,
    pub stream_id: Option<Uuid>,
}

#[instrument]
pub async fn handler(
    DbConnection(mut db): DbConnection<'_>,
    Path(record_id): Path<Uuid>,
    Json(body): Json<UpdateEpisodeRequest>,
) -> impl IntoResponse {
    use schema::episodes::dsl::*;

    tracing::info!("update_episode");

    let result: Result<Episode, diesel::result::Error> =
        diesel::update(episodes.filter(id.eq(record_id)))
            .set(&UpdateEpisodeChangeset {
                title: body.title,
                description: body.description,
                stream_id: body.stream_id,
            })
            .get_result(&mut db.connection)
            .await;

    match result {
        Ok(result) => (
            [(axum::http::header::CONTENT_TYPE, "application/json")],
            axum::Json(json!(EpisodeDetailView::from(result))),
        )
            .into_response(),

        Err(e) => {
            tracing::error!("Error updating record: {}", e);
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    }
}
