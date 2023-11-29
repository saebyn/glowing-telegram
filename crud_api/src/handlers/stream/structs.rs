use serde::{Deserialize, Serialize};

use crate::models::Stream;

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
}

#[derive(Debug, Serialize)]
pub struct StreamDetailView {
    pub id: String,
    pub title: String,
    pub description: String,
    pub thumbnail: String,
    pub created_at: String,
    pub updated_at: Option<String>,
    pub topic_ids: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct StreamSimpleView {
    pub id: String,
    pub title: String,
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
            thumbnail: stream.thumbnail_url.to_string(),
            created_at: stream.created_at.to_string(),
            updated_at: stream.updated_at.map(|dt| dt.to_string()),
            topic_ids: vec![],
        }
    }
}

impl From<Stream> for StreamDetailView {
    fn from(stream: Stream) -> Self {
        StreamDetailView {
            id: stream.id.to_string(),
            title: stream.title.to_string(),
            description: stream.description.to_string(),
            thumbnail: stream.thumbnail_url.to_string(),
            created_at: stream.created_at.to_string(),
            updated_at: stream.updated_at.map(|dt| dt.to_string()),
            topic_ids: vec![],
        }
    }
}
