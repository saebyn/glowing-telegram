use axum::extract::Query;
use axum::http::header;
use axum::response::IntoResponse;
use diesel::expression::expression_types::NotSelectable;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::json;
use tracing;
use tracing::instrument;

use common_api_lib::db::DbConnection;

use super::structs::ListParams;
use super::structs::StreamSimpleView;
use crate::models::Stream;
use crate::schema;

#[instrument]
pub async fn handler(
    DbConnection(mut db): DbConnection<'_>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    use schema::streams::dsl::*;

    tracing::info!("get_streams_list");

    let ListParams { range, sort, order } = params;

    let (offset, limit) = match range {
        Some((start, stop)) => (start, stop - start + 1),
        None => (0, 10),
    };

    let total: i64 = match streams.count().get_result(&mut db.connection).await {
        Ok(total) => total,
        Err(e) => {
            tracing::error!("Error getting total count: {}", e);
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    let order: Box<dyn BoxableExpression<streams, diesel::pg::Pg, SqlType = NotSelectable>> =
        match (sort, order) {
            (Some(sort), Some(order)) => match (sort.as_str(), order.as_str()) {
                ("id", "ASC") => Box::new(id.asc()),
                ("id", "DESC") => Box::new(id.desc()),
                ("title", "ASC") => Box::new(title.asc()),
                ("title", "DESC") => Box::new(title.desc()),
                ("created_at", "ASC") => Box::new(created_at.asc()),
                ("created_at", "DESC") => Box::new(created_at.desc()),
                ("updated_at", "ASC") => Box::new(updated_at.asc()),
                ("updated_at", "DESC") => Box::new(updated_at.desc()),
                _ => Box::new(id.asc()),
            },
            _ => Box::new(id.asc()),
        };

    let results: Vec<Stream> = match streams
        .limit(limit)
        .offset(offset)
        .order_by(order)
        .select(streams::all_columns())
        .load(&mut db.connection)
        .await
    {
        Ok(results) => results,
        Err(e) => {
            tracing::error!("Error getting results: {}", e);
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    let prepared_results = results
        .into_iter()
        .map(|record| StreamSimpleView::from(record))
        .collect::<Vec<StreamSimpleView>>();

    let pagination_info = format!(
        "streams {start}-{stop}/{total}",
        start = offset,
        stop = offset + limit,
        total = total
    );

    (
        [
            (header::CONTENT_RANGE, pagination_info),
            (header::CONTENT_TYPE, "application/json".to_string()),
        ],
        axum::Json(json!(prepared_results)),
    )
        .into_response()
}
