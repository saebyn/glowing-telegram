use crate::models::Topic;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct CreateTopicRequest {
    pub title: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct TopicDetailView {
    pub id: String,
    pub title: String,
    pub description: String,
    pub created_at: String,
    pub updated_at: Option<String>,
}

impl From<Topic> for TopicDetailView {
    fn from(topic: Topic) -> Self {
        TopicDetailView {
            id: topic.id.to_string(),
            title: topic.title,
            description: topic.description,
            created_at: topic.created_at.to_string(),
            updated_at: topic.updated_at.map(|x| x.to_string()),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct TopicSimpleView {
    pub id: String,
    pub title: String,
    pub description: String,
}

impl From<Topic> for TopicSimpleView {
    fn from(topic: Topic) -> Self {
        TopicSimpleView {
            id: topic.id.to_string(),
            title: topic.title,
            description: topic.description,
        }
    }
}
