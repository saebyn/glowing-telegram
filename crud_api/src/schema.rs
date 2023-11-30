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
        uri -> Varchar,
        duration -> Interval,
        start_time -> Interval,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        stream_id -> Nullable<Uuid>,
        audio_bitrate -> Nullable<Int4>,
        audio_track_count -> Nullable<Int4>,
        #[max_length = 255]
        content_type -> Nullable<Varchar>,
        #[max_length = 255]
        filename -> Nullable<Varchar>,
        frame_rate -> Nullable<Float4>,
        height -> Nullable<Int4>,
        width -> Nullable<Int4>,
        video_bitrate -> Nullable<Int4>,
        size -> Nullable<Int8>,
        last_modified -> Nullable<Timestamp>,
    }
}

diesel::joinable!(video_clips -> streams (stream_id));

diesel::allow_tables_to_appear_in_same_query!(streams, video_clips,);
