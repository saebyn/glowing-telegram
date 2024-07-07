use axum::extract::Query;

use axum::http::header;
use axum::response::IntoResponse;
use diesel::expression::expression_types::NotSelectable;
use diesel::BoolExpressionMethods;
use diesel::BoxableExpression;
use diesel::ExpressionMethods;
use diesel::NullableExpressionMethods;
use diesel::PgTextExpressionMethods;
use diesel::QueryDsl;
use diesel::Table;
use diesel_async::RunQueryDsl;
use serde_json::json;
use tracing;
use uuid::Uuid;

use crate::db::DbConnection;

use super::structs::EpisodeSimpleView;
use crate::handlers::structs::ListParams;
use crate::models::Episode;
use crate::schema::episodes::dsl::episodes;
use crate::schema::streams;

#[tracing::instrument]
pub async fn handler(
    DbConnection(mut db): DbConnection<'_>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    use crate::create_order_expression;
    use crate::schema::episodes::dsl::*;

    tracing::info!("get_episode_list");

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

    let total: i64 = match episodes
        .filter(predicate)
        .count()
        .get_result(&mut db.connection)
        .await
    {
        Ok(total) => total,
        Err(e) => {
            tracing::error!("Error getting total count: {}", e);
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                .into_response();
        }
    };

    let predicate = create_predicate(&filter);

    let order: Box<
        dyn BoxableExpression<
            episodes,
            diesel::pg::Pg,
            SqlType = NotSelectable,
        >,
    > = create_order_expression!(
        sort,
        title,
        series_id,
        is_published,
        order_index
    );

    let results: Vec<(Episode, Option<chrono::NaiveDateTime>, Option<String>)> = match episodes
        .limit(range.count)
        .offset(range.start)
        .order_by(order)
        .select((
            episodes::all_columns(),
            diesel::dsl::sql::<diesel::sql_types::Nullable<diesel::sql_types::Timestamp>>(
                "(SELECT stream_date FROM streams WHERE streams.id = episodes.stream_id)",
            ),
            diesel::dsl::sql::<diesel::sql_types::Nullable<diesel::sql_types::Text>>(
                "(SELECT playlist_id FROM series WHERE series.id = episodes.series_id)",
            ),
        ))
        .filter(predicate)
        .load::<(Episode, Option<chrono::NaiveDateTime>, Option<String>)>(&mut db.connection)
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
        .map(EpisodeSimpleView::from)
        .collect::<Vec<EpisodeSimpleView>>();

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
) -> Box<
    dyn BoxableExpression<
        episodes,
        diesel::pg::Pg,
        SqlType = diesel::sql_types::Bool,
    >,
> {
    use crate::schema::episodes::dsl::*;

    let id_filter: Box<
        dyn BoxableExpression<
            episodes,
            diesel::pg::Pg,
            SqlType = diesel::sql_types::Bool,
        >,
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
        dyn BoxableExpression<
            episodes,
            diesel::pg::Pg,
            SqlType = diesel::sql_types::Bool,
        >,
    > = match filter["title"].as_str() {
        Some(q) => Box::new(title.ilike(format!("%{}%", q))),
        None => Box::new(id.ne(Uuid::nil())),
    };

    let is_published_filter: Box<
        dyn BoxableExpression<
            episodes,
            diesel::pg::Pg,
            SqlType = diesel::sql_types::Bool,
        >,
    > = match filter["is_published"].as_bool() {
        Some(true) => Box::new(is_published.eq(true)),
        Some(false) => Box::new(is_published.eq(false)),
        None => Box::new(id.ne(Uuid::nil())),
    };

    let series_id_filter: Box<
        dyn BoxableExpression<
            episodes,
            diesel::pg::Pg,
            SqlType = diesel::sql_types::Bool,
        >,
    > = match filter["series_id"].as_str() {
        Some(q) => match Uuid::parse_str(q) {
            Ok(q) => Box::new(series_id.assume_not_null().eq(q)),
            Err(_) => Box::new(id.ne(Uuid::nil())),
        },

        None => Box::new(id.ne(Uuid::nil())),
    };

    let stream_id_filter: Box<
        dyn BoxableExpression<
            episodes,
            diesel::pg::Pg,
            SqlType = diesel::sql_types::Bool,
        >,
    > = match filter["stream_id"].as_str() {
        Some(q) => match Uuid::parse_str(q) {
            Ok(q) => Box::new(stream_id.assume_not_null().eq(q)),
            Err(_) => Box::new(id.ne(Uuid::nil())),
        },

        None => Box::new(id.ne(Uuid::nil())),
    };

    let stream_name_filter: Box<
        dyn BoxableExpression<
            episodes,
            diesel::pg::Pg,
            SqlType = diesel::sql_types::Bool,
        >,
    > = match filter["stream_name"].as_str() {
        Some(q) => Box::new(
            stream_id.assume_not_null().eq_any(
                crate::schema::streams::dsl::streams
                    .select(streams::id)
                    .filter(streams::title.ilike(format!("%{}%", q))),
            ),
        ),
        None => Box::new(id.ne(Uuid::nil())),
    };

    Box::new(
        id_filter
            .and(title_filter)
            .and(is_published_filter)
            .and(series_id_filter)
            .and(stream_id_filter)
            .and(stream_name_filter),
    )
}
