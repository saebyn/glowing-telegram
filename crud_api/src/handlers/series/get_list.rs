use axum::extract::Query;

use axum::http::header;
use axum::response::IntoResponse;
use diesel::expression::expression_types::NotSelectable;
use diesel::BoxableExpression;
use diesel::ExpressionMethods;
use diesel::QueryDsl;
use diesel::Table;
use diesel_async::RunQueryDsl;
use serde_json::json;
use tracing;
use uuid::Uuid;

use common_api_lib::db::DbConnection;

use super::structs::SeriesSimpleView;
use crate::handlers::structs::ListParams;
use crate::models::Series;
use crate::schema::series::dsl::series;

#[tracing::instrument]
pub async fn handler(
    DbConnection(mut db): DbConnection<'_>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    use crate::create_order_expression;
    use crate::schema::series::dsl::*;

    tracing::info!("get_series_list");

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

    let total: i64 = match series
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

    let order: Box<dyn BoxableExpression<series, diesel::pg::Pg, SqlType = NotSelectable>> =
        create_order_expression!(sort, id, title);

    let results: Vec<(Series, i32)> = match series
        .limit(range.count)
        .offset(range.start)
        .order_by(order)
        .select((
            series::all_columns(),
            // largest episode order index
            diesel::dsl::sql::<diesel::sql_types::Int4>(
                "(SELECT COALESCE(MAX(order_index), 0) FROM episodes WHERE episodes.series_id = series.id)",
            ),
        ))
        .filter(predicate)
        .load::<(Series, i32)>(&mut db.connection)
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
        .map(|record| SeriesSimpleView::from(record))
        .collect::<Vec<SeriesSimpleView>>();

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
) -> Box<dyn BoxableExpression<series, diesel::pg::Pg, SqlType = diesel::sql_types::Bool>> {
    use crate::schema::series::dsl::*;

    let id_filter: Box<
        dyn BoxableExpression<series, diesel::pg::Pg, SqlType = diesel::sql_types::Bool>,
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
        false => Box::new(diesel::dsl::sql("1 = 1")),
    };

    Box::new(id_filter)
}
