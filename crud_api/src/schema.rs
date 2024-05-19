// @generated automatically by Diesel CLI.

diesel::table! {
    episodes (id) {
        id -> Uuid,
        title -> Varchar,
        description -> Text,
        thumbnail_url -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        stream_id -> Uuid,
        tracks -> Jsonb,
        series_id -> Nullable<Uuid>,
        order_index -> Int4,
        render_uri -> Nullable<Text>,
        is_published -> Bool,
    }
}

diesel::table! {
    series (id) {
        id -> Uuid,
        title -> Varchar,
        description -> Text,
        thumbnail_url -> Nullable<Varchar>,
        created_at -> Timestamptz,
        updated_at -> Nullable<Timestamptz>,
        playlist_id -> Nullable<Text>,
    }
}

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
        transcription_task_url -> Nullable<Text>,
        transcription_segments -> Nullable<Jsonb>,
        silence_detection_task_url -> Nullable<Text>,
        silence_segments -> Nullable<Jsonb>,
        #[max_length = 255]
        stream_id -> Nullable<Varchar>,
        #[max_length = 255]
        stream_platform -> Nullable<Varchar>,
        duration -> Interval,
        stream_date -> Timestamptz,
        series_id -> Nullable<Uuid>,
    }
}

diesel::table! {
    topic_episodes (id) {
        id -> Uuid,
        topic_id -> Uuid,
        episode_id -> Uuid,
    }
}

diesel::table! {
    topic_series (id) {
        id -> Uuid,
        topic_id -> Uuid,
        series_id -> Uuid,
    }
}

diesel::table! {
    topics (id) {
        id -> Uuid,
        title -> Varchar,
        description -> Text,
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

diesel::joinable!(episodes -> series (series_id));
diesel::joinable!(episodes -> streams (stream_id));
diesel::joinable!(streams -> series (series_id));
diesel::joinable!(topic_episodes -> episodes (episode_id));
diesel::joinable!(topic_episodes -> topics (topic_id));
diesel::joinable!(topic_series -> series (series_id));
diesel::joinable!(topic_series -> topics (topic_id));
diesel::joinable!(video_clips -> streams (stream_id));

diesel::allow_tables_to_appear_in_same_query!(
    episodes,
    series,
    streams,
    topic_episodes,
    topic_series,
    topics,
    video_clips,
);
