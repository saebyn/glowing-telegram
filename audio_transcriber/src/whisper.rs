/// This module provides functionality to transcribe audio files using the Whisper model.
/// It handles downloading audio files from S3, running the Whisper model on them,
/// and processing the transcription results.
use types::Silence;

use aws_sdk_s3::primitives::ByteStream;
use std::process::Stdio;
use thiserror::Error;
use tokio::{io::AsyncWriteExt, process::Command};
use types::Transcription;

#[derive(Error, Debug)]
pub enum AudioTranscriberError {
    #[error("Failed to get object from S3: {0}")]
    S3Error(
        #[from]
        aws_sdk_s3::error::SdkError<
            aws_sdk_s3::operation::get_object::GetObjectError,
        >,
    ),
    #[error("Failed to create temporary directory: {0}")]
    TempDirError(#[from] std::io::Error),
    #[error("Failed to read from bytestream: {0}")]
    ByteStreamError(String),
    #[error("Failed to write to stdin: {0}")]
    StdinWriteError(String),
    #[error("Whisper process failed with status: {0}")]
    WhisperProcessError(std::process::ExitStatus),
    #[error("Failed to read transcription file: {0}")]
    TranscriptionFileError(String),
    #[error("Failed to parse transcription JSON: {0}")]
    JsonParseError(#[from] serde_json::Error),
}

/// Represents the Whisper model to be used for transcription.
////// The available models are:
/// - Tiny
/// - Base
/// - Small
/// - Medium
/// - Large
/// - Turbo
///
/// Each model has different performance characteristics and resource requirements.
/// You can choose the model based on your application's needs and available resources.
#[derive(Debug)]
pub enum WhisperModel {
    Tiny,
    Base,
    Small,
    Medium,
    Large,
    Turbo,
}

/// Configuration options for the Whisper transcription model.
/// # Fields
/// - `model`: The Whisper model to use for transcription.
/// - `model_dir`: Directory where the Whisper model files are located.
/// - `initial_prompt`: Initial prompt to provide to the Whisper model.
/// - `language`: Language code for the transcription.
/// - `clip_timestamps`: Comma-separated list of timestamps to clip the transcription.
/// - `verbose`: Whether to enable verbose output during transcription.
/// # Example
/// ```
/// let options = WhisperOptions {
///     model: WhisperModel::Turbo,
///     model_dir: "/model/".to_string(),
///     initial_prompt: "Transcribe this audio".to_string(),
///     language: "en".to_string(),
///     clip_timestamps: "0:00-0:30".to_string(),
///     verbose: true,
/// };
#[derive(Debug)]
pub struct WhisperOptions {
    pub model: WhisperModel,
    pub model_dir: String,
    pub initial_prompt: String,
    pub language: String,
    pub clip_timestamps: String,
    pub verbose: bool,
}

/// Runs the Whisper transcription model on an audio file stored in an S3 bucket.
///
/// # Parameters
/// - `client`: Reference to an AWS S3 client used to access the bucket.
/// - `input_bucket`: The name of the S3 bucket containing the audio file.
/// - `input_key`: The key (path) of the audio file within the S3 bucket.
/// - `options`: Configuration options for the Whisper model and transcription.
///
/// # Returns
/// Returns a `Result` containing the transcription on success, or an `AudioTranscriberError` on failure.
///
/// # Errors
/// This function can return the following errors:
/// - `AudioTranscriberError::S3Error`: If there is a problem accessing the S3 object.
/// - `AudioTranscriberError::TempDirError`: If a temporary directory cannot be created.
/// - `AudioTranscriberError::ByteStreamError`: If reading from the S3 bytestream fails.
/// - `AudioTranscriberError::StdinWriteError`: If writing to the Whisper process stdin fails.
/// - `AudioTranscriberError::WhisperProcessError`: If the Whisper process fails.
/// - `AudioTranscriberError::TranscriptionFileError`: If reading the transcription file fails.
/// - `AudioTranscriberError::JsonParseError`: If parsing the transcription JSON fails.
pub async fn run_whisper_on_s3_object(
    client: &aws_sdk_s3::Client,
    input_bucket: &str,
    input_key: &str,
    options: WhisperOptions,
) -> Result<Transcription, AudioTranscriberError> {
    let mut object = client
        .get_object()
        .bucket(input_bucket)
        .key(input_key)
        .send()
        .await?;

    run_whisper_on_bytestream(options, &mut object.body).await
}

async fn run_whisper_on_bytestream(
    options: WhisperOptions,
    bytestream: &mut ByteStream,
) -> Result<Transcription, AudioTranscriberError> {
    let temp_dir = tempfile::tempdir()?;

    let mut whisper_detection = build_whisper_command(&temp_dir, options)?;

    let mut whisper_stdin =
        whisper_detection.stdin.take().expect("failed to get stdin");

    let mut byte_count = 0_usize;
    while let Some(bytes) = bytestream.try_next().await.map_err(|err| {
        AudioTranscriberError::ByteStreamError(err.to_string())
    })? {
        let bytes_len = bytes.len();

        whisper_stdin.write_all(&bytes).await.map_err(|err| {
            AudioTranscriberError::StdinWriteError(err.to_string())
        })?;

        byte_count += bytes_len;
        tracing::debug!("Wrote {} bytes to stdin", bytes_len);
    }

    drop(whisper_stdin);
    tracing::info!("Wrote a total of {} bytes to stdin", byte_count);

    let whisper_status = whisper_detection.wait().await?;

    if !whisper_status.success() {
        return Err(AudioTranscriberError::WhisperProcessError(
            whisper_status,
        ));
    }

    let transcription_json = std::fs::read_to_string(
        temp_dir.path().join("-.json"),
    )
    .map_err(|err| {
        AudioTranscriberError::TranscriptionFileError(err.to_string())
    })?;

    tracing::debug!("Transcription JSON: {}", transcription_json);

    Ok(serde_json::from_str::<Transcription>(&transcription_json)?)
}

fn build_whisper_command(
    temp_dir: &tempfile::TempDir,
    options: WhisperOptions,
) -> Result<tokio::process::Child, std::io::Error> {
    let model_str = match options.model {
        WhisperModel::Tiny => "tiny",
        WhisperModel::Base => "base",
        WhisperModel::Small => "small",
        WhisperModel::Medium => "medium",
        WhisperModel::Large => "large",
        WhisperModel::Turbo => "turbo",
    };

    Command::new("whisper")
        .args([
            "--model",
            model_str,
            "--initial_prompt",
            &options.initial_prompt,
            "--model_dir",
            &options.model_dir,
            "--output_format",
            "json",
            "--output_dir",
            temp_dir.path().to_str().unwrap(),
            "--task",
            "transcribe",
            "--device",
            "cuda",
            "--language",
            &options.language,
            "--clip_timestamps",
            &options.clip_timestamps,
            "--verbose",
            if options.verbose { "True" } else { "False" },
            "-",
        ])
        .stdin(Stdio::piped())
        .spawn()
}

/// Converts a list of silence segments into a string of speaking segments for Whisper's `--clip_timestamps` argument.
///
/// # Parameters
/// - `silence_segments`: A slice of `Silence` structs, each representing a period of silence with optional start and end times (in seconds).
/// - `total_duration`: The total duration of the audio (in seconds). If `None`, the last segment will extend to the end of the file.
///
/// # Returns
/// A string representing the speaking segments, formatted as comma-separated pairs of start and end times (e.g., `"0,5,10,15"`).
/// Each pair `"start,end"` indicates a segment of speech between silences. If the end time is omitted, the segment continues to the end of the file.
/// If there are no silence segments, returns `"0"`.
///
/// # Example
/// For silences at 5-10s and 15-20s in a 25s file, returns `"0,5,10,15,20,25"`.
pub fn convert_silence_to_clip_timestamps(
    silence_segments: &[Silence],
    total_duration: Option<f64>,
) -> String {
    if silence_segments.is_empty() {
        return "0".to_string();
    }

    let mut speaking_segments = Vec::new();
    let mut current_time = 0.0;

    // Sort silence segments by start time
    let mut sorted_silence: Vec<_> = silence_segments.iter().collect();
    sorted_silence.sort_by(|a, b| {
        let a_start = a.start.unwrap_or(0.0);
        let b_start = b.start.unwrap_or(0.0);
        a_start
            .partial_cmp(&b_start)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for silence in sorted_silence {
        let silence_start = silence.start.unwrap_or(0.0);
        let silence_end = silence.end.unwrap_or(0.0);

        // Add speaking segment before this silence
        if current_time < silence_start {
            speaking_segments.push(format!("{current_time},{silence_start}"));
        }

        current_time = silence_end;
    }

    // Add final speaking segment if there's time remaining
    if let Some(duration) = total_duration {
        if current_time < duration {
            speaking_segments.push(format!("{current_time},{duration}"));
        }
    } else if current_time > 0.0 {
        // If we don't have total duration, just add a segment from last silence end.
        // If we omit the end of a segment, it will use the end of the file.
        speaking_segments.push(format!("{current_time}"));
    }

    if speaking_segments.is_empty() {
        "0".to_string()
    } else {
        speaking_segments.join(",")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_silence_to_clip_timestamps() {
        // Test case 1: No silence - should return "0"
        let no_silence = vec![];
        assert_eq!(
            convert_silence_to_clip_timestamps(&no_silence, Some(100.0)),
            "0"
        );

        // Test case 2: Single silence in middle - should return speaking segments around it
        let single_silence = vec![Silence {
            start: Some(30.0),
            end: Some(40.0),
        }];
        assert_eq!(
            convert_silence_to_clip_timestamps(&single_silence, Some(100.0)),
            "0,30,40,100"
        );

        // Test case 2.1: Single silence in the middle with no total duration
        assert_eq!(
            convert_silence_to_clip_timestamps(&single_silence, None),
            "0,30,40"
        );

        // Test case 3: Multiple silences
        let multiple_silences = vec![
            Silence {
                start: Some(10.0),
                end: Some(20.0),
            },
            Silence {
                start: Some(50.0),
                end: Some(60.0),
            },
        ];
        assert_eq!(
            convert_silence_to_clip_timestamps(
                &multiple_silences,
                Some(100.0)
            ),
            "0,10,20,50,60,100"
        );

        // Test case 3.1: Multiple silences with no total duration
        assert_eq!(
            convert_silence_to_clip_timestamps(&multiple_silences, None),
            "0,10,20,50,60"
        );

        // Test case 4: Silence at the beginning
        let silence_at_start = vec![Silence {
            start: Some(0.0),
            end: Some(10.0),
        }];
        assert_eq!(
            convert_silence_to_clip_timestamps(&silence_at_start, Some(100.0)),
            "10,100"
        );

        // Test case 4.1: Silence at the beginning with no total duration
        assert_eq!(
            convert_silence_to_clip_timestamps(&silence_at_start, None),
            "10"
        );

        // Test case 5: Silence at the end
        let silence_at_end = vec![Silence {
            start: Some(90.0),
            end: Some(100.0),
        }];
        assert_eq!(
            convert_silence_to_clip_timestamps(&silence_at_end, Some(100.0)),
            "0,90"
        );
    }
}
