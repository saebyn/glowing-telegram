use tokio::process::Command;

/// Per-second peak amplitudes extracted from a WAV file.
#[derive(Debug)]
pub struct AudioPeaks {
    /// One peak amplitude value per second of audio.
    pub peaks: Vec<f64>,
    /// Duration in seconds (equals `peaks.len()` as a float).
    pub duration: f64,
}

/// Extract per-second peak amplitudes from a WAV file.
///
/// Uses ffmpeg's `astats` filter with `reset=16000` (one measurement per 16 000
/// samples, matching the 16 kHz mono WAV produced by `audio_extraction::extract`).
///
/// # Arguments
/// * `wav_path` - Path to the WAV file on disk.
///
/// # Returns
/// An [`AudioPeaks`] containing one peak value per second and the total duration.
///
/// # Errors
/// Returns an error if ffmpeg fails to run or its output cannot be parsed.
#[tracing::instrument]
pub async fn extract(
    wav_path: &str,
) -> Result<AudioPeaks, Box<dyn std::error::Error>> {
    tracing::info!("Extracting audio peaks from {}", wav_path);

    let output = Command::new("ffmpeg")
        .arg("-hide_banner")
        .arg("-i")
        .arg(wav_path)
        .arg("-af")
        .arg("astats=metadata=1:reset=16000")
        .arg("-f")
        .arg("null")
        .arg("-")
        .output()
        .await?;

    let stderr = String::from_utf8_lossy(&output.stderr);

    tracing::trace!("ffmpeg astats output: {}", stderr);

    if !output.status.success() && !stderr.contains("lavfi.astats") {
        tracing::error!("ffmpeg astats failed: {}", stderr);
        return Err("ffmpeg astats failed".into());
    }

    let peaks = parse_peaks(&stderr);

    let duration = peaks.len() as f64;

    tracing::info!(
        "Extracted {} peak values (duration: {}s)",
        peaks.len(),
        duration
    );

    Ok(AudioPeaks { peaks, duration })
}

/// Parse `lavfi.astats.Overall.Peak_amplitude=<value>` lines from ffmpeg stderr.
fn parse_peaks(stderr: &str) -> Vec<f64> {
    stderr
        .lines()
        .filter_map(|line| {
            let marker = "lavfi.astats.Overall.Peak_amplitude=";
            let pos = line.find(marker)?;
            let value_str = line[pos + marker.len()..].trim();
            value_str.parse::<f64>().ok()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::parse_peaks;

    #[test]
    fn test_parse_peaks_basic() {
        let stderr = "\
[Parsed_astats_0 @ 0x...] lavfi.astats.Overall.Peak_amplitude=0.123456\n\
[Parsed_astats_0 @ 0x...] lavfi.astats.Overall.Peak_amplitude=0.654321\n\
[Parsed_astats_0 @ 0x...] lavfi.astats.Overall.Peak_amplitude=0.000000\n";
        let peaks = parse_peaks(stderr);
        assert_eq!(peaks.len(), 3);
        assert!((peaks[0] - 0.123_456).abs() < 1e-6);
        assert!((peaks[1] - 0.654_321).abs() < 1e-6);
        assert!((peaks[2]).abs() < 1e-9);
    }

    #[test]
    fn test_parse_peaks_empty() {
        let peaks = parse_peaks("no matching lines here\n");
        assert!(peaks.is_empty());
    }
}
