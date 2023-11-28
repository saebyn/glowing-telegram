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
pub struct VideoClips {
    pub id: uuid::Uuid,
    pub title: String,
    pub description: String,
    pub url: String,
    pub duration: diesel::pg::data_types::PgInterval,
    pub start_time: diesel::pg::data_types::PgInterval,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: Option<chrono::NaiveDateTime>,
    pub stream_id: Option<uuid::Uuid>,
}
