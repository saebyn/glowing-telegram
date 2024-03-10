use serde::Serialize;

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
