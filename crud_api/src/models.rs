use diesel::prelude::*;
use uuid::Uuid;

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schema::streams)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Stream {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub prefix: String,
    pub speech_audio_url: String,
    pub thumbnail_url: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: Option<chrono::NaiveDateTime>,
    pub transcription_task_url: Option<String>,
    pub transcription_segments: Option<serde_json::Value>,
    pub silence_detection_task_url: Option<String>,
    pub silence_segments: Option<serde_json::Value>,
    pub stream_id: Option<String>,
    pub stream_platform: Option<String>,
    pub duration: diesel::pg::data_types::PgInterval,
    pub stream_date: chrono::NaiveDateTime,
}

// then use it for this table model
#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schema::video_clips)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct VideoClip {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub uri: String,
    pub duration: diesel::pg::data_types::PgInterval,
    pub start_time: diesel::pg::data_types::PgInterval,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: Option<chrono::NaiveDateTime>,
    pub stream_id: Option<Uuid>,
    pub audio_bitrate: Option<i32>,
    pub audio_track_count: Option<i32>,
    pub content_type: Option<String>,
    pub filename: Option<String>,
    pub frame_rate: Option<f32>,
    pub height: Option<i32>,
    pub width: Option<i32>,
    pub video_bitrate: Option<i32>,
    pub size: Option<i64>,
    pub last_modified: Option<chrono::NaiveDateTime>,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schema::topics)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Topic {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: Option<chrono::NaiveDateTime>,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schema::episodes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Episode {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub thumbnail_url: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: Option<chrono::NaiveDateTime>,
    pub stream_id: Uuid,
    pub tracks: serde_json::Value,
    pub series_id: Option<Uuid>,
    pub order_index: i32,
    pub render_uri: Option<String>,
    pub is_published: bool,
}

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schema::series)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Series {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub thumbnail_url: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: Option<chrono::NaiveDateTime>,
}

#[derive(Queryable)]
#[diesel(table_name = crate::schema::topic_episodes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TopicEpisode {
    pub id: Uuid,
    pub topic_id: Uuid,
    pub episode_id: Uuid,
}

#[derive(Queryable)]
#[diesel(table_name = crate::schema::topic_series)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct TopicSeries {
    pub id: Uuid,
    pub topic_id: Uuid,
    pub series_id: Uuid,
}
