use tokio::process::Command;

/// Per-second peak amplitudes extracted from a WAV file.
#[derive(Debug)]
pub struct AudioPeaks {
    /// One peak amplitude (linear, 0.0..=1.0) per second of audio.
    pub peaks: Vec<f64>,
    /// Duration in seconds (typically equals `peaks.len()`).
    pub duration: f64,
}

/// Extract per-second peak amplitudes from a WAV file.
///
/// Uses the ffmpeg filter chain:
///   `asetnsamples=n=16000,astats=metadata=1:reset=1,ametadata=print:file=-`
///
/// `asetnsamples` rechunks the audio into exactly 16 000-sample frames
/// (= 1 second at 16 kHz mono), then `astats` with `reset=1` computes fresh
/// stats for each frame. `ametadata=print:file=-` writes the lavfi metadata
/// to stdout, one block per frame.
///
/// The peak value extracted is `lavfi.astats.1.Peak_level` (dB).
///
/// # Arguments
/// * `wav_path` - Path to the 16 kHz mono WAV file on disk.
///
/// # Returns
/// An [`AudioPeaks`] containing one peak dB value per second and the total duration.
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
        .arg("asetnsamples=n=16000,astats=metadata=1:reset=1,ametadata=print:file=-")
        .arg("-f")
        .arg("null")
        .arg("-")
        .output()
        .await?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::error!("ffmpeg astats failed: {}", stderr);
        return Err("ffmpeg astats failed".into());
    }

    // ametadata writes to stdout (file=-)
    let stdout = String::from_utf8_lossy(&output.stdout);

    tracing::trace!("ffmpeg ametadata output: {}", stdout);

    let peaks: Vec<f64> = parse_peaks(&stdout)
        .into_iter()
        .map(db_to_amplitude)
        .collect();

    let duration = peaks.len() as f64;

    tracing::info!(
        "Extracted {} peak values (duration: {}s)",
        peaks.len(),
        duration
    );

    Ok(AudioPeaks { peaks, duration })
}

/// Convert a dBFS value to a linear amplitude in [0.0, 1.0].
///
/// Formula: `amplitude = 10^(db / 20)`, clamped to [0.0, 1.0].
/// Silence (e.g. −84 dB or lower) maps to ~0.0; 0 dBFS maps to 1.0.
fn db_to_amplitude(db: f64) -> f64 {
    (10_f64.powf(db / 20.0)).clamp(0.0, 1.0)
}

/// Parse `lavfi.astats.1.Peak_level=<value>` lines from ametadata stdout output.
fn parse_peaks(stdout: &str) -> Vec<f64> {
    stdout
        .lines()
        .filter_map(|line| {
            let marker = "lavfi.astats.1.Peak_level=";
            let pos = line.find(marker)?;
            let value_str = line[pos + marker.len()..].trim();
            value_str.parse::<f64>().ok()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{db_to_amplitude, parse_peaks};

    #[test]
    fn test_parse_peaks_basic() {
        let stdout = "\
frame:0    pts:0       pts_time:0\n\
lavfi.astats.1.Peak_level=-18.063656\n\
frame:1    pts:16000   pts_time:1\n\
lavfi.astats.1.Peak_level=-21.500000\n\
frame:2    pts:32000   pts_time:2\n\
lavfi.astats.1.Peak_level=0.000000\n";
        let peaks = parse_peaks(stdout);
        assert_eq!(peaks.len(), 3);
        assert!((peaks[0] - -18.063_656).abs() < 1e-4);
        assert!((peaks[1] - -21.5).abs() < 1e-6);
        assert!((peaks[2]).abs() < 1e-9);
    }

    #[test]
    fn test_parse_peaks_empty() {
        let peaks = parse_peaks("no matching lines here\n");
        assert!(peaks.is_empty());
    }

    #[test]
    fn test_db_to_amplitude_zero_dbfs() {
        // 0 dBFS = full scale = 1.0
        assert!((db_to_amplitude(0.0) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn test_db_to_amplitude_silence() {
        // -84 dBFS ≈ 0.00006, effectively silence
        let a = db_to_amplitude(-84.0);
        assert!(a < 0.001);
        assert!(a >= 0.0);
    }

    #[test]
    fn test_db_to_amplitude_minus_6() {
        // -6 dBFS ≈ 0.501 (half amplitude)
        let a = db_to_amplitude(-6.0);
        assert!((a - 0.501_187).abs() < 1e-5);
    }

    #[test]
    fn test_db_to_amplitude_clamped() {
        // Values above 0 dBFS (e.g. due to clipping) clamp to 1.0
        assert!((db_to_amplitude(3.0) - 1.0).abs() < 1e-9);
    }
}
