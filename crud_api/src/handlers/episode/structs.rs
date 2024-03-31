use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::Episode;

#[derive(Debug, Serialize)]
pub struct EpisodeSimpleView {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: Option<String>,
}

impl From<Episode> for EpisodeSimpleView {
    fn from(episode: Episode) -> Self {
        EpisodeSimpleView {
            id: episode.id.to_string(),
            title: episode.title,
            created_at: episode.created_at.to_string(),
            updated_at: episode.updated_at.map(|dt| dt.to_string()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct EpisodeDetailView {
    pub id: String,
    pub title: String,
    pub description: String,
    pub thumbnail_url: Option<String>,
    pub created_at: String,
    pub updated_at: Option<String>,

    pub render_uri: Option<String>,

    pub stream_id: String,

    pub tracks: Vec<Track>,
}

impl From<Episode> for EpisodeDetailView {
    fn from(episode: Episode) -> Self {
        EpisodeDetailView {
            id: episode.id.to_string(),
            title: episode.title,
            description: episode.description,
            thumbnail_url: episode.thumbnail_url,
            created_at: episode.created_at.to_string(),
            updated_at: episode.updated_at.map(|dt| dt.to_string()),

            render_uri: episode.render_uri,

            stream_id: episode.stream_id.to_string(),

            tracks: serde_json::from_value(episode.tracks).unwrap_or(vec![]),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Track {
    start: iso8601::Duration,
    end: iso8601::Duration,
}

#[derive(Debug, Deserialize)]
pub struct CreateEpisodeRequest {
    pub title: String,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,

    pub stream_id: Uuid,

    pub tracks: Vec<Track>,
}

#[derive(Debug, Deserialize)]
pub struct BulkCreateEpisodeRequest {
    pub records: Vec<CreateEpisodeRequest>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEpisodeRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub stream_id: Option<Uuid>,
    pub tracks: Option<Vec<Track>>,
    pub render_uri: Option<String>,
}
