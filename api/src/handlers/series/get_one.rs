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

use super::structs::SeriesDetailView;
use crate::models::Series;
use crate::schema;

#[instrument]
pub async fn handler(
    DbConnection(mut db): DbConnection<'_>,
    Path(record_id): Path<Uuid>,
) -> impl IntoResponse {
    use schema::series::dsl::*;

    tracing::info!("get_series");

    let result: Result<Series, _> = series
        .filter(id.eq(record_id))
        .select(series::all_columns())
        .first(&mut db.connection)
        .await;

    match result {
        Ok(result) => {
            let series_view = SeriesDetailView::from(result);

            (
                [(header::CONTENT_TYPE, "application/json")],
                axum::Json(json!(series_view)),
            )
                .into_response()
        }
        Err(_) => (StatusCode::NOT_FOUND).into_response(),
    }
}
