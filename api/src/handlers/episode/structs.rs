use diesel::{AsChangeset, Insertable};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use crate::{models::Episode, schema::episodes};

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
    pub has_youtube_video: bool,

    pub stream_date: Option<String>,

    pub notify_subscribers: bool,
    pub category: i16,
    pub tags: Vec<Option<String>>,
}

impl From<(Episode, Option<chrono::NaiveDateTime>, Option<String>)>
    for EpisodeSimpleView
{
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
            playlist_id,
            is_published: episode.is_published,

            render_uri: episode.render_uri,
            has_youtube_video: episode.youtube_video_id.is_some(),

            stream_date: stream_date
                .map(|dt| dt.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()),

            notify_subscribers: episode.notify_subscribers,
            category: episode.category,
            tags: episode.tags,
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
            has_youtube_video: episode.youtube_video_id.is_some(),

            stream_date: None,

            notify_subscribers: episode.notify_subscribers,
            category: episode.category,
            tags: episode.tags,
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

    pub notify_subscribers: bool,
    pub category: i16,
    pub tags: Vec<Option<String>>,
    pub youtube_video_id: Option<String>,
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

            notify_subscribers: episode.notify_subscribers,
            category: episode.category,
            tags: episode.tags,
            youtube_video_id: episode.youtube_video_id,
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
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

    pub notify_subscribers: Option<bool>,
    pub category: Option<i16>,
    pub tags: Option<Vec<Option<String>>>,
    pub youtube_video_id: Option<String>,
}

#[derive(Debug, Insertable)]
#[diesel(table_name = episodes)]
pub struct CreateEpisodeInsertable {
    pub title: String,
    pub description: String,
    pub thumbnail_url: Option<String>,
    pub stream_id: Uuid,
    pub series_id: Option<Uuid>,
    pub order_index: i32,
    pub tracks: serde_json::Value,
    pub notify_subscribers: bool,
    pub category: i16,
    pub tags: Vec<Option<String>>,
    pub youtube_video_id: Option<String>,
}

impl From<&CreateEpisodeRequest> for CreateEpisodeInsertable {
    fn from(body: &CreateEpisodeRequest) -> Self {
        CreateEpisodeInsertable {
            title: body.title.clone(),
            description: match &body.description {
                Some(description) => description.clone(),
                None => "".to_string(),
            },
            thumbnail_url: body.thumbnail_url.clone(),
            stream_id: body.stream_id,
            series_id: body.series_id,
            order_index: body.order_index.unwrap_or(0),
            tracks: serde_json::to_value(body.tracks.clone())
                .unwrap_or(json!([])),
            notify_subscribers: body.notify_subscribers.unwrap_or(false),
            category: body.category.unwrap_or(20),
            tags: body.tags.clone().unwrap_or_default(),
            youtube_video_id: body.youtube_video_id.clone(),
        }
    }
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
    pub notify_subscribers: Option<bool>,
    pub category: Option<i16>,
    pub tags: Option<Vec<Option<String>>>,
    pub youtube_video_id: Option<String>,
}

#[derive(Debug, AsChangeset)]
#[diesel(table_name = episodes)]
pub struct UpdateEpisodeChangeset {
    pub title: Option<String>,
    pub description: Option<String>,
    pub stream_id: Option<Uuid>,
    pub render_uri: Option<String>,
    pub tracks: Option<serde_json::Value>,
    pub thumbnail_url: Option<String>,
    pub series_id: Option<Uuid>,
    pub order_index: Option<i32>,
    pub is_published: Option<bool>,
    pub notify_subscribers: Option<bool>,
    pub category: Option<i16>,
    pub tags: Option<Vec<Option<String>>>,
    pub youtube_video_id: Option<String>,
}

impl From<UpdateEpisodeRequest> for UpdateEpisodeChangeset {
    fn from(body: UpdateEpisodeRequest) -> Self {
        let tracks_json = match body.tracks {
            Some(actual_tracks) => match serde_json::to_value(actual_tracks) {
                Ok(value) => Some(value),
                Err(e) => {
                    tracing::error!("Error serializing tracks: {}", e);
                    None
                }
            },

            None => None,
        };

        UpdateEpisodeChangeset {
            title: body.title,
            description: body.description,
            stream_id: body.stream_id,
            render_uri: body.render_uri,
            tracks: tracks_json,
            thumbnail_url: body.thumbnail_url,
            series_id: body.series_id,
            order_index: body.order_index,
            is_published: body.is_published,
            notify_subscribers: body.notify_subscribers,
            category: body.category,
            tags: body.tags,
            youtube_video_id: body.youtube_video_id,
        }
    }
}
