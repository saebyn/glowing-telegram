use axum::response::IntoResponse;
use axum::extract::{Path, State};
use axum::http::header;
use serde::Serialize;
use diesel::prelude::*;
use tracing;
use tracing::instrument;
use diesel_async::{RunQueryDsl};

use crate::state::AppState;
use crate::schema;

#[derive(Debug, Serialize)]
struct StreamView {
  pub id: String,
  pub title: String,
  pub thumbnail: String,
  pub created_at: String,
  pub updated_at: Option<String>,
  pub topic_ids: Vec<String>,
}

#[instrument]
pub async fn handler(
  Path(key): Path<String>,
  State(state): State<AppState>,
) -> impl IntoResponse {
    use schema::streams::dsl::*;

    tracing::info!("get_list");

    // get list of records from streams table using diesel
    let mut connection = state.pool.get().await.unwrap();
    let results: Vec<crate::models::Stream> = streams
        .limit(10)
        .offset(0)
        .load(&mut connection).await.unwrap();

    let pagination_info = "items 0-10/10";

    // convert the results into a JSON response
    let prepared_results = results
        .iter()
        .map(|stream| {
            StreamView {
                id: stream.id.to_string(),
                title: stream.title.to_string(),
                thumbnail: stream.thumbnail_url.to_string(),
                created_at: stream.created_at.to_string(),
                updated_at: stream.updated_at.map(|dt| dt.to_string()),
                topic_ids: vec![],
            }
        })
        .collect::<Vec<StreamView>>();
    
    ([(header::CONTENT_RANGE, pagination_info)], axum::Json(prepared_results))
}
