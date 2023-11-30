use diesel::prelude::*;

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schema::streams)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Stream {
    pub id: uuid::Uuid,
    pub title: String,
    pub description: String,
    pub prefix: String,
    pub speech_audio_url: String,
    pub thumbnail_url: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: Option<chrono::NaiveDateTime>,
}

// then use it for this table model
#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schema::video_clips)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct VideoClip {
    pub id: uuid::Uuid,
    pub title: String,
    pub description: String,
    pub uri: String,
    pub duration: diesel::pg::data_types::PgInterval,
    pub start_time: diesel::pg::data_types::PgInterval,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: Option<chrono::NaiveDateTime>,
    pub stream_id: Option<uuid::Uuid>,
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
