use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::process::Command;

#[derive(Serialize, Deserialize, Debug)]
pub struct Segment {
    #[serde(deserialize_with = "crate::serde::deserialize_duration")]
    #[serde(serialize_with = "crate::serde::serialize_duration")]
    pub start: std::time::Duration,
    #[serde(deserialize_with = "crate::serde::deserialize_duration")]
    #[serde(serialize_with = "crate::serde::serialize_duration")]
    pub end: std::time::Duration,
}

#[derive(Debug)]
pub struct ExtractError(pub &'static str);

// Extracts segments of silence from an audio file.
//
// # Arguments
// path - The path to the audio file.
// track - The audio track to extract.
// noise - The noise tolerance value.
// duration - The minimum detected noise duration.
//
// # Returns
// A vector of segments of silence.
//
// # Errors
// Returns an error if the extraction fails.
#[tracing::instrument]
pub async fn extract(
    path: &str,
    track: u32,
    noise: f64,
    duration: f64,
) -> Result<Vec<Segment>, ExtractError> {
    // This filter logs a message when it detects that the input audio volume is less or equal to a noise tolerance value for a duration greater or equal to the minimum detected noise duration.
    // https://ffmpeg.org/ffmpeg-filters.html#silencedetect
    let command_output = match Command::new("ffmpeg")
        .arg("-hide_banner")
        .arg("-i")
        .arg(path)
        .arg("-map")
        .arg(format!("0:a:{track}"))
        .arg("-af")
        .arg(format!("silencedetect=noise={noise}:duration={duration}",))
        .arg("-f")
        .arg("null")
        .arg("-")
        .output()
        .await
    {
        Ok(output) => output,
        Err(e) => {
            tracing::error!("Failed to spawn ffmpeg: {}", e);
            return Err(ExtractError("Failed to spawn ffmpeg"));
        }
    };

    let command_stderr = String::from_utf8_lossy(&command_output.stderr);

    // handle output status code
    if !command_output.status.success() {
        tracing::error!("ffmpeg error: {}", command_stderr);
    }

    // trace output
    tracing::trace!("ffmpeg output: {}", command_stderr);

    // detect error in filter by looking for "Conversion failed!" in output
    if command_stderr.contains("Conversion failed!") {
        tracing::error!("Conversion failed!");
        return Err(ExtractError("Conversion failed!"));
    }

    let re = match Regex::new(
        r"silence_end: (?<end>\d+(\.\d+)?) \| silence_duration: (?<duration>\d+(\.\d+)?)",
    ) {
        Ok(re) => re,
        Err(e) => {
            tracing::error!("Failed to compile regex: {}", e);
            return Err(ExtractError("Failed to compile regex"));
        }
    };

    let mut segments = Vec::new();

    for cap in re.captures_iter(&command_stderr) {
        let end = match cap["end"].parse::<f64>() {
            Ok(end) => end,
            Err(e) => {
                tracing::error!("Failed to parse end: {}", e);
                return Err(ExtractError("Failed to parse end"));
            }
        };
        let duration = match cap["duration"].parse::<f64>() {
            Ok(duration) => duration,
            Err(e) => {
                tracing::error!("Failed to parse duration: {}", e);
                return Err(ExtractError("Failed to parse duration"));
            }
        };

        let start = end - duration;

        segments.push(Segment {
            start: std::time::Duration::from_secs_f64(start),
            end: std::time::Duration::from_secs_f64(end),
        });
    }

    Ok(segments)
}
