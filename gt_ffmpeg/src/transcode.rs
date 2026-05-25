use tokio::process::Command;

use crate::ffprobe;

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
/// * `offset` - The offset in seconds to treat the input video as starting from.
/// # Returns
/// A list of transcoded files in the temporary directory as a vector of strings representing the file paths.
/// # Errors
/// This function will return an error if the transcoding process fails.
#[tracing::instrument]
pub async fn hls(
    temp_dir: &str,
    input: &str,
    offset: Option<f64>,
) -> Result<Vec<HLSEntry>> {
    tracing::info!("transcode::hls");

    let hls_segment_format = format!("{}/%03d.ts", temp_dir);
    let hls_playlist_path = format!("{}/index.m3u8", temp_dir);
    let input_metadata = ffprobe::probe(input).await?;
    let stereo_audio_stream_indexes = stereo_audio_stream_indexes(&input_metadata);

    let mut command = Command::new("ffmpeg");

    if let Some(offset) = offset {
        // change the PTS of the video to start from the offset
        // so that the HLS segments start from the offset, as these
        // segments will be used to create the HLS playlist separately
        // from the playlist created by FFmpeg
        command.arg("-output_ts_offset").arg(offset.to_string());
    }

    command
        .arg("-hide_banner") // hides FFmpeg banners for cleaner logs
        .arg("-i") // flag to specify input video file
        .arg(input); // input video file

    if let Some(mono_audio_filter) =
        build_mono_audio_filter(&stereo_audio_stream_indexes)
    {
        command
            .arg("-filter_complex")
            .arg(mono_audio_filter)
            .arg("-map")
            .arg("0:v:0")
            .arg("-map")
            .arg("[aout]");
    } else {
        command.arg("-map").arg("0:v:0").arg("-map").arg("0:a:0?");
    }

    command
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
        .arg("44100");

    if stereo_audio_stream_indexes.is_empty() {
        command
            .arg("-ac") // sets number of audio channels
            .arg("2");
    } else {
        command
            .arg("-ac") // sets number of audio channels
            .arg(stereo_audio_stream_indexes.len().to_string());
    }

    command
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

fn stereo_audio_stream_indexes(input_metadata: &ffprobe::FFProbeOutput) -> Vec<u32> {
    input_metadata
        .streams
        .iter()
        .filter(|stream| {
            stream.codec_type == "audio"
                && stream.channels == Some(2)
                && stream.channel_layout.as_deref() == Some("stereo")
        })
        .map(|stream| stream.index)
        .collect()
}

fn build_mono_audio_filter(stereo_audio_stream_indexes: &[u32]) -> Option<String> {
    match stereo_audio_stream_indexes {
        [] => None,
        [stream_index] => {
            Some(format!("[0:{stream_index}]pan=mono|c0=.5*c0+.5*c1[aout]"))
        }
        _ => {
            let mut filter_parts = Vec::with_capacity(
                stereo_audio_stream_indexes.len() + 1,
            );
            let mut mono_labels = Vec::with_capacity(stereo_audio_stream_indexes.len());

            for (output_index, stream_index) in
                stereo_audio_stream_indexes.iter().enumerate()
            {
                let mono_label = format!("a{output_index}");
                filter_parts.push(format!(
                    "[0:{stream_index}]pan=mono|c0=.5*c0+.5*c1[{mono_label}]"
                ));
                mono_labels.push(format!("[{mono_label}]"));
            }

            filter_parts.push(format!(
                "{}amerge=inputs={}[aout]",
                mono_labels.join(""),
                stereo_audio_stream_indexes.len()
            ));

            Some(filter_parts.join(";"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::build_mono_audio_filter;

    #[test]
    fn builds_single_stream_mono_filter() {
        assert_eq!(
            build_mono_audio_filter(&[1]),
            Some("[0:1]pan=mono|c0=.5*c0+.5*c1[aout]".to_string())
        );
    }

    #[test]
    fn builds_multi_stream_mono_filter() {
        assert_eq!(
            build_mono_audio_filter(&[1, 2, 3, 4]),
            Some(
                "[0:1]pan=mono|c0=.5*c0+.5*c1[a0];\
[0:2]pan=mono|c0=.5*c0+.5*c1[a1];\
[0:3]pan=mono|c0=.5*c0+.5*c1[a2];\
[0:4]pan=mono|c0=.5*c0+.5*c1[a3];\
[a0][a1][a2][a3]amerge=inputs=4[aout]"
                    .to_string()
            )
        );
    }
}
