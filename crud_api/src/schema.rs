// @generated automatically by Diesel CLI.

diesel::table! {
    streams (id) {
        id -> Uuid,
        title -> Varchar,
        description -> Text,
        prefix -> Varchar,
        speech_audio_url -> Varchar,
        thumbnail_url -> Varchar,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    video_clips (id) {
        id -> Uuid,
        title -> Varchar,
        description -> Text,
        url -> Varchar,
        duration -> Interval,
        start_time -> Interval,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        stream_id -> Nullable<Uuid>,
    }
}

diesel::joinable!(video_clips -> streams (stream_id));

diesel::allow_tables_to_appear_in_same_query!(streams, video_clips,);
