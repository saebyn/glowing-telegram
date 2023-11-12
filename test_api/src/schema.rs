// @generated automatically by Diesel CLI.

diesel::table! {
  transcription_app_transcription (id) {
      id -> Int4,
      #[max_length = 100]
      audio_file -> Varchar,
      created_at -> Timestamptz,
      updated_at -> Timestamptz,
      reviewed -> Bool,
      episode_id -> Nullable<Int8>,
  }
}

diesel::table! {
  transcription_app_transcriptionsegment (id) {
      id -> Int4,
      start -> Float8,
      end -> Float8,
      text -> Text,
      created_at -> Timestamptz,
      updated_at -> Timestamptz,
      transcription_id -> Int4,
  }
}

diesel::table! {
  video_app_episode (id) {
      id -> Int8,
      #[max_length = 100]
      title -> Varchar,
      description -> Text,
      created_at -> Timestamptz,
      updated_at -> Timestamptz,
      start -> Nullable<Interval>,
      end -> Nullable<Interval>,
      video_id -> Int8,
  }
}

diesel::table! {
  video_app_topic (id) {
      id -> Int8,
      #[max_length = 100]
      title -> Varchar,
      description -> Text,
      created_at -> Timestamptz,
      updated_at -> Timestamptz,
  }
}

diesel::table! {
  video_app_video (id) {
      id -> Int8,
      #[max_length = 100]
      title -> Varchar,
      description -> Text,
      #[max_length = 100]
      thumbnail -> Nullable<Varchar>,
      created_at -> Timestamptz,
      updated_at -> Timestamptz,
      #[max_length = 100]
      prefix -> Varchar,
      #[max_length = 100]
      speech_audio_track -> Varchar,
      topic_id -> Nullable<Int8>,
  }
}

diesel::table! {
  video_app_videoclip (id) {
      id -> Int8,
      #[max_length = 100]
      title -> Varchar,
      description -> Text,
      #[max_length = 100]
      clip -> Varchar,
      created_at -> Timestamptz,
      updated_at -> Timestamptz,
      video_id -> Int8,
      duration -> Interval,
      start -> Interval,
  }
}

diesel::joinable!(transcription_app_transcription -> video_app_episode (episode_id));
diesel::joinable!(transcription_app_transcriptionsegment -> transcription_app_transcription (transcription_id));
diesel::joinable!(video_app_episode -> video_app_video (video_id));
diesel::joinable!(video_app_video -> video_app_topic (topic_id));
diesel::joinable!(video_app_videoclip -> video_app_video (video_id));

diesel::allow_tables_to_appear_in_same_query!(
    transcription_app_transcription,
    transcription_app_transcriptionsegment,
    video_app_episode,
    video_app_topic,
    video_app_video,
    video_app_videoclip,
);
