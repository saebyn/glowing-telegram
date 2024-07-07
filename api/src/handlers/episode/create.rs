use axum::extract::Json;
use axum::response::IntoResponse;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::json;
use tracing;
use tracing::instrument;

use crate::db::DbConnection;

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
            thumbnail_url.eq::<Option<String>>(body.thumbnail_url.clone()),
            stream_id.eq(body.stream_id),
            tracks.eq(json!(body.tracks)),
            series_id.eq(body.series_id),
            order_index.eq(body.order_index.unwrap_or(0)),
            notify_subscribers.eq(body.notify_subscribers.unwrap_or(false)),
            category.eq(body.category.unwrap_or(20)),
            tags.eq(body.tags.unwrap_or(vec![])),
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
