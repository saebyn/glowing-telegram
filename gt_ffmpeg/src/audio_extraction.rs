use std::process::Stdio;

use tokio::process::{ChildStdout, Command};

/// Extracts audio from a video file.
///
/// # Arguments
/// path - The path to the video file.
/// track - The audio track to extract.
///
/// # Returns
/// A `Stdio` object that can be used to read the extracted audio.
///
/// # Errors
/// If the audio extraction fails, an error is returned.
pub fn extract(
    path: &str,
    track: u32,
) -> Result<ChildStdout, Box<dyn std::error::Error>> {
    tracing::info!("Extracting audio from {}", path);

    let audio_extraction = match Command::new("ffmpeg")
        .arg("-hide_banner")
        .arg("-i")
        .arg(path)
        .arg("-map")
        .arg(format!("0:a:{track}"))
        .arg("-acodec")
        .arg("pcm_s16le")
        .arg("-ac")
        .arg("1")
        .arg("-ar")
        .arg("16000")
        .arg("-f")
        .arg("wav")
        .arg("-")
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(process) => process,
        Err(e) => {
            tracing::error!("Failed to spawn ffmpeg: {}", e);
            return Err(Box::new(e));
        }
    };

    audio_extraction
        .stdout
        .map_or_else(|| Err("Failed to extract audio".into()), Ok)
}
