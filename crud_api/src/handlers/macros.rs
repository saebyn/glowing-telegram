#[macro_export]
macro_rules! create_order_expression {
  ($sort:expr, $($field:ident),*) => {
      match $sort {
          Some((sort, order)) => match (sort.as_str(), order.as_str()) {
              ("id", "ASC") => Box::new(id.asc()),
              ("id", "DESC") => Box::new(id.desc()),
              ("created_at", "ASC") => Box::new(created_at.asc()),
              ("created_at", "DESC") => Box::new(created_at.desc()),
              ("updated_at", "ASC") => Box::new(updated_at.asc()),
              ("updated_at", "DESC") => Box::new(updated_at.desc()),
              $(
                  (stringify!($field), "ASC") => Box::new($field.asc()),
                  (stringify!($field), "DESC") => Box::new($field.desc()),
              )*
              _ => Box::new(id.asc()),
          },
          _ => Box::new(id.asc()),
      }
  };
}

#[macro_export]
macro_rules! create_list_handler {
    ($func:ident,
      $table:ident, $struct:ident, $struct_view_simple:ident, $($field:ident),*) => {
        #[instrument]
        pub async fn $func(
            DbConnection(mut db): DbConnection<'_>,
            Query(params): Query<ListParams>,
        ) -> impl IntoResponse {
            use crate::create_order_expression;
            use uuid::Uuid;
            use schema::$table::dsl::*;
            tracing::info!("get_{}_list", stringify!($table));

            let ListParams { range, sort, filter } = params;

            let ids_filter: Box<dyn BoxableExpression<$table,
                diesel::pg::Pg, SqlType = diesel::sql_types::Bool>> = match filter {
                Some(ref filter) => match filter.id.len() {
                    0 => Box::new(id.ne(Uuid::nil())),
                    _ => Box::new(id.eq_any(filter.id.clone())),
                },
                None => Box::new(id.ne(Uuid::nil())),
            };


            let total: i64 = match $table.filter(
                ids_filter
            )
            .count().get_result(&mut db.connection).await {
                Ok(total) => total,
                Err(e) => {
                    tracing::error!("Error getting total count: {}", e);
                    return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
                }
            };

            let order: Box<dyn BoxableExpression<$table, diesel::pg::Pg, SqlType = NotSelectable>> =
                create_order_expression!(sort, $($field),*);

            let ids_filter: Box<dyn BoxableExpression<$table,
            diesel::pg::Pg, SqlType = diesel::sql_types::Bool>> = match filter {
                Some(filter) => match filter.id.len() {
                    0 => Box::new(id.ne(Uuid::nil())),
                    _ => Box::new(id.eq_any(filter.id)),
                },
                None => Box::new(id.ne(Uuid::nil())),
            };

            let results: Vec<$struct> = match $table
                .limit(range.count)
                .offset(range.start)
                .order_by(order)
                .select($table::all_columns())
                .filter(
                    ids_filter
                )
                .load(&mut db.connection)
                .await
            {
                Ok(results) => results,
                Err(e) => {
                    tracing::error!("Error getting results: {}", e);
                    return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
                }
            };

            // If the

            let prepared_results = results
                .into_iter()
                .map(|record| $struct_view_simple::from(record))
                .collect::<Vec<$struct_view_simple>>();

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
    };
}
