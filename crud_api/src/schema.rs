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
