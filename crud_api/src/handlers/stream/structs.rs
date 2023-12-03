use serde::{Deserialize, Serialize};

use crate::{
    handlers::utils::parse_duration_to_string,
    models::{Stream, VideoClip},
};

#[derive(Debug, Deserialize)]
pub struct CreateStreamRequest {
    pub title: String,
    pub description: Option<String>,
    pub thumbnail: Option<String>,
    pub topic_ids: Option<Vec<i32>>,
    pub prefix: String,
    pub speech_audio_track: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateStreamRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub thumbnail: Option<String>,
    pub topic_ids: Option<Vec<i32>>,
    pub prefix: Option<String>,
    pub speech_audio_track: Option<String>,

    pub video_clips: Option<Vec<VideoClipInlineView>>,
}

#[derive(Debug, Serialize)]
pub struct StreamDetailView {
    pub id: String,
    pub title: String,
    pub description: String,
    pub prefix: String,
    pub thumbnail: String,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub topic_ids: Vec<String>,

    pub video_clips: Vec<VideoClipInlineView>,
}

#[derive(Debug, Serialize)]
pub struct StreamSimpleView {
    pub id: String,
    pub title: String,
    pub prefix: String,
    pub thumbnail: String,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub topic_ids: Vec<String>,
}

impl From<Stream> for StreamSimpleView {
    fn from(stream: Stream) -> Self {
        StreamSimpleView {
            id: stream.id.to_string(),
            title: stream.title.to_string(),
            prefix: stream.prefix.to_string(),
            thumbnail: stream.thumbnail_url.to_string(),
            created_at: stream.created_at.to_string(),
            updated_at: stream.updated_at.map(|dt| dt.to_string()),
            topic_ids: vec![],
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct VideoClipInlineView {
    pub id: Option<String>,
    pub title: String,
    pub uri: String,
    pub duration: String,
    pub start_time: String,
    pub audio_bitrate: Option<i32>,
    pub audio_track_count: Option<i32>,
    pub content_type: Option<String>,
    pub filename: Option<String>,
    pub frame_rate: Option<f32>,
    pub height: Option<i32>,
    pub width: Option<i32>,
    pub video_bitrate: Option<i32>,
    pub size: Option<i64>,
    pub last_modified: Option<String>,
}

impl Into<VideoClipInlineView> for VideoClip {
    fn into(self) -> VideoClipInlineView {
        VideoClipInlineView {
            id: Some(self.id.to_string()),
            title: self.title.to_string(),
            uri: self.uri.to_string(),
            duration: parse_duration_to_string(self.duration),
            start_time: parse_duration_to_string(self.start_time),
            audio_bitrate: self.audio_bitrate,
            audio_track_count: self.audio_track_count,
            content_type: self.content_type.to_owned(),
            filename: self.filename.to_owned(),
            frame_rate: self.frame_rate,
            height: self.height,
            width: self.width,
            video_bitrate: self.video_bitrate,
            size: self.size,
            last_modified: self.last_modified.map(|dt| dt.to_string()),
        }
    }
}

impl From<(Stream, Vec<VideoClip>)> for StreamDetailView {
    fn from((stream, video_clips): (Stream, Vec<VideoClip>)) -> Self {
        StreamDetailView {
            id: stream.id.to_string(),
            title: stream.title.to_string(),
            description: stream.description.to_string(),
            prefix: stream.prefix.to_string(),

            thumbnail: stream.thumbnail_url.to_string(),
            created_at: stream.created_at.to_string(),
            updated_at: stream.updated_at.map(|dt| dt.to_string()),
            topic_ids: vec![],

            video_clips: video_clips.into_iter().map(|vc| vc.into()).collect(),
        }
    }
}
