use axum::extract::Query;
use axum::http::header;
use axum::response::IntoResponse;
use diesel::expression::expression_types::NotSelectable;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::json;
use tracing;
use tracing::instrument;

use crate::db::DbConnection;

use super::structs::VideoClipSimpleView;
use crate::handlers::structs::ListParams;
use crate::models::VideoClip;

use crate::{create_list_handler, schema};

create_list_handler!(handler, video_clips, VideoClip, VideoClipSimpleView, title);
