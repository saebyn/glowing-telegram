use serde::{Deserialize, Serialize};

/**
 * An individual segment of a transcript with a start and end duration in ISO 8601 format, and
 * the text of the segment.
 */
#[derive(Serialize, Deserialize, Debug)]
pub struct Segment {
    #[serde(serialize_with = "crate::serde::serialize_duration")]
    pub start: std::time::Duration,
    #[serde(serialize_with = "crate::serde::serialize_duration")]
    pub end: std::time::Duration,
    pub text: String,
}
