use axum::extract::Path;
use axum::http::{header, StatusCode};
use axum::response::IntoResponse;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::json;
use tracing;
use tracing::instrument;
use uuid::Uuid;

use crate::db::DbConnection;

use super::structs::EpisodeDetailView;
use crate::models::Episode;
use crate::schema;

#[instrument]
pub async fn handler(
    DbConnection(mut db): DbConnection<'_>,
    Path(record_id): Path<Uuid>,
) -> impl IntoResponse {
    use schema::episodes::dsl::*;

    tracing::info!("get_episode");

    let result: Result<Episode, _> = episodes
        .filter(id.eq(record_id))
        .select(episodes::all_columns())
        .first(&mut db.connection)
        .await;

    match result {
        Ok(result) => {
            let episode_view = EpisodeDetailView::from(result);

            (
                [(header::CONTENT_TYPE, "application/json")],
                axum::Json(json!(episode_view)),
            )
                .into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND).into_response(),
    }
}
