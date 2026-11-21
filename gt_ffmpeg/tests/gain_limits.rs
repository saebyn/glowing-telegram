//! Synthetic full-FFmpeg checks for representable authored gain values.
use serde_json::json;
use std::process::Command;
use types::CutList;

fn ffmpeg(args: &[&str]) -> Vec<u8> {
    let result = Command::new("ffmpeg").args(args).output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    result.stdout
}

#[tokio::test]
#[ignore = "requires ffmpeg and ffprobe"]
async fn renders_gain_above_two_and_the_safe_integer_maximum() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("gains.mkv");
    ffmpeg(&[
        "-v",
        "error",
        "-f",
        "lavfi",
        "-i",
        "color=black:s=96x64:r=60:d=2",
        "-f",
        "lavfi",
        "-i",
        "aevalsrc=0.1*sin(2*PI*400*t)|0.2*sin(2*PI*400*t):s=48000:d=2",
        "-f",
        "lavfi",
        "-i",
        "aevalsrc=1e-17*sin(2*PI*400*t)|2e-17*sin(2*PI*400*t):s=48000:d=2",
        "-map",
        "0:v",
        "-map",
        "1:a",
        "-map",
        "2:a",
        "-c:v",
        "ffv1",
        "-c:a",
        "pcm_f32le",
        input.to_str().unwrap(),
    ]);
    for (stream, gain, expected) in [
        (0, 2.1, [0.21, 0.42]),
        (1, 9_007_199_254_740_991.0, [0.09007199, 0.18014398]),
    ] {
        for seconds_points in [false, true] {
            let mut value = json!({
                "version":"1.0.0", "inputMedia":[{"s3Location":"gains.mkv","sections":[{"startFrame":0,"endFrame":120}]}],
                "outputTrack":[{"mediaIndex":0,"sectionIndex":0,"sourceAudio":[
                    {"streamIndex":stream,"sourceChannel":0,"outputChannel":0,"keyframes":[{"frame":0,"volume":gain}]},
                    {"streamIndex":stream,"sourceChannel":1,"outputChannel":1,"keyframes":[{"frame":0,"volume":gain}]}
                ]}]
            });
            if seconds_points {
                for route in value["outputTrack"][0]["sourceAudio"]
                    .as_array_mut()
                    .unwrap()
                {
                    route.as_object_mut().unwrap().remove("keyframes");
                    route["points"] = json!([{"time":0.0,"gain":gain}]);
                }
            }
            let data: CutList = serde_json::from_value(value).unwrap();
            gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
                .await
                .unwrap();
            let output = dir
                .path()
                .join(format!("gain-{stream}-{seconds_points}.mp4"));
            let mut command = gt_ffmpeg::edit::build_ffmpeg_command(
                &data,
                60.0,
                output.to_str().unwrap(),
                (96, 64),
            );
            command.current_dir(dir.path());
            let result = command.output().await.unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
            let audio = ffmpeg(&[
                "-v",
                "error",
                "-i",
                output.to_str().unwrap(),
                "-map",
                "0:a:0",
                "-f",
                "f32le",
                "-acodec",
                "pcm_f32le",
                "-",
            ]);
            for (channel, expected) in expected.into_iter().enumerate() {
                let energy: f32 = (24000..24480)
                    .map(|frame| {
                        let offset = (frame * 2 + channel) * 4;
                        let sample = f32::from_le_bytes(
                            audio[offset..offset + 4].try_into().unwrap(),
                        );
                        sample * sample
                    })
                    .sum();
                let actual = (2.0 * energy / 480.0).sqrt();
                println!(
                    "gain={gain} channel={channel}: expected={expected}, decoded={actual}"
                );
                assert!(
                    (actual - expected).abs() < 0.012,
                    "gain={gain} channel={channel}: expected {expected}, got {actual}"
                );
            }
        }
    }
}

#[tokio::test]
#[ignore = "requires ffmpeg and ffprobe"]
async fn source_gain_and_assembled_mixing_each_apply_once() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("mix.mkv");
    ffmpeg(&[
        "-v",
        "error",
        "-f",
        "lavfi",
        "-i",
        "color=black:s=96x64:r=60:d=2",
        "-f",
        "lavfi",
        "-i",
        "aevalsrc=0.1*sin(2*PI*400*t)|0.2*sin(2*PI*400*t):s=48000:d=2",
        "-map",
        "0:v",
        "-map",
        "1:a",
        "-c:v",
        "ffv1",
        "-c:a",
        "pcm_f32le",
        input.to_str().unwrap(),
    ]);
    for (name, routed, mixed, expected) in [
        ("legacy", false, false, [0.1, 0.2]),
        ("source", true, false, [0.05, 0.1]),
        ("both", true, true, [0.0125, 0.025]),
        ("both-points", true, true, [0.0125, 0.025]),
    ] {
        let mut value = json!({
            "version":"1.0.0", "inputMedia":[{"s3Location":"mix.mkv","sections":[{"startFrame":0,"endFrame":120}]}],
            "outputTrack":[{"mediaIndex":0,"sectionIndex":0}]
        });
        if routed {
            value["outputTrack"][0]["sourceAudio"] = json!([
                {"streamIndex":0,"sourceChannel":0,"outputChannel":0,"keyframes":[{"frame":0,"volume":0.5}]},
                {"streamIndex":0,"sourceChannel":1,"outputChannel":1,"keyframes":[{"frame":0,"volume":0.5}]}
            ]);
            if name == "both-points" {
                for route in value["outputTrack"][0]["sourceAudio"]
                    .as_array_mut()
                    .unwrap()
                {
                    route.as_object_mut().unwrap().remove("keyframes");
                    route["points"] = json!([{"time":0.0,"gain":0.5}]);
                }
            }
        }
        if mixed {
            value["audioMixing"] = json!([
                {"sourceChannel":0,"outputChannel":0,"keyframes":[{"frame":0,"volume":0.25}]},
                {"sourceChannel":1,"outputChannel":1,"keyframes":[{"frame":0,"volume":0.25}]}
            ]);
        }
        let data: CutList = serde_json::from_value(value).unwrap();
        gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
            .await
            .unwrap();
        let output = dir.path().join(format!("{name}.mp4"));
        let result = gt_ffmpeg::edit::build_ffmpeg_command(
            &data,
            60.0,
            output.to_str().unwrap(),
            (96, 64),
        )
        .current_dir(dir.path())
        .output()
        .await
        .unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        let audio = ffmpeg(&[
            "-v",
            "error",
            "-i",
            output.to_str().unwrap(),
            "-map",
            "0:a:0",
            "-f",
            "f32le",
            "-acodec",
            "pcm_f32le",
            "-",
        ]);
        for (channel, expected) in expected.into_iter().enumerate() {
            let energy: f32 = (24000..24480)
                .map(|frame| {
                    let offset = (frame * 2 + channel) * 4;
                    let sample = f32::from_le_bytes(
                        audio[offset..offset + 4].try_into().unwrap(),
                    );
                    sample * sample
                })
                .sum();
            let actual = (2.0 * energy / 480.0).sqrt();
            println!(
                "{name} channel {channel}: expected {expected}, decoded {actual}"
            );
            assert!(
                (actual - expected).abs() < 0.003,
                "{name} channel {channel}: expected {expected}, got {actual}"
            );
        }
    }
}
