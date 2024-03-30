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

use super::structs::StreamSimpleView;
use crate::handlers::structs::ListParams;
use crate::models::Stream;
use crate::{create_list_handler, schema};

create_list_handler!(
    handler,
    streams,
    Stream,
    StreamSimpleView,
    title,
    prefix,
    stream_date
);
