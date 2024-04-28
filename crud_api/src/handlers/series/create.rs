use axum::extract::Json;
use axum::response::IntoResponse;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::json;
use tracing;
use tracing::instrument;

use common_api_lib::db::DbConnection;

use super::structs::{CreateSeriesRequest, SeriesDetailView};
use crate::models::Series;

#[instrument]
pub async fn handler(
    DbConnection(mut db): DbConnection<'_>,
    Json(body): Json<CreateSeriesRequest>,
) -> impl IntoResponse {
    use crate::schema::series::dsl::*;

    tracing::info!("create_series");

    let record = match diesel::insert_into(series)
        .values((
            title.eq(body.title),
            description.eq(body.description.unwrap_or("".to_string())),
            thumbnail_url.eq(body.thumbnail_url.unwrap_or("".to_string())),
        ))
        .get_result::<Series>(&mut db.connection)
        .await
    {
        Ok(record) => record,
        Err(e) => {
            tracing::error!("Error inserting record: {}", e);
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    axum::Json(json!(SeriesDetailView::from(record))).into_response()
}
