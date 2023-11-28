use axum::extract::{Query, State};
use axum::http::header;
use axum::response::IntoResponse;
use diesel::expression::expression_types::NotSelectable;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::json;
use tracing;
use tracing::instrument;

use crate::handlers::stream::structs::StreamView;
use crate::models::Stream;
use crate::schema;
use crate::state::AppState;

use super::structs::Params;

#[instrument]
pub async fn handler(
    State(state): State<AppState>,
    Query(params): Query<Params>,
) -> impl IntoResponse {
    use schema::streams::dsl::*;

    tracing::info!("get_streams_list");

    let Params { range, sort, order } = params;

    let (offset, limit) = match range {
        Some((start, stop)) => (start, stop - start + 1),
        None => (0, 10),
    };

    let mut connection = state.pool.get().await.unwrap();

    let total: i64 = streams.count().get_result(&mut connection).await.unwrap();

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

    let results: Vec<Stream> = streams
        .limit(limit)
        .offset(offset)
        .order_by(order)
        .select(streams::all_columns())
        .load(&mut connection)
        .await
        .unwrap();

    let prepared_results = results
        .iter()
        .map(|record| StreamView {
            id: record.id.to_string(),
            title: record.title.to_string(),
            created_at: record.created_at.to_string(),
            updated_at: record.updated_at.map(|dt| dt.to_string()),
            thumbnail: record.thumbnail_url.to_string(),
            topic_ids: vec![],
        })
        .collect::<Vec<StreamView>>();

    let pagination_info = format!(
        "streams {start}-{stop}/{total}",
        start = offset,
        stop = offset + limit,
        total = total
    );

    (
        [(header::CONTENT_RANGE, pagination_info)],
        axum::Json(json!(prepared_results)),
    )
}
