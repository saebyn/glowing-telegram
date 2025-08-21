//! Integration tests for the audio_transcriber container
//!
//! These tests verify that the audio_transcriber works correctly when built as a container
//! and interacting with AWS services (mocked via LocalStack).

pub mod test_config;

// Re-export common test utilities
pub use test_config::TestConfig;
