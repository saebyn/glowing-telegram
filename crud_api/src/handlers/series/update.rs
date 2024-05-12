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

use super::structs::SeriesDetailView;
use super::structs::UpdateSeriesRequest;
use crate::models::Series;
use crate::schema::{self, series};

#[derive(Debug, AsChangeset)]
#[diesel(table_name = series)]
pub struct UpdateSeriesChangeset {
    pub title: Option<String>,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub playlist_id: Option<String>,
}

#[instrument]
pub async fn handler(
    DbConnection(mut db): DbConnection<'_>,
    Path(record_id): Path<Uuid>,
    Json(body): Json<UpdateSeriesRequest>,
) -> impl IntoResponse {
    use schema::series::dsl::*;

    tracing::info!("update_series");

    let result: Result<Series, diesel::result::Error> =
        diesel::update(series.filter(id.eq(record_id)))
            .set(&UpdateSeriesChangeset {
                title: body.title,
                description: body.description,
                thumbnail_url: body.thumbnail_url,
                playlist_id: body.playlist_id,
            })
            .get_result(&mut db.connection)
            .await;

    match result {
        Ok(result) => (
            [(axum::http::header::CONTENT_TYPE, "application/json")],
            axum::Json(json!(SeriesDetailView::from(result))),
        )
            .into_response(),

        Err(e) => {
            tracing::error!("Error updating record: {}", e);
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    }
}
