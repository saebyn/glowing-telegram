use axum::extract::Path;
use axum::response::IntoResponse;
use axum::Json;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::json;
use tracing;
use tracing::instrument;
use uuid::Uuid;

use common_api_lib::db::DbConnection;

use super::structs::StreamDetailView;
use super::structs::UpdateStreamRequest;
use super::structs::VideoClipInlineView;
use crate::models::Stream;
use crate::models::VideoClip;
use crate::schema::video_clips;
use crate::schema::{self, streams};

#[derive(Debug, AsChangeset)]
#[diesel(table_name = streams)]
pub struct UpdateStreamChangeset {
    pub title: Option<String>,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub prefix: Option<String>,
    pub speech_audio_url: Option<String>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = video_clips)]
pub struct VideoClipInsertable {
    pub id: Option<uuid::Uuid>,
    pub title: String,
    pub description: String,
    pub uri: String,
    pub duration: diesel::pg::data_types::PgInterval,
    pub start_time: diesel::pg::data_types::PgInterval,
    pub stream_id: Option<uuid::Uuid>,
    pub audio_bitrate: Option<i32>,
    pub audio_track_count: Option<i32>,
    pub content_type: Option<String>,
    pub filename: Option<String>,
    pub frame_rate: Option<f32>,
    pub height: Option<i32>,
    pub width: Option<i32>,
    pub video_bitrate: Option<i32>,
    pub size: Option<i64>,
    pub last_modified: Option<chrono::NaiveDateTime>,
}

impl From<(VideoClipInlineView, uuid::Uuid)> for VideoClipInsertable {
    fn from(self, (clip, stream_id)) -> VideoClipInsertable {
        VideoClipInsertable {
            id: self.id,
            title: self.title,
            description: "".to_string(),
            uri: self.uri,
            duration: self.duration,
            start_time: self.start_time,
            stream_id: self.stream_id,
            audio_bitrate: self.audio_bitrate,
            audio_track_count: self.audio_track_count,
            content_type: self.content_type,
            filename: self.filename,
            frame_rate: self.frame_rate,
            height: self.height,
            width: self.width,
            video_bitrate: self.video_bitrate,
            size: self.size,
            last_modified: self.last_modified,
        }
    }
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = video_clips)]
pub struct VideoClipChangeset {
    pub title: Option<String>,
    pub description: Option<String>,
    pub uri: Option<String>,
    pub duration: Option<diesel::pg::data_types::PgInterval>,
    pub start_time: Option<diesel::pg::data_types::PgInterval>,
    pub stream_id: Option<uuid::Uuid>,
    pub audio_bitrate: Option<i32>,
    pub audio_track_count: Option<i32>,
    pub content_type: Option<String>,
    pub filename: Option<String>,
    pub frame_rate: Option<f32>,
    pub height: Option<i32>,
    pub width: Option<i32>,
    pub video_bitrate: Option<i32>,
    pub size: Option<i64>,
    pub last_modified: Option<chrono::NaiveDateTime>,
}

#[instrument]
pub async fn handler(
    DbConnection(mut db): DbConnection<'_>,
    Path(record_id): Path<Uuid>,
    Json(body): Json<UpdateStreamRequest>,
) -> impl IntoResponse {
    use schema::streams::dsl::*;

    tracing::info!("update_stream");

    // insert body.video_clips into video_clips table, updating existing records and deleting missing records
    if let Some(video_clips) = body.video_clips {
        for video_clip in video_clips {
            let duration_value = match crate::handlers::utils::parse_duration(video_clip.duration) {
                Ok(duration_value) => duration_value,
                Err(e) => {
                    tracing::error!("Error parsing duration: {}", e);
                    return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
                }
            };

            let start_time_value =
                match crate::handlers::utils::parse_duration(video_clip.start_time) {
                    Ok(start_time_value) => start_time_value,
                    Err(e) => {
                        tracing::error!("Error parsing start_time: {}", e);
                        return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
                    }
                };

            let record = match diesel::insert_into(crate::schema::video_clips::table)
                .values((
                    crate::schema::video_clips::dsl::title.eq(video_clip.title),
                    crate::schema::video_clips::dsl::description
                        .eq(video_clip.description.unwrap_or("".to_string())),
                    crate::schema::video_clips::dsl::uri
                        .eq(video_clip.uri.unwrap_or("".to_string())),
                    crate::schema::video_clips::dsl::duration.eq(duration_value),
                    crate::schema::video_clips::dsl::start_time.eq(start_time_value),
                    crate::schema::video_clips::dsl::stream_id.eq(record_id),
                ))
                .on_conflict(crate::schema::video_clips::dsl::id)
                .do_update()
                .set((
                    crate::schema::video_clips::dsl::title.eq(video_clip.title),
                    crate::schema::video_clips::dsl::description
                        .eq(video_clip.description.unwrap_or("".to_string())),
                    crate::schema::video_clips::dsl::uri
                        .eq(video_clip.uri.unwrap_or("".to_string())),
                    crate::schema::video_clips::dsl::duration.eq(duration_value),
                    crate::schema::video_clips::dsl::start_time.eq(start_time_value),
                    crate::schema::video_clips::dsl::stream_id.eq(record_id),
                ))
                .get_result::<VideoClip>(&mut db.connection)
                .await
            {
                Ok(record) => record,
                Err(e) => {
                    tracing::error!("Error inserting record: {}", e);
                    return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
                }
            };
        }
    };

    let result: Result<Stream, diesel::result::Error> =
        diesel::update(streams.filter(id.eq(record_id)))
            .set(&UpdateStreamChangeset {
                title: body.title,
                description: body.description,
                thumbnail_url: body.thumbnail,
                prefix: body.prefix,
                speech_audio_url: body.speech_audio_track,
            })
            .get_result(&mut db.connection)
            .await;

    let video_clips_result: Result<Vec<VideoClip>, _> =
        crate::schema::video_clips::dsl::video_clips
            .filter(crate::schema::video_clips::dsl::stream_id.eq(record_id))
            .select(crate::schema::video_clips::dsl::video_clips::all_columns())
            .load(&mut db.connection)
            .await;

    let video_clips = match video_clips_result {
        Ok(video_clips) => video_clips,
        Err(_) => vec![],
    };

    match result {
        Ok(result) => (
            [(axum::http::header::CONTENT_TYPE, "application/json")],
            axum::Json(json!(StreamDetailView::from((result, video_clips)))),
        )
            .into_response(),

        Err(e) => {
            tracing::error!("Error updating record: {}", e);
            return (axum::http::StatusCode::INTERNAL_SERVER_ERROR).into_response();
        }
    }
}
