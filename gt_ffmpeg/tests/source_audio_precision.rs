//! Lossless captures of the public offline render graph, using synthetic media.
use serde_json::json;
use std::process::Command;
use types::CutList;

fn fixture(dir: &std::path::Path) {
    let fixture = dir.join("fixture.mkv");
    let generated = Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=black:s=96x64:r=60:d=2",
            "-f",
            "lavfi",
            "-i",
            "aevalsrc=0.25|0.5:s=48000:d=2",
            "-map",
            "0:v",
            "-map",
            "1:a",
            "-c:v",
            "ffv1",
            "-c:a",
            "pcm_f64le",
        ])
        .arg(&fixture)
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
}

async fn capture(data: &CutList, dir: &std::path::Path) -> Vec<u8> {
    gt_ffmpeg::edit::validate_source_audio(data, dir, 60.0)
        .await
        .unwrap();
    let output = dir.join("capture.mkv");
    let built = gt_ffmpeg::edit::build_ffmpeg_command(
        data,
        60.0,
        output.to_str().unwrap(),
        (96, 64),
    );
    let mut args: Vec<_> = built
        .as_std()
        .get_args()
        .map(|arg| arg.to_os_string())
        .collect();
    let audio_codec = args.iter().position(|arg| arg == "-c:a").unwrap();
    args[audio_codec + 1] = "pcm_f64le".into();
    let format = args.iter().position(|arg| arg == "-f").unwrap();
    args[format + 1] = "matroska".into();
    let result = Command::new("ffmpeg")
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let decoded = Command::new("ffmpeg")
        .args(["-v", "error", "-i"])
        .arg(&output)
        .args(["-map", "0:a:0", "-c:a", "pcm_f64le", "-f", "f64le", "-"])
        .output()
        .unwrap();
    assert!(
        decoded.status.success(),
        "{}",
        String::from_utf8_lossy(&decoded.stderr)
    );
    decoded.stdout
}

fn sample(decoded: &[u8], sample: usize, channel: usize) -> f64 {
    let offset = (sample * 2 + channel) * 8;
    f64::from_le_bytes(decoded[offset..offset + 8].try_into().unwrap())
}

#[tokio::test]
#[ignore = "requires local ffmpeg and ffprobe"]
async fn representable_faint_gain_survives_lossless_intermediate_graph() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let data: CutList = serde_json::from_value(json!({
        "version":"1.0.0", "inputMedia":[{"s3Location":"fixture.mkv","sections":[{"startFrame":0,"endFrame":120}]}],
        "outputTrack":[{"mediaIndex":0,"sectionIndex":0,"sourceAudio":[
            {"streamIndex":0,"sourceChannel":0,"outputChannel":0,"points":[{"time":0,"gain":1e-44}]},
            {"streamIndex":0,"sourceChannel":1,"outputChannel":1,"points":[{"time":0,"gain":1e-44}]}
        ]}]
    })).unwrap();
    let decoded = capture(&data, dir.path()).await;
    for (channel, expected) in [(0, 2.5e-45), (1, 5e-45)] {
        let actual = sample(&decoded, 24000, channel);
        assert!(
            (actual - expected).abs() < expected * 1e-9,
            "channel {channel}: expected {expected:e}, got {actual:e}"
        );
    }
}

#[tokio::test]
#[ignore = "requires local ffmpeg and ffprobe"]
async fn distinct_subframe_points_are_interpolated_per_sample_and_held() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let data: CutList = serde_json::from_value(json!({
        "version":"1.0.0", "inputMedia":[{"s3Location":"fixture.mkv","sections":[{"startFrame":0,"endFrame":120}]}],
        "outputTrack":[{"mediaIndex":0,"sectionIndex":0,"sourceAudio":[
            {"streamIndex":0,"sourceChannel":0,"outputChannel":0,"points":[{"time":0.001,"gain":0},{"time":0.002,"gain":1}]},
            {"streamIndex":0,"sourceChannel":1,"outputChannel":1,"points":[{"time":0.001,"gain":1},{"time":0.002,"gain":0}]}
        ]}]
    })).unwrap();
    let decoded = capture(&data, dir.path()).await;
    // .001s and .002s correspond exactly to samples 48 and 96 at 48 kHz.
    for (index, left, right) in [
        (0, 0.0, 0.5),
        (48, 0.0, 0.5),
        (60, 0.0625, 0.375),
        (72, 0.125, 0.25),
        (84, 0.1875, 0.125),
        (96, 0.25, 0.0),
        (120, 0.25, 0.0),
    ] {
        for (channel, expected) in [left, right].into_iter().enumerate() {
            let actual = sample(&decoded, index, channel);
            assert!(
                (actual - expected).abs() < 1e-6,
                "sample {index}, channel {channel}: expected {expected}, got {actual}"
            );
        }
    }
}

#[tokio::test]
#[cfg(target_os = "linux")]
#[ignore = "requires local ffmpeg and ffprobe"]
async fn oversized_filter_argument_is_rejected_before_render() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let points: Vec<_> = (0..5000).map(|i| json!({"time": i as f64 / 48000.0, "gain": if i%2 == 0 { 0.0 } else { 1.0 }})).collect();
    let data: CutList = serde_json::from_value(json!({
        "version":"1.0.0", "inputMedia":[{"s3Location":"fixture.mkv","sections":[{"startFrame":0,"endFrame":120}]}],
        "outputTrack":[{"mediaIndex":0,"sectionIndex":0,"sourceAudio":[
            {"streamIndex":0,"sourceChannel":0,"outputChannel":0,"points":points},
            {"streamIndex":0,"sourceChannel":1,"outputChannel":1}
        ]}]
    })).unwrap();
    let error =
        gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
            .await
            .unwrap_err()
            .to_string();
    assert!(
        error.contains("filter") && error.contains("argument"),
        "{error}"
    );
}

#[tokio::test]
#[cfg(target_os = "linux")]
async fn provably_oversized_curves_fail_before_media_is_opened() {
    let points: Vec<_> = (0..50_000)
        .map(|i| json!({"time": i as f64 / 48000.0, "gain": 1.0}))
        .collect();
    let data: CutList = serde_json::from_value(json!({
        "version":"1.0.0", "inputMedia":[{"s3Location":"does-not-exist.mkv","sections":[{"startFrame":0,"endFrame":120}]}],
        "outputTrack":[{"mediaIndex":0,"sectionIndex":0,"sourceAudio":[
            {"streamIndex":0,"sourceChannel":0,"outputChannel":0,"points":points},
            {"streamIndex":0,"sourceChannel":1,"outputChannel":1}
        ]}]
    })).unwrap();
    let error = gt_ffmpeg::edit::validate_source_audio(
        &data,
        std::path::Path::new("."),
        60.0,
    )
    .await
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("filter argument") && error.contains("limit"),
        "{error}"
    );
}

#[tokio::test]
#[ignore = "requires local ffmpeg and ffprobe"]
async fn hundreds_of_ordered_points_render_without_expression_depth_failure() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let points: Vec<_> = (0..=200)
        .map(|i| json!({"time": i as f64 / 1000.0, "gain": i as f64 / 200.0}))
        .collect();
    let data: CutList = serde_json::from_value(json!({
        "version":"1.0.0", "inputMedia":[{"s3Location":"fixture.mkv","sections":[{"startFrame":0,"endFrame":120}]}],
        "outputTrack":[{"mediaIndex":0,"sectionIndex":0,"sourceAudio":[
            {"streamIndex":0,"sourceChannel":0,"outputChannel":0,"points":points},
            {"streamIndex":0,"sourceChannel":1,"outputChannel":1}
        ]}]
    })).unwrap();
    let decoded = capture(&data, dir.path()).await;
    for (index, expected) in [
        (0, 0.0),
        (2400, 0.0625),
        (4800, 0.125),
        (7200, 0.1875),
        (9600, 0.25),
        (10000, 0.25),
    ] {
        let actual = sample(&decoded, index, 0);
        assert!(
            (actual - expected).abs() < 1e-6,
            "sample {index}: expected {expected}, got {actual}"
        );
    }
}

#[tokio::test]
#[cfg(target_os = "linux")]
#[ignore = "requires local ffmpeg and ffprobe"]
async fn large_curve_within_kernel_argument_limit_is_not_rejected_early() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let points: Vec<_> = (0..1400)
        .map(|i| json!({"time":i as f64 / 1000.0,"gain":0.25}))
        .collect();
    let data: CutList = serde_json::from_value(json!({
        "version":"1.0.0", "inputMedia":[{"s3Location":"fixture.mkv","sections":[{"startFrame":0,"endFrame":120}]}],
        "outputTrack":[{"mediaIndex":0,"sectionIndex":0,"sourceAudio":[
            {"streamIndex":0,"sourceChannel":0,"outputChannel":0,"points":points},
            {"streamIndex":0,"sourceChannel":1,"outputChannel":1}
        ]}]
    })).unwrap();
    let decoded = capture(&data, dir.path()).await;
    let actual = sample(&decoded, 24000, 0);
    assert!(
        (actual - 0.0625).abs() < 1e-6,
        "expected 0.0625, got {actual}"
    );
}

#[tokio::test]
#[ignore = "requires local ffmpeg and ffprobe"]
async fn second_based_gain_tracks_the_output_clock_through_trim_and_five_second_fade()
 {
    let dir = tempfile::tempdir().unwrap();
    let generated = Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=black:s=96x64:r=60:d=8",
            "-f",
            "lavfi",
            "-i",
            "aevalsrc=0.25|0.5:s=48000:d=8",
            "-f",
            "lavfi",
            "-i",
            "aevalsrc=0.125|0.25:s=48000:d=8",
            "-map",
            "0:v",
            "-map",
            "1:a",
            "-map",
            "2:a",
            "-c:v",
            "ffv1",
            "-c:a",
            "pcm_f64le",
        ])
        .arg(dir.path().join("fade.mkv"))
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let data: CutList = serde_json::from_value(json!({
        "version":"1.0.0", "inputMedia":[{"s3Location":"fade.mkv","sections":[{"startFrame":60,"endFrame":420},{"startFrame":120,"endFrame":480}]}],
        "outputTrack":[{"mediaIndex":0,"sectionIndex":0}, {"mediaIndex":0,"sectionIndex":1,"transitionIn":{"type":"fade","duration":300},"sourceAudio":[
            {"streamIndex":1,"sourceChannel":0,"outputChannel":0,"points":[{"time":1,"gain":0},{"time":6,"gain":1}]},
            {"streamIndex":1,"sourceChannel":1,"outputChannel":1,"points":[{"time":1,"gain":0},{"time":6,"gain":1}]}
        ]}]
    })).unwrap();
    let decoded = capture(&data, dir.path()).await;
    // FFmpeg's linear acrossfade uses (N-1-n)/N on the outgoing side.
    for n in [0, 1, 60000, 120000, 180000, 239999] {
        let x = n as f64 / 240000.0;
        let outgoing = (239999 - n) as f64 / 240000.0;
        for (channel, first, second) in [(0, 0.25, 0.125), (1, 0.5, 0.25)] {
            let expected = first * outgoing + second * x * x;
            let actual = sample(&decoded, 48000 + n, channel);
            assert!(
                (actual - expected).abs() < 2e-6,
                "n={n}, channel={channel}: expected={expected}, got={actual}"
            );
        }
    }
}

#[tokio::test]
#[ignore = "requires local ffmpeg and ffprobe"]
async fn repeated_trimmed_source_uses_absolute_time_after_cuts() {
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    let data: CutList = serde_json::from_value(json!({
        "version":"1.0.0", "inputMedia":[{"s3Location":"fixture.mkv","sections":[{"startFrame":0,"endFrame":60},{"startFrame":60,"endFrame":120}]}],
        "outputTrack":[{"mediaIndex":0,"sectionIndex":0}, {"mediaIndex":0,"sectionIndex":1,"sourceAudio":[
            {"streamIndex":0,"sourceChannel":0,"outputChannel":0,"points":[{"time":1.001,"gain":0},{"time":1.002,"gain":1}]},
            {"streamIndex":0,"sourceChannel":1,"outputChannel":1}
        ]},{"mediaIndex":0,"sectionIndex":0,"sourceAudio":[
            {"streamIndex":0,"sourceChannel":0,"outputChannel":0,"points":[{"time":2.001,"gain":1},{"time":2.002,"gain":0}]},
            {"streamIndex":0,"sourceChannel":1,"outputChannel":1}
        ]}]
    })).unwrap();
    let decoded = capture(&data, dir.path()).await;
    for (index, expected) in [
        (100, 0.25),
        (48048, 0.0),
        (48072, 0.125),
        (48096, 0.25),
        (96048, 0.25),
        (96072, 0.125),
        (96096, 0.0),
    ] {
        let actual = sample(&decoded, index, 0);
        assert!(
            (actual - expected).abs() < 1e-6,
            "sample {index}: expected {expected}, got {actual}"
        );
    }
}

#[tokio::test]
#[ignore = "requires local ffmpeg and ffprobe"]
async fn malformed_seconds_curves_fail_visibly_instead_of_rendering_defaults()
{
    let dir = tempfile::tempdir().unwrap();
    fixture(dir.path());
    for (points, has_keyframes, expected) in [
        (json!([]), false, "nonempty"),
        (
            json!([{"time":0.002,"gain":1},{"time":0.001,"gain":1}]),
            false,
            "increasing",
        ),
        (
            json!([{"time":0.001,"gain":1},{"time":0.001,"gain":0}]),
            false,
            "increasing",
        ),
        (json!([{"time":-0.001,"gain":1}]), false, "nonnegative"),
        (json!([{"time":0.001,"gain":1e-50}]), false, "gain"),
        (json!([{"time":0.001,"gain":1}]), true, "keyframes"),
    ] {
        let mut value = json!({"version":"1.0.0","inputMedia":[{"s3Location":"fixture.mkv","sections":[{"startFrame":0,"endFrame":120}]}],"outputTrack":[{"mediaIndex":0,"sectionIndex":0,"sourceAudio":[{"streamIndex":0,"sourceChannel":0,"outputChannel":0,"points":points},{"streamIndex":0,"sourceChannel":1,"outputChannel":1}]}]});
        if has_keyframes {
            value["outputTrack"][0]["sourceAudio"][0]["keyframes"] = json!([]);
        }
        let data: CutList = serde_json::from_value(value).unwrap();
        let error =
            gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
                .await
                .unwrap_err()
                .to_string();
        assert!(error.contains(expected), "expected {expected}: {error}");
    }
    let unknown = json!({"version":"1.0.0","inputMedia":[],"outputTrack":[{"mediaIndex":0,"sectionIndex":0,"sourceAudio":[{"streamIndex":0,"sourceChannel":0,"outputChannel":0,"points":[{"time":0,"gain":1,"gaim":0}]}]}]});
    assert!(
        serde_json::from_value::<CutList>(unknown)
            .unwrap_err()
            .to_string()
            .contains("gaim")
    );
}

#[tokio::test]
#[ignore = "requires local ffmpeg and ffprobe"]
async fn delayed_physical_stream_keeps_silence_and_uses_output_timeline_points()
 {
    let dir = tempfile::tempdir().unwrap();
    let generated = Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=black:s=96x64:r=60:d=2",
            "-f",
            "lavfi",
            "-i",
            "aevalsrc=0.1|0.2:s=48000:d=2",
            "-itsoffset",
            "0.5",
            "-f",
            "lavfi",
            "-i",
            "aevalsrc=0.25|0.5:s=48000:d=1.5",
            "-map",
            "0:v",
            "-map",
            "1:a",
            "-map",
            "2:a",
            "-c:v",
            "ffv1",
            "-c:a",
            "pcm_f64le",
        ])
        .arg(dir.path().join("delayed.mkv"))
        .output()
        .unwrap();
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );
    let data: CutList = serde_json::from_value(json!({
        "version":"1.0.0", "inputMedia":[{"s3Location":"delayed.mkv","sections":[{"startFrame":0,"endFrame":120}]}],
        "outputTrack":[{"mediaIndex":0,"sectionIndex":0,"sourceAudio":[
            {"streamIndex":1,"sourceChannel":0,"outputChannel":0,"points":[{"time":0,"gain":0},{"time":2,"gain":1}]},
            {"streamIndex":1,"sourceChannel":1,"outputChannel":1,"points":[{"time":0,"gain":0},{"time":2,"gain":1}]}
        ]}]
    })).unwrap();
    let decoded = capture(&data, dir.path()).await;
    for (index, left, right) in [
        (12000, 0.0, 0.0),
        (36000, 0.09375, 0.1875),
        (60000, 0.15625, 0.3125),
    ] {
        for (channel, expected) in [left, right].into_iter().enumerate() {
            let actual = sample(&decoded, index, channel);
            assert!(
                (actual - expected).abs() < 1e-6,
                "sample {index} channel {channel}: expected {expected}, got {actual}"
            );
        }
    }
}
