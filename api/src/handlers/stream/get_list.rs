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
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR)
                .into_response();
        }
    };

    let predicate = create_predicate(&filter);

    let order: Box<
        dyn BoxableExpression<
            streams,
            diesel::pg::Pg,
            SqlType = NotSelectable,
        >,
    > = create_order_expression!(sort, title, stream_date, prefix);

    let results: Vec<(Stream, i64, i64)> = match streams
        .limit(range.count)
        .offset(range.start)
        .order_by(order)
        .select((
            streams::all_columns(),
            // count of video clips
            diesel::dsl::sql::<diesel::sql_types::BigInt>(
                "(SELECT COUNT(*) FROM video_clips WHERE video_clips.stream_id = streams.id)",
            ),
            // count of episodes
            diesel::dsl::sql::<diesel::sql_types::BigInt>(
                "(SELECT COUNT(*) FROM episodes WHERE episodes.stream_id = streams.id)",
            ),
        ))
        .filter(predicate)
        .load::<(Stream, i64, i64)>(&mut db.connection)
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
        .map(StreamSimpleView::from)
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
) -> Box<
    dyn BoxableExpression<
        streams,
        diesel::pg::Pg,
        SqlType = diesel::sql_types::Bool,
    >,
> {
    use crate::schema::streams::dsl::*;

    let id_filter: Box<
        dyn BoxableExpression<
            streams,
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
            streams,
            diesel::pg::Pg,
            SqlType = diesel::sql_types::Bool,
        >,
    > = match filter["q"].as_str() {
        Some(q) => Box::new(title.ilike(format!("%{}%", q))),
        None => Box::new(id.ne(Uuid::nil())),
    };

    let has_video_clips_filter: Box<
        dyn BoxableExpression<streams, diesel::pg::Pg, SqlType = diesel::sql_types::Bool>,
    > = match filter["has_video_clips"].as_bool() {
        Some(true) => Box::new(diesel::dsl::sql::<diesel::sql_types::Bool>(
            "(SELECT COUNT(*) FROM video_clips WHERE video_clips.stream_id = streams.id) > 0",
        )),
        Some(false) => Box::new(diesel::dsl::sql::<diesel::sql_types::Bool>(
            "(SELECT COUNT(*) FROM video_clips WHERE video_clips.stream_id = streams.id) = 0",
        )),
        None => Box::new(id.ne(Uuid::nil())),
    };

    let has_transcription_filter: Box<
        dyn BoxableExpression<
            streams,
            diesel::pg::Pg,
            SqlType = diesel::sql_types::Bool,
        >,
    > = match filter["has_transcription"].as_bool() {
        Some(true) => Box::new(transcription_segments.is_not_null()),
        Some(false) => Box::new(transcription_segments.is_null()),
        None => Box::new(id.ne(Uuid::nil())),
    };

    let has_silence_detection_filter: Box<
        dyn BoxableExpression<
            streams,
            diesel::pg::Pg,
            SqlType = diesel::sql_types::Bool,
        >,
    > = match filter["has_silence_detection"].as_bool() {
        Some(true) => Box::new(silence_segments.is_not_null()),
        Some(false) => Box::new(silence_segments.is_null()),
        None => Box::new(id.ne(Uuid::nil())),
    };

    let has_episodes_filter: Box<
        dyn BoxableExpression<streams, diesel::pg::Pg, SqlType = diesel::sql_types::Bool>,
    > = match filter["has_episodes"].as_bool() {
        Some(true) => Box::new(diesel::dsl::sql::<diesel::sql_types::Bool>(
            "(SELECT COUNT(*) FROM episodes WHERE episodes.stream_id = streams.id) > 0",
        )),
        Some(false) => Box::new(diesel::dsl::sql::<diesel::sql_types::Bool>(
            "(SELECT COUNT(*) FROM episodes WHERE episodes.stream_id = streams.id) = 0",
        )),
        None => Box::new(id.ne(Uuid::nil())),
    };

    // stream_date__gte
    let stream_date_gte_filter: Box<
        dyn BoxableExpression<
            streams,
            diesel::pg::Pg,
            SqlType = diesel::sql_types::Bool,
        >,
    > = match filter["stream_date__gte"].as_str() {
        Some(stream_date_gte) => {
            match chrono::NaiveDate::parse_from_str(
                stream_date_gte,
                "%Y-%m-%d",
            ) {
                Ok(stream_date_gte) => {
                    Box::new(stream_date.ge(stream_date_gte.and_time(
                        chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap(),
                    )))
                }
                Err(e) => {
                    tracing::error!("Error parsing stream_date__gte: {}", e);
                    return Box::new(id.ne(Uuid::nil()));
                }
            }
        }
        None => Box::new(id.ne(Uuid::nil())),
    };

    // series_id
    let series_id_filter: Box<
        dyn BoxableExpression<
            streams,
            diesel::pg::Pg,
            SqlType = diesel::sql_types::Bool,
        >,
    > = match filter["series_id"].as_str() {
        Some(series_id_value) => match Uuid::parse_str(series_id_value) {
            Ok(series_id_value) => {
                Box::new(series_id.assume_not_null().eq(series_id_value))
            }
            Err(e) => {
                tracing::error!("Error parsing series_id: {}", e);
                return Box::new(id.ne(Uuid::nil()));
            }
        },
        None => Box::new(id.ne(Uuid::nil())),
    };

    Box::new(
        id_filter
            .and(title_filter)
            .and(has_video_clips_filter)
            .and(has_transcription_filter)
            .and(has_silence_detection_filter)
            .and(has_episodes_filter)
            .and(stream_date_gte_filter)
            .and(series_id_filter),
    )
}
