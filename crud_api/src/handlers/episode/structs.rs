use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::Episode;

#[derive(Debug, Serialize)]
pub struct EpisodeSimpleView {
    pub id: String,
    pub title: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: Option<String>,

    pub render_uri: Option<String>,
    pub series_id: Option<String>,
    pub order_index: i32,
    pub playlist_id: Option<String>,
    pub is_published: bool,

    pub stream_date: Option<String>,
}

impl From<(Episode, Option<chrono::NaiveDateTime>, Option<String>)> for EpisodeSimpleView {
    fn from(
        (episode, stream_date, playlist_id): (
            Episode,
            Option<chrono::NaiveDateTime>,
            Option<String>,
        ),
    ) -> Self {
        EpisodeSimpleView {
            id: episode.id.to_string(),
            title: episode.title,
            description: episode.description,
            created_at: episode
                .created_at
                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                .to_string(),
            updated_at: episode
                .updated_at
                .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()),

            series_id: episode.series_id.map(|id| id.to_string()),
            order_index: episode.order_index,
            playlist_id: playlist_id,
            is_published: episode.is_published,

            render_uri: episode.render_uri,

            stream_date: stream_date.map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()),
        }
    }
}

impl From<Episode> for EpisodeSimpleView {
    fn from(episode: Episode) -> Self {
        EpisodeSimpleView {
            id: episode.id.to_string(),
            title: episode.title,
            description: episode.description,
            created_at: episode
                .created_at
                .format("%Y-%m-%dT%H:%M:%S%.3fZ")
                .to_string(),
            updated_at: episode
                .updated_at
                .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()),

            series_id: episode.series_id.map(|id| id.to_string()),
            order_index: episode.order_index,
            playlist_id: None,
            is_published: episode.is_published,

            render_uri: episode.render_uri,

            stream_date: None,
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
    pub series_id: Option<String>,
    pub order_index: i32,
    pub is_published: bool,

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
            series_id: episode.series_id.map(|id| id.to_string()),
            order_index: episode.order_index,
            is_published: episode.is_published,

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
    pub series_id: Option<Uuid>,
    pub order_index: Option<i32>,

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
    pub series_id: Option<Uuid>,
    pub order_index: Option<i32>,
    pub is_published: Option<bool>,
}
