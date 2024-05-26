use redact::Secret;
/**
 * This file contains the structs used in the application.
 */
use serde::{Deserialize, Serialize};

use common_api_lib::task::TaskRequest;
use serde_json::json;

/**
 * The application state struct.
 *
 * This struct contains the application state.
 */
#[derive(Clone, Debug)]
pub struct AppState {
    pub redis: redis::Client,

    pub render_storage_path: String,

    pub youtube_auth_uri: String,
    pub youtube_token_uri: String,
    pub youtube_client_id: String,
    pub youtube_client_secret: Secret<String>,

    pub redirect_url: String,

    pub task_api_url: String,
    pub task_api_external_url: String,

    pub this_api_base_url: String,

    pub http_client: reqwest::Client,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct YoutubeUploadRequest {
    title: String,
    description: String,
    #[serde(default = "default_language")]
    language: String,
    tags: Vec<String>,
    category: u8,
    render_uri: String,
    #[serde(default = "default_mime_type")]
    mime_type: String,
    thumbnail_uri: Option<String>,
    recording_date: Option<iso8601::DateTime>,
    playlist_id: Option<String>,
    playlist_position: Option<u32>,
    notify_subscribers: bool,

    task_title: String,
}

#[derive(Serialize, Debug, Deserialize)]
pub struct YoutubeUploadTaskPayload {
    pub title: String,
    pub description: String,
    pub language: String,
    pub mime_type: String,
    pub tags: Vec<String>,
    pub category: u8,
    pub render_uri: String,
    pub thumbnail_uri: Option<String>,
    pub recording_date: Option<iso8601::DateTime>,
    pub playlist_id: Option<String>,
    pub playlist_position: Option<u32>,
    pub notify_subscribers: bool,
}

impl From<&YoutubeUploadRequest> for YoutubeUploadTaskPayload {
    fn from(request: &YoutubeUploadRequest) -> Self {
        YoutubeUploadTaskPayload {
            title: request.title.clone(),
            description: request.description.clone(),
            language: request.language.clone(),
            mime_type: request.mime_type.clone(),
            tags: request.tags.clone(),
            category: request.category,
            render_uri: request.render_uri.clone(),
            thumbnail_uri: request.thumbnail_uri.clone(),
            recording_date: request.recording_date,
            playlist_id: request.playlist_id.clone(),
            playlist_position: request.playlist_position,
            notify_subscribers: request.notify_subscribers,
        }
    }
}

// Create a way to transform a YoutubeUploadRequest and an AppState into a TaskRequest, but TaskRequest is not defined in this file.
impl YoutubeUploadRequest {
    pub fn to_task_request(&self, app_state: &AppState) -> TaskRequest {
        let payload = YoutubeUploadTaskPayload::from(self);

        TaskRequest {
            url: format!("{}/upload/task", app_state.this_api_base_url),
            title: self.task_title.clone(),
            payload: json!(payload),
            data_key: "summary".to_string(),
        }
    }
}

fn default_mime_type() -> String {
    "video/mp4".to_string()
}

fn default_language() -> String {
    "en-US".to_string()
}
