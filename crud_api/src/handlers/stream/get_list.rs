use axum::extract::Query;

use axum::http::header;
use axum::response::IntoResponse;
use diesel::expression::expression_types::NotSelectable;
use diesel::BoolExpressionMethods;
use diesel::BoxableExpression;
use diesel::ExpressionMethods;
use diesel::PgTextExpressionMethods;
use diesel::QueryDsl;
use diesel::Table;
use diesel_async::RunQueryDsl;
use serde_json::json;
use tracing;
use uuid::Uuid;

use common_api_lib::db::DbConnection;

use super::structs::StreamSimpleView;
use crate::handlers::structs::ListParams;
use crate::models::Stream;
use crate::schema::streams::dsl::streams;

#[tracing::instrument]
pub async fn handler(
    DbConnection(mut db): DbConnection<'_>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    use crate::create_order_expression;
    use crate::schema::streams::dsl::*;

    tracing::info!("get_stream_list");

    let ListParams {
        range,
        sort,
        filter,
    } = params;

    let filter: serde_json::Value = match serde_json::from_str(&filter) {
        Ok(filter) => filter,
        Err(e) => {
            tracing::error!("Error parsing filter: {}", e);
            return (axum::http::StatusCode::BAD_REQUEST).into_response();
        }
    };

    let predicate = create_predicate(&filter);

    let total: i64 = match streams
        .filter(predicate)
        .count()
        .get_result(&mut db.connection)
        .await
    {
        Ok(total) => total,
        Err(e) => {
            tracing::error!("Error getting total count: {}", e);
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    let predicate = create_predicate(&filter);

    let order: Box<dyn BoxableExpression<streams, diesel::pg::Pg, SqlType = NotSelectable>> =
        create_order_expression!(sort, id, title, stream_date, prefix);

    let results: Vec<(Stream, i64)> = match streams
        .limit(range.count)
        .offset(range.start)
        .order_by(order)
        .select((
            streams::all_columns(),
            // count of video clips
            diesel::dsl::sql::<diesel::sql_types::BigInt>(
                "(SELECT COUNT(*) FROM video_clips WHERE video_clips.stream_id = streams.id)",
            ),
        ))
        .filter(predicate)
        .load::<(Stream, i64)>(&mut db.connection)
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
        "{} {start}-{stop}/{total}",
        stringify!($table),
        start = range.start,
        stop = range.start + range.count,
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

fn create_predicate(
    filter: &serde_json::Value,
) -> Box<dyn BoxableExpression<streams, diesel::pg::Pg, SqlType = diesel::sql_types::Bool>> {
    use crate::schema::streams::dsl::*;

    let id_filter: Box<
        dyn BoxableExpression<streams, diesel::pg::Pg, SqlType = diesel::sql_types::Bool>,
    > = match filter["id"].is_array() {
        true => {
            let ids = filter["id"]
                .as_array()
                .unwrap()
                .iter()
                .map(|oid| oid.as_str().unwrap())
                .map(|oid| Uuid::parse_str(oid).unwrap())
                .collect::<Vec<uuid::Uuid>>();

            Box::new(id.eq_any(ids))
        }
        false => Box::new(id.ne(Uuid::nil())),
    };

    let title_filter: Box<
        dyn BoxableExpression<streams, diesel::pg::Pg, SqlType = diesel::sql_types::Bool>,
    > = match filter["q"].as_str() {
        Some(q) => Box::new(title.ilike(format!("%{}%", q))),
        None => Box::new(title.ne("")),
    };

    Box::new(id_filter.and(title_filter))
}
