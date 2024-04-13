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
use crate::handlers::utils::parse_duration;
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
    pub transcription_task_url: Option<String>,
    // TODO - this should be a Option<Vec<Segment>>
    pub transcription_segments: Option<serde_json::Value>,

    pub silence_detection_task_url: Option<String>,
    // TODO - this should be a Option<Vec<Segment>>
    pub silence_segments: Option<serde_json::Value>,
}

impl UpdateStreamChangeset {
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.description.is_none()
            && self.thumbnail_url.is_none()
            && self.prefix.is_none()
            && self.speech_audio_url.is_none()
            && self.transcription_task_url.is_none()
            && self.transcription_segments.is_none()
            && self.silence_detection_task_url.is_none()
            && self.silence_segments.is_none()
    }
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
    fn from((clip, stream_id): (VideoClipInlineView, uuid::Uuid)) -> VideoClipInsertable {
        let duration = parse_duration(Some(clip.duration));
        let start_time = parse_duration(Some(clip.start_time));

        VideoClipInsertable {
            id: clip.id.map_or(None, |id| uuid::Uuid::try_parse(&id).ok()),
            title: clip.title,
            description: "".to_string(),
            uri: clip.uri,
            duration,
            start_time,
            stream_id: Some(stream_id),
            audio_bitrate: clip.audio_bitrate,
            audio_track_count: clip.audio_track_count,
            content_type: clip.content_type,
            filename: clip.filename,
            frame_rate: clip.frame_rate,
            height: clip.height,
            width: clip.width,
            video_bitrate: clip.video_bitrate,
            size: clip.size,
            last_modified: clip.last_modified.map_or(None, |last_modified| {
                iso8601::datetime(&last_modified.to_string())
                    .map_or(None, |dt: iso8601::DateTime| dt.into_naive())
            }),
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

impl From<VideoClipInlineView> for VideoClipChangeset {
    fn from(clip: VideoClipInlineView) -> VideoClipChangeset {
        let duration = parse_duration(Some(clip.duration));
        let start_time = parse_duration(Some(clip.start_time));

        VideoClipChangeset {
            title: Some(clip.title),
            description: None,
            uri: Some(clip.uri),
            duration: Some(duration),
            start_time: Some(start_time),
            stream_id: None,
            audio_bitrate: clip.audio_bitrate,
            audio_track_count: clip.audio_track_count,
            content_type: clip.content_type,
            filename: clip.filename,
            frame_rate: clip.frame_rate,
            height: clip.height,
            width: clip.width,
            video_bitrate: clip.video_bitrate,
            size: clip.size,
            last_modified: clip.last_modified.map_or(None, |last_modified| {
                iso8601::datetime(&last_modified.to_string())
                    .map_or(None, |dt: iso8601::DateTime| dt.into_naive())
            }),
        }
    }
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
            match diesel::insert_into(crate::schema::video_clips::table)
                .values(VideoClipInsertable::from((video_clip.clone(), record_id)))
                .on_conflict(crate::schema::video_clips::dsl::id)
                .do_update()
                .set(VideoClipChangeset::from(video_clip))
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

    let changeset = UpdateStreamChangeset {
        title: body.title,
        description: body.description,
        thumbnail_url: body.thumbnail,
        prefix: body.prefix,
        speech_audio_url: body.speech_audio_track,
        transcription_task_url: body.transcription_task_url,
        transcription_segments: body.transcription_segments,
        silence_detection_task_url: body.silence_detection_task_url,
        silence_segments: body.silence_segments,
    };

    let result: Result<Stream, diesel::result::Error> = if !changeset.is_empty() {
        // If any of the fields are present in the body besides the video_clips, update the stream record
        diesel::update(streams.filter(id.eq(record_id)))
            .set(&changeset)
            .get_result(&mut db.connection)
            .await
    } else {
        streams
            .filter(id.eq(record_id))
            .get_result(&mut db.connection)
            .await
    };

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
