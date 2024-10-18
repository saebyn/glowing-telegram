use tokio::process::Command;

/// Extract keyframes from a video file.
///
/// # Arguments
/// path - The path to the video file.
/// width - The width of the keyframes, in pixels. The height is automatically calculated to maintain the aspect ratio if -1 is passed.
/// height - The height of the keyframes, in pixels. The width is automatically calculated to maintain the aspect ratio if -1 is passed.
///
/// # Returns
/// A vector of paths to the keyframes.
///
/// # Errors
/// If the keyframe extraction fails, an error is returned.
pub async fn extract(
    temp_dir: &tempfile::TempDir,
    path: &str,
    width: i32,
    height: i32,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    tracing::info!("Extracting keyframes from {}", path);

    let output_path = temp_dir.path().join("frame-%06d.png");

    let mut keyframes_extraction_process = match Command::new("ffmpeg")
        .arg("-hide_banner")
        .arg("-skip_frame")
        .arg("nokey")
        .arg("-i")
        .arg(path)
        .arg("-vsync")
        .arg("0")
        .arg("-vf")
        .arg(format!("scale={width}:{height}"))
        .arg("-f")
        .arg("image2")
        .arg("-frame_pts")
        .arg("true")
        .arg(output_path)
        .spawn()
    {
        Ok(process) => process,
        Err(e) => {
            tracing::error!("Failed to spawn ffmpeg: {}", e);
            return Err(Box::new(e));
        }
    };

    // Wait for the keyframe extraction process to finish.
    let status = keyframes_extraction_process.wait().await?;

    if !status.success() {
        return Err("Failed to extract keyframes".into());
    }

    // Return the path to the keyframes.
    let mut keyframes: Vec<_> = std::fs::read_dir(temp_dir.path())?
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_file())
        .map(|entry| entry.path().to_string_lossy().to_string())
        .collect();

    keyframes.sort();

    Ok(keyframes)
}
