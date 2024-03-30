use axum::extract::Json;
use axum::response::IntoResponse;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::json;
use tracing;
use tracing::instrument;

use common_api_lib::db::DbConnection;

use super::structs::{CreateTopicRequest, TopicDetailView};
use crate::models::Topic;

#[instrument]
pub async fn handler(
    DbConnection(mut db): DbConnection<'_>,
    Json(body): Json<CreateTopicRequest>,
) -> impl IntoResponse {
    use crate::schema::topics::dsl::*;

    tracing::info!("create_topic");

    let record = match diesel::insert_into(topics)
        .values((title.eq(body.title), description.eq(body.description)))
        .get_result::<Topic>(&mut db.connection)
        .await
    {
        Ok(record) => record,
        Err(e) => {
            tracing::error!("Error inserting record: {}", e);
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    };

    axum::Json(json!(TopicDetailView::from(record))).into_response()
}
