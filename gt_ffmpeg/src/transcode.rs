use tokio::process::Command;

type Result<T> =
    std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug)]
pub struct HLSEntry {
    pub path: String,
    pub duration: f64,
}

/// Transcode a video into HLS format.
/// This function will transcode a video into HLS format, and return a list of the transcoded files. It does not output the m3u8 file.
/// # Arguments
/// * `temp_dir` - The temporary directory to store the transcoded files.
/// * `input` - The input video file.
/// # Returns
/// A list of transcoded files in the temporary directory as a vector of strings representing the file paths.
/// # Errors
/// This function will return an error if the transcoding process fails.
#[tracing::instrument]
pub async fn hls(temp_dir: &str, input: &str) -> Result<Vec<HLSEntry>> {
    tracing::info!("transcode::hls");

    let hls_segment_format = format!("{}/%03d.ts", temp_dir);
    let hls_playlist_path = format!("{}/index.m3u8", temp_dir);

    Command::new("ffmpeg")
        .arg("-hide_banner") // hides FFmpeg banners for cleaner logs
        .arg("-i") // flag to specify input video file
        .arg(input)
        .arg("-c:v") // choose video codec
        .arg("libx264") // use x264 for H.264 encoding
        .arg("-preset") // sets encoding speed vs. compression tradeoff
        .arg("veryfast")
        .arg("-tune") // optimizes for specific usage
        .arg("zerolatency") // reduces latency for streaming
        .arg("-crf") // sets constant rate factor for quality
        .arg("30") // 0-51, lower is better quality. 23 is default
        .arg("-vf") // sets video filter
        .arg("scale=-2:480") // scales to 480p while maintaining aspect ratio
        .arg("-c:a") // choose audio codec
        .arg("aac")
        .arg("-b:a") // sets audio bitrate
        .arg("128k")
        .arg("-ar") // sets audio sampling rate
        .arg("44100")
        .arg("-ac") // sets number of audio channels
        .arg("2")
        .arg("-f") // sets output format
        .arg("hls")
        .arg("-hls_time") // duration per segment in seconds
        .arg("4")
        .arg("-hls_list_size") // sets maximum number of segments in playlist
        .arg("0")
        .arg("-hls_segment_filename") // naming pattern for segments
        .arg(hls_segment_format)
        .arg(hls_playlist_path.clone())
        // inherit stdout and stderr from the parent process, so that
        // FFmpeg output is displayed in the console
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .output()
        .await?;

    // read the HLS playlist to get the list of transcoded files and their durations
    let playlist = tokio::fs::read_to_string(hls_playlist_path).await?;

    let mut files = Vec::new();
    let mut next_entry = HLSEntry {
        duration: 0.0,
        path: "".to_string(),
    };
    for line in playlist.lines() {
        if line.starts_with("#EXTINF:") {
            let duration = line
                .trim_start_matches("#EXTINF:")
                .split(',')
                .next()
                .unwrap()
                .parse::<f64>()?;
            next_entry.duration = duration;
        } else if line.ends_with(".ts") {
            next_entry.path = format!("{}/{}", temp_dir, line);
            files.push(next_entry);
            next_entry = HLSEntry {
                path: "".to_string(),
                duration: 0.0,
            };
        }
    }

    Ok(files)
}
