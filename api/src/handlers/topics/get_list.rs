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

use crate::db::DbConnection;

use super::structs::TopicSimpleView;
use crate::handlers::structs::ListParams;
use crate::models::Topic;
use crate::schema::topics::dsl::topics;

#[tracing::instrument]
pub async fn handler(
    DbConnection(mut db): DbConnection<'_>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    use crate::create_order_expression;
    use crate::schema::topics::dsl::*;

    tracing::info!("get_topic_list");

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

    let total: i64 = match topics
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

    let order: Box<dyn BoxableExpression<topics, diesel::pg::Pg, SqlType = NotSelectable>> =
        create_order_expression!(sort, id, title);

    let results: Vec<Topic> = match topics
        .limit(range.count)
        .offset(range.start)
        .order_by(order)
        .select(topics::all_columns())
        .filter(predicate)
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
        .map(TopicSimpleView::from)
        .collect::<Vec<TopicSimpleView>>();

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
) -> Box<dyn BoxableExpression<topics, diesel::pg::Pg, SqlType = diesel::sql_types::Bool>> {
    use crate::schema::topics::dsl::*;

    let id_filter: Box<
        dyn BoxableExpression<topics, diesel::pg::Pg, SqlType = diesel::sql_types::Bool>,
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
        dyn BoxableExpression<topics, diesel::pg::Pg, SqlType = diesel::sql_types::Bool>,
    > = match filter["q"].as_str() {
        Some(q) => Box::new(title.ilike(format!("%{}%", q))),
        None => Box::new(title.ne("")),
    };

    Box::new(id_filter.and(title_filter))
}
