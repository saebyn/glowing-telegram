use serde::{Deserialize, Serialize};

use crate::models::Series;

#[derive(Debug, Deserialize)]
pub struct CreateSeriesRequest {
    pub title: String,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub playlist_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSeriesRequest {
    pub title: Option<String>,
    pub description: Option<String>,
    pub thumbnail_url: Option<String>,
    pub playlist_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SeriesDetailView {
    pub id: String,
    pub title: String,
    pub description: String,
    pub thumbnail_url: Option<String>,
    pub playlist_id: Option<String>,
    pub created_at: String,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SeriesSimpleView {
    pub id: String,
    pub title: String,
    pub created_at: String,
    pub updated_at: Option<String>,

    pub max_episode_order_index: Option<i32>,
}

impl From<Series> for SeriesSimpleView {
    fn from(series: Series) -> Self {
        SeriesSimpleView {
            id: series.id.to_string(),
            title: series.title.to_string(),

            created_at: series.created_at.to_string(),
            updated_at: series.updated_at.map(|dt| dt.to_string()),

            max_episode_order_index: None,
        }
    }
}

impl From<(Series, i32)> for SeriesSimpleView {
    fn from((series, max_episode_order_index): (Series, i32)) -> Self {
        SeriesSimpleView {
            id: series.id.to_string(),
            title: series.title.to_string(),

            created_at: series.created_at.to_string(),
            updated_at: series.updated_at.map(|dt| dt.to_string()),

            max_episode_order_index: Some(max_episode_order_index),
        }
    }
}

impl From<Series> for SeriesDetailView {
    fn from(series: Series) -> Self {
        SeriesDetailView {
            id: series.id.to_string(),
            title: series.title.to_string(),
            description: series.description.to_string(),
            thumbnail_url: series.thumbnail_url.map(|url| url.to_string()),
            playlist_id: series.playlist_id.map(|id| id.to_string()),
            created_at: series.created_at.to_string(),
            updated_at: series.updated_at.map(|dt| dt.to_string()),
        }
    }
}
