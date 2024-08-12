use axum::extract::Json;
use axum::response::IntoResponse;
use diesel_async::RunQueryDsl;
use serde_json::json;
use tracing;
use tracing::instrument;

use crate::{
    db::DbConnection, handlers::episode::structs::CreateEpisodeInsertable,
};

use super::structs::{BulkCreateEpisodeRequest, EpisodeSimpleView};
use crate::models::Episode;

#[instrument]
pub async fn handler(
    DbConnection(mut db): DbConnection<'_>,
    Json(body): Json<BulkCreateEpisodeRequest>,
) -> impl IntoResponse {
    use crate::schema::episodes::dsl::*;

    tracing::info!("create_bulk_episode");

    let records = match diesel::insert_into(episodes)
        .values(
            body.records
                .iter()
                .map(|episode| episode.into())
                .collect::<Vec<CreateEpisodeInsertable>>(),
        )
        .get_results::<Episode>(&mut db.connection)
        .await
    {
        Ok(records) => records,
        Err(e) => {
            tracing::error!("Error inserting records: {}", e);
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                .into_response();
        }
    };

    axum::Json(json!(records
        .into_iter()
        .map(EpisodeSimpleView::from)
        .collect::<Vec<_>>()))
    .into_response()
}
