use serde::{Deserialize, Serialize};

use crate::models::VideoClip;

#[derive(Debug, Deserialize)]
pub struct CreateVideoClipRequest {
    pub title: String,
    pub description: Option<String>,
    pub url: Option<String>,
    pub duration: Option<String>,
    pub start_time: Option<String>,
    pub stream_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateVideoClipRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub url: Option<String>,
    pub duration: Option<String>,
    pub start_time: Option<String>,
    pub stream_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VideoClipDetailView {
    pub id: String,
    pub title: String,
    pub description: String,
    pub url: String,
    pub duration: String,
    pub start_time: String,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub stream_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct VideoClipSimpleView {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: Option<String>,
}

impl From<VideoClip> for VideoClipSimpleView {
    fn from(video_clip: VideoClip) -> Self {
        VideoClipSimpleView {
            id: video_clip.id.to_string(),
            title: video_clip.title.to_string(),

            created_at: video_clip.created_at.to_string(),
            updated_at: video_clip.updated_at.map(|dt| dt.to_string()),
        }
    }
}

impl From<VideoClip> for VideoClipDetailView {
    fn from(video_clip: VideoClip) -> Self {
        VideoClipDetailView {
            id: video_clip.id.to_string(),
            title: video_clip.title.to_string(),
            description: video_clip.description.to_string(),
            url: video_clip.url.to_string(),
            duration: chrono::Duration::microseconds(video_clip.duration.microseconds).to_string(),
            start_time: chrono::Duration::microseconds(video_clip.start_time.microseconds)
                .to_string(),
            stream_id: video_clip.stream_id.map(|id| id.to_string()),
            created_at: video_clip.created_at.to_string(),
            updated_at: video_clip.updated_at.map(|dt| dt.to_string()),
        }
    }
}
