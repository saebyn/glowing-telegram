//! Offline end-to-end renderer test; synthetic inputs only.
//! Run with `cargo test -p gt_ffmpeg --test source_audio_envelopes -- --ignored`.

use serde_json::json;
use std::process::Command;
use types::CutList;

fn ffmpeg(args: &[&str]) -> Vec<u8> {
    let output = Command::new("ffmpeg").args(args).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

#[test]
#[ignore = "runs an isolated fake ffprobe subprocess"]
fn incomplete_probe_metadata_is_rejected() {
    if let Ok(case) = std::env::var("GT_FAKE_PROBE_CASE") {
        let data: CutList = serde_json::from_value(json!({
            "version":"1.0.0", "inputMedia":[{"s3Location":"fixture.mkv","sections":[{"startFrame":0,"endFrame":2}]}],
            "outputTrack":[{"mediaIndex":0,"sectionIndex":0,"sourceAudio":[
                {"streamIndex":0,"sourceChannel":0,"outputChannel":0},
                {"streamIndex":0,"sourceChannel":1,"outputChannel":1}
            ]}]
        })).unwrap();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let error = runtime
            .block_on(gt_ffmpeg::edit::validate_source_audio(
                &data,
                std::path::Path::new("."),
                60.0,
            ))
            .unwrap_err()
            .to_string();
        match case.as_str() {
            "video" | "video_nan" => {
                assert!(error.contains("video start"), "{error}")
            }
            "video_pts" | "video_blank_frame" => assert!(
                error.contains("video frame") && error.contains("timestamp"),
                "{error}"
            ),
            "video_decode_error" => assert!(
                error.contains("video") && error.contains("decoder"),
                "{error}"
            ),
            "video_status_error" => assert!(
                error.contains("video") && error.contains("ffprobe"),
                "{error}"
            ),
            "video_no_frames" => assert!(
                error.contains("video frame range")
                    && error.contains("0 frames"),
                "{error}"
            ),
            "audio_leads" => assert!(
                error.contains("audio") && error.contains("before the video"),
                "{error}"
            ),
            "audio_nan" => assert!(
                error.contains("audio") && error.contains("start timestamp"),
                "{error}"
            ),
            "audio_timebase"
            | "audio_timebase_negative_numerator"
            | "audio_timebase_negative_denominator"
            | "audio_timebase_zero" => assert!(
                error.contains("audio") && error.contains("time base"),
                "{error}"
            ),
            "audio_samples_zero" => {
                assert!(error.contains("sample count"), "{error}")
            }
            "audio_pts_mismatch" => assert!(
                error.contains("timestamp disagrees with PTS"),
                "{error}"
            ),
            "audio_pts" => assert!(
                error.contains("audio") && error.contains("PTS"),
                "{error}"
            ),
            "audio_blank_frame" => assert!(
                error.contains("audio") && error.contains("timestamp"),
                "{error}"
            ),
            "audio_decode_error" => assert!(
                error.contains("audio") && error.contains("decoder"),
                "{error}"
            ),
            "audio_status_error" => assert!(
                error.contains("audio") && error.contains("ffprobe"),
                "{error}"
            ),
            "audio_empty" | "audio_timebase_exact" => {
                assert!(error.contains("no timestamped frames"), "{error}")
            }
            _ => assert!(
                error.contains("audio") && error.contains("sample rate"),
                "{error}"
            ),
        }
        return;
    }
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let probe = dir.path().join("ffprobe");
    std::fs::write(&probe, r##"#!/bin/sh
case "$*" in
 *"stream=channels"*) echo '{"streams":[{"channels":2}]}' ;;
 *"stream=start_time,time_base:format=start_time"*)
   if [ "$GT_FAKE_PROBE_CASE" = video ]; then echo '{"streams":[{"time_base":"1/60000"}],"format":{"start_time":"0"}}';
   elif [ "$GT_FAKE_PROBE_CASE" = video_nan ]; then echo '{"streams":[{"start_time":"NaN","time_base":"1/60000"}],"format":{"start_time":"0"}}';
   else echo '{"streams":[{"start_time":"0","time_base":"1/60000"}],"format":{"start_time":"0"}}'; fi ;;
 *"frame=pts_time,best_effort_timestamp_time,nb_samples"*)
   if [ "$GT_FAKE_PROBE_CASE" = audio_pts ]; then echo 'frame|best_effort_timestamp_time=0|nb_samples=1024';
   elif [ "$GT_FAKE_PROBE_CASE" = audio_blank_frame ]; then echo 'frame';
   elif [ "$GT_FAKE_PROBE_CASE" = audio_samples_zero ]; then echo 'frame|pts_time=0|best_effort_timestamp_time=0|nb_samples=0';
   elif [ "$GT_FAKE_PROBE_CASE" = audio_pts_mismatch ]; then echo 'frame|pts_time=0|best_effort_timestamp_time=0.01|nb_samples=1024';
   elif [ "$GT_FAKE_PROBE_CASE" = audio_decode_error ]; then echo 'frame|pts_time=0|best_effort_timestamp_time=0|nb_samples=1024'; echo 'decoder failure' >&2;
   elif [ "$GT_FAKE_PROBE_CASE" = audio_status_error ]; then printf 'frame|pts_time=0|best_effort_timestamp_time=0|nb_samples=1024\nframe|pts_time=0.021333|best_effort_timestamp_time=0.021333|nb_samples=1024\nframe|pts_time=0.042667|best_effort_timestamp_time=0.042667|nb_samples=1024\n'; exit 7; fi ;;
 *"frame=pts_time,best_effort_timestamp_time"*)
   if [ "$GT_FAKE_PROBE_CASE" = video_pts ]; then echo 'frame|best_effort_timestamp_time=0';
   elif [ "$GT_FAKE_PROBE_CASE" = video_blank_frame ]; then echo 'frame';
   elif [ "$GT_FAKE_PROBE_CASE" = video_decode_error ]; then echo 'decoder failure' >&2; printf 'frame|pts_time=0|best_effort_timestamp_time=0\nframe|pts_time=0.016667|best_effort_timestamp_time=0.016667\n';
   elif [ "$GT_FAKE_PROBE_CASE" = video_status_error ]; then printf 'frame|pts_time=0|best_effort_timestamp_time=0\nframe|pts_time=0.016667|best_effort_timestamp_time=0.016667\n'; exit 7;
   elif [ "$GT_FAKE_PROBE_CASE" = video_no_frames ]; then :;
   else printf 'frame|pts_time=0|best_effort_timestamp_time=0\nframe|pts_time=0.016667|best_effort_timestamp_time=0.016667\n'; fi ;;
 *"stream=start_time,sample_rate,time_base"*)
   if [ "$GT_FAKE_PROBE_CASE" = audio_leads ]; then echo '{"streams":[{"start_time":"-1","sample_rate":"48000","time_base":"1/48000"}]}';
   elif [ "$GT_FAKE_PROBE_CASE" = audio_nan ]; then echo '{"streams":[{"start_time":"NaN","sample_rate":"48000","time_base":"1/48000"}]}';
   elif [ "$GT_FAKE_PROBE_CASE" = audio_timebase ]; then echo '{"streams":[{"start_time":"0","sample_rate":"48000","time_base":"-1/-48000"}]}';
   elif [ "$GT_FAKE_PROBE_CASE" = audio_timebase_negative_numerator ]; then echo '{"streams":[{"start_time":"0","sample_rate":"48000","time_base":"-1/48000"}]}';
   elif [ "$GT_FAKE_PROBE_CASE" = audio_timebase_negative_denominator ]; then echo '{"streams":[{"start_time":"0","sample_rate":"48000","time_base":"1/-48000"}]}';
   elif [ "$GT_FAKE_PROBE_CASE" = audio_timebase_zero ]; then echo '{"streams":[{"start_time":"0","sample_rate":"48000","time_base":"0/48000"}]}';
   elif [ "$GT_FAKE_PROBE_CASE" = audio_rate_zero ]; then echo '{"streams":[{"start_time":"0","sample_rate":"0","time_base":"1/48000"}]}';
   elif [ "$GT_FAKE_PROBE_CASE" = audio_rate_nan ]; then echo '{"streams":[{"start_time":"0","sample_rate":"NaN","time_base":"1/48000"}]}';
   elif [ "$GT_FAKE_PROBE_CASE" = audio_timebase_exact ]; then echo '{"streams":[{"start_time":"0","sample_rate":"48000","time_base":"1/1000"}]}';
   elif [ "$GT_FAKE_PROBE_CASE" = audio_pts ] || [ "$GT_FAKE_PROBE_CASE" = audio_empty ] || [ "$GT_FAKE_PROBE_CASE" = audio_blank_frame ] || [ "$GT_FAKE_PROBE_CASE" = audio_decode_error ] || [ "$GT_FAKE_PROBE_CASE" = audio_status_error ] || [ "$GT_FAKE_PROBE_CASE" = audio_samples_zero ] || [ "$GT_FAKE_PROBE_CASE" = audio_pts_mismatch ]; then echo '{"streams":[{"start_time":"0","sample_rate":"48000","time_base":"1/48000"}]}';
   else echo '{"streams":[{"start_time":"0","time_base":"1/48000"}]}'; fi ;;
 *) exit 3 ;;
esac
"##).unwrap();
    std::fs::set_permissions(&probe, std::fs::Permissions::from_mode(0o755))
        .unwrap();
    for case in [
        "video",
        "video_nan",
        "video_pts",
        "video_blank_frame",
        "video_decode_error",
        "video_status_error",
        "video_no_frames",
        "audio",
        "audio_leads",
        "audio_nan",
        "audio_timebase",
        "audio_timebase_negative_numerator",
        "audio_timebase_negative_denominator",
        "audio_timebase_zero",
        "audio_timebase_exact",
        "audio_rate_zero",
        "audio_rate_nan",
        "audio_samples_zero",
        "audio_pts_mismatch",
        "audio_pts",
        "audio_blank_frame",
        "audio_decode_error",
        "audio_status_error",
        "audio_empty",
    ] {
        let result = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "incomplete_probe_metadata_is_rejected",
                "--ignored",
                "--nocapture",
            ])
            .env("PATH", dir.path())
            .env("GT_FAKE_PROBE_CASE", case)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "case={case}: {} {}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

#[tokio::test]
#[ignore = "requires ffprobe and ffmpeg"]
async fn rejects_missing_physical_audio_stream() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("input.mkv");
    ffmpeg(&[
        "-v",
        "error",
        "-f",
        "lavfi",
        "-i",
        "color=black:s=96x64:r=60:d=0.1",
        "-f",
        "lavfi",
        "-i",
        "aevalsrc=0.1|0.2:s=48000:d=0.1",
        "-map",
        "0:v",
        "-map",
        "1:a",
        "-c:v",
        "ffv1",
        "-c:a",
        "pcm_s16le",
        file.to_str().unwrap(),
    ]);
    let mut data: CutList = serde_json::from_value(json!({
        "version":"1.0.0",
        "inputMedia":[{"s3Location":"input.mkv","sections":[{"startFrame":0,"endFrame":6}]}],
        "outputTrack":[{"mediaIndex":0,"sectionIndex":0,"sourceAudio":[
            {"streamIndex":1,"sourceChannel":0,"outputChannel":0},
            {"streamIndex":1,"sourceChannel":1,"outputChannel":1}
        ]}]
    })).unwrap();
    let error =
        gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
            .await
            .unwrap_err()
            .to_string();
    assert!(
        error.contains("outputTrack[0]")
            && error.contains("stream")
            && error.contains('1'),
        "{error}"
    );
    let routes = data.output_track[0].source_audio.as_mut().unwrap();
    routes[0].stream_index = 0;
    routes[1].stream_index = 0;
    routes[1].source_channel = 2;
    let error =
        gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
            .await
            .unwrap_err()
            .to_string();
    assert!(
        error.contains("source channel 2") && error.contains("out of bounds"),
        "{error}"
    );
    let routes = data.output_track[0].source_audio.as_mut().unwrap();
    routes[1].source_channel = 1;
    gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
        .await
        .unwrap();
    data.output_track[0].source_audio.as_mut().unwrap().pop();
    let error =
        gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
            .await
            .unwrap_err()
            .to_string();
    assert!(error.contains("both stereo output channels"), "{error}");
    // Each incomplete shape must fail independently, before any FFmpeg graph
    // can silently replace the absent output channel with silence.
    data.output_track[0].source_audio.as_mut().unwrap()[0].output_channel = 1;
    assert!(
        gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
            .await
            .is_err()
    );
    data.output_track[0].source_audio.as_mut().unwrap().clear();
    assert!(
        gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
            .await
            .is_err()
    );
    data.output_track[0].source_audio = Some(
        serde_json::from_value(json!([
            {"streamIndex":0,"sourceChannel":0,"outputChannel":0},
            {"streamIndex":0,"sourceChannel":1,"outputChannel":1}
        ]))
        .unwrap(),
    );
    data.output_track[0].source_audio.as_mut().unwrap()[0].keyframes =
        Some(vec![types::AudioChannelKeyframe {
            frame: 1,
            volume: 0.5,
        }]);
    gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
        .await
        .unwrap();
    for volume in [
        0.0,
        f32::from_bits(1) as f64,
        2.1,
        9_007_199_254_740_991.0,
        f32::MAX as f64,
    ] {
        data.output_track[0].source_audio.as_mut().unwrap()[0].keyframes =
            Some(vec![types::AudioChannelKeyframe { frame: 0, volume }]);
        gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
            .await
            .unwrap();
    }
    for (frame, volume) in [
        (-1, 0.5),
        (0, -0.1),
        (0, 1e39),
        (0, 1e-50),
        (0, f64::NAN),
        (0, f64::INFINITY),
    ] {
        data.output_track[0].source_audio.as_mut().unwrap()[0].keyframes =
            Some(vec![types::AudioChannelKeyframe { frame, volume }]);
        let error =
            gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
                .await
                .unwrap_err()
                .to_string();
        assert!(error.contains("invalid keyframe"), "{error}");
        if frame == 0 {
            assert!(error.contains("gain"), "{error}");
        }
    }
}

#[tokio::test]
#[ignore = "requires ffprobe and ffmpeg"]
async fn delayed_source_preserves_silence_and_audio_in_its_original_position()
{
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("delayed.mkv");
    ffmpeg(&[
        "-v",
        "error",
        "-f",
        "lavfi",
        "-i",
        "color=black:s=96x64:r=60:d=4",
        "-f",
        "lavfi",
        "-i",
        "aevalsrc=0.1|0.2:s=48000:d=4",
        "-itsoffset",
        "2",
        "-f",
        "lavfi",
        "-i",
        "aevalsrc=0.3*sin(2*PI*400*t)|0.4*sin(2*PI*400*t):s=48000:d=2",
        "-map",
        "0:v",
        "-map",
        "1:a",
        "-map",
        "2:a",
        "-c:v",
        "ffv1",
        "-c:a",
        "pcm_s16le",
        file.to_str().unwrap(),
    ]);
    let data: CutList = serde_json::from_value(json!({
        "version":"1.0.0",
        "inputMedia":[{"s3Location":"delayed.mkv","sections":[{"startFrame":0,"endFrame":240}]}],
        "outputTrack":[{"mediaIndex":0,"sectionIndex":0,"sourceAudio":[
            {"streamIndex":1,"sourceChannel":0,"outputChannel":0},
            {"streamIndex":1,"sourceChannel":1,"outputChannel":1}
        ]}]
    })).unwrap();
    gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
        .await
        .unwrap();
    let output = dir.path().join("output.mp4");
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
    let decoded = ffmpeg(&[
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
    let amplitude = |time: f64, channel: usize| {
        let start = (time * 48000.0) as usize;
        let energy: f32 = (start..start + 480)
            .map(|frame| {
                let offset = (frame * 2 + channel) * 4;
                let sample = f32::from_le_bytes(
                    decoded[offset..offset + 4].try_into().unwrap(),
                );
                sample * sample
            })
            .sum();
        (energy / 480.0 * 2.0).sqrt()
    };
    for (time, channel, expected) in
        [(0.5, 0, 0.0), (0.5, 1, 0.0), (2.5, 0, 0.3), (2.5, 1, 0.4)]
    {
        let actual = amplitude(time, channel);
        println!(
            "delayed t={time} channel={channel}: expected {expected}, got {actual}"
        );
        assert!(
            (actual - expected).abs() < 0.012,
            "t={time}, channel={channel}: expected {expected}, got {actual}"
        );
    }
}

#[tokio::test]
#[ignore = "requires ffmpeg and ffprobe"]
async fn early_ending_audio_keeps_the_remaining_section_silent() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("short.mkv");
    ffmpeg(&[
        "-v",
        "error",
        "-f",
        "lavfi",
        "-i",
        "color=black:s=96x64:r=60:d=4",
        "-f",
        "lavfi",
        "-i",
        "aevalsrc=0.3*sin(2*PI*400*t)|0.4*sin(2*PI*400*t):s=48000:d=2",
        "-map",
        "0:v",
        "-map",
        "1:a",
        "-c:v",
        "ffv1",
        "-c:a",
        "pcm_s16le",
        file.to_str().unwrap(),
    ]);
    let mut data: CutList = serde_json::from_value(json!({
        "version":"1.0.0",
        "inputMedia":[{"s3Location":"short.mkv","sections":[{"startFrame":0,"endFrame":240}]}],
        "outputTrack":[{"mediaIndex":0,"sectionIndex":0,"sourceAudio":[
            {"streamIndex":0,"sourceChannel":0,"outputChannel":0},
            {"streamIndex":0,"sourceChannel":1,"outputChannel":1}
        ]}]
    })).unwrap();
    gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
        .await
        .unwrap();
    let output = dir.path().join("short-output.mp4");
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
    let decoded = ffmpeg(&[
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
    let amplitude = |time: f64, channel: usize| {
        let start = (time * 48000.0) as usize;
        let energy: f32 = (start..start + 480)
            .map(|frame| {
                let offset = (frame * 2 + channel) * 4;
                let sample = f32::from_le_bytes(
                    decoded[offset..offset + 4].try_into().unwrap(),
                );
                sample * sample
            })
            .sum();
        (energy / 480.0 * 2.0).sqrt()
    };
    for (time, channel, expected) in
        [(1.0, 0, 0.3), (1.0, 1, 0.4), (3.0, 0, 0.0), (3.0, 1, 0.0)]
    {
        let actual = amplitude(time, channel);
        println!(
            "early-end t={time} channel={channel}: expected {expected}, got {actual}"
        );
        assert!(
            (actual - expected).abs() < 0.012,
            "t={time}, channel={channel}: expected {expected}, got {actual}"
        );
    }
    data.input_media[0].sections[0].start_frame = 180;
    let output = dir.path().join("entirely-after-audio.mp4");
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
    let decoded = ffmpeg(&[
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
    assert!(
        decoded.len() >= 48000 * 2 * 4 / 2,
        "output audio ended too early: {} bytes",
        decoded.len()
    );
    let max = decoded
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes(b.try_into().unwrap()).abs())
        .fold(0.0_f32, f32::max);
    assert!(max < 0.001, "expected silence, got {max}");
    data.input_media[0].sections[0].end_frame = 300;
    let error =
        gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
            .await
            .unwrap_err()
            .to_string();
    assert!(
        error.contains("video") && error.contains("frame"),
        "{error}"
    );
}

#[tokio::test]
#[ignore = "requires ffmpeg and ffprobe"]
async fn rejects_ambiguous_video_origin_instead_of_moving_leading_audio() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("video-late.mkv");
    ffmpeg(&[
        "-v",
        "error",
        "-itsoffset",
        "2",
        "-f",
        "lavfi",
        "-i",
        "color=black:s=96x64:r=60:d=4",
        "-f",
        "lavfi",
        "-i",
        "aevalsrc=0.1|0.2:s=48000:d=6",
        "-map",
        "0:v",
        "-map",
        "1:a",
        "-c:v",
        "ffv1",
        "-c:a",
        "pcm_s16le",
        file.to_str().unwrap(),
    ]);
    let data: CutList = serde_json::from_value(json!({
        "version":"1.0.0", "inputMedia":[{"s3Location":"video-late.mkv","sections":[{"startFrame":0,"endFrame":240}]}],
        "outputTrack":[{"mediaIndex":0,"sectionIndex":0,"sourceAudio":[
            {"streamIndex":0,"sourceChannel":0,"outputChannel":0},
            {"streamIndex":0,"sourceChannel":1,"outputChannel":1}
        ]}]
    })).unwrap();
    let error =
        gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
            .await
            .unwrap_err()
            .to_string();
    assert!(
        error.contains("video") && error.contains("start"),
        "{error}"
    );
}

#[tokio::test]
#[ignore = "requires ffmpeg and ffprobe"]
async fn rejects_negative_audio_lead_without_shifting_the_timeline() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("negative.mkv");
    ffmpeg(&[
        "-v",
        "error",
        "-f",
        "lavfi",
        "-i",
        "color=black:s=96x64:r=60:d=4",
        "-itsoffset",
        "-1",
        "-f",
        "lavfi",
        "-i",
        "aevalsrc=0.1|0.2:s=48000:d=5",
        "-map",
        "0:v",
        "-map",
        "1:a",
        "-c:v",
        "ffv1",
        "-c:a",
        "pcm_s16le",
        "-avoid_negative_ts",
        "disabled",
        file.to_str().unwrap(),
    ]);
    let probe = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "stream=start_time",
            "-of",
            "json",
            file.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(probe.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&probe.stdout).unwrap();
    assert_eq!(value["streams"][0]["start_time"], "-1.000000");
    let data: CutList = serde_json::from_value(json!({
        "version":"1.0.0", "inputMedia":[{"s3Location":"negative.mkv","sections":[{"startFrame":0,"endFrame":120}]}],
        "outputTrack":[{"mediaIndex":0,"sectionIndex":0,"sourceAudio":[{"streamIndex":0,"sourceChannel":0,"outputChannel":0},{"streamIndex":0,"sourceChannel":1,"outputChannel":1}]}]
    })).unwrap();
    let error =
        gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
            .await
            .unwrap_err()
            .to_string();
    assert!(
        error.contains("video") && error.contains("start"),
        "{error}"
    );
}

#[tokio::test]
#[ignore = "requires ffmpeg and ffprobe"]
async fn rejects_an_internal_audio_timestamp_gap_instead_of_collapsing_it() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("gap.mkv");
    ffmpeg(&[
        "-v",
        "error",
        "-f",
        "lavfi",
        "-i",
        "color=black:s=96x64:r=60:d=4",
        "-f",
        "lavfi",
        "-i",
        "aevalsrc=0.1|0.2:s=48000:d=4",
        "-af",
        "aselect='not(between(t,1,1.06))'",
        "-map",
        "0:v",
        "-map",
        "1:a",
        "-c:v",
        "ffv1",
        "-c:a",
        "pcm_s16le",
        file.to_str().unwrap(),
    ]);
    let data: CutList = serde_json::from_value(json!({
        "version":"1.0.0", "inputMedia":[{"s3Location":"gap.mkv","sections":[{"startFrame":0,"endFrame":240}]}],
        "outputTrack":[{"mediaIndex":0,"sectionIndex":0,"sourceAudio":[
            {"streamIndex":0,"sourceChannel":0,"outputChannel":0},
            {"streamIndex":0,"sourceChannel":1,"outputChannel":1}
        ]}]
    })).unwrap();
    let error =
        gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
            .await
            .unwrap_err()
            .to_string();
    assert!(
        error.contains("audio") && error.contains("timestamp"),
        "{error}"
    );
}

#[tokio::test]
#[ignore = "requires ffmpeg and ffprobe"]
async fn rejects_a_small_but_observable_audio_timestamp_gap() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("small-gap.mkv");
    ffmpeg(&[
        "-v",
        "error",
        "-f",
        "lavfi",
        "-i",
        "color=black:s=96x64:r=60:d=3",
        "-f",
        "lavfi",
        "-i",
        "aevalsrc=0.2*sin(2*PI*400*t)|0.2*sin(2*PI*400*t):s=48000:d=3",
        "-map",
        "0:v",
        "-map",
        "1:a",
        "-c:v",
        "ffv1",
        "-c:a",
        "pcm_s16le",
        "-af",
        "asetpts='if(gte(T,1),PTS+0.0015/TB,PTS)'",
        file.to_str().unwrap(),
    ]);
    let data: CutList = serde_json::from_value(json!({
        "version":"1.0.0", "inputMedia":[{"s3Location":"small-gap.mkv","sections":[{"startFrame":0,"endFrame":180}]}],
        "outputTrack":[{"mediaIndex":0,"sectionIndex":0,"sourceAudio":[{"streamIndex":0,"sourceChannel":0,"outputChannel":0},{"streamIndex":0,"sourceChannel":1,"outputChannel":1}]}]
    })).unwrap();
    let error =
        gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
            .await
            .unwrap_err()
            .to_string();
    assert!(
        error.contains("audio") && error.contains("discontinuous timestamp"),
        "{error}"
    );
}

#[tokio::test]
#[ignore = "requires ffmpeg and ffprobe"]
async fn rejects_video_with_a_gap_in_its_frame_timeline() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("video-gap.mkv");
    ffmpeg(&[
        "-v",
        "error",
        "-f",
        "lavfi",
        "-i",
        "color=black:s=96x64:r=60:d=4",
        "-f",
        "lavfi",
        "-i",
        "aevalsrc=0.1|0.2:s=48000:d=4",
        "-vf",
        "select='not(between(t,1,1.1))'",
        "-fps_mode",
        "vfr",
        "-map",
        "0:v",
        "-map",
        "1:a",
        "-c:v",
        "ffv1",
        "-c:a",
        "pcm_s16le",
        file.to_str().unwrap(),
    ]);
    let data: CutList = serde_json::from_value(json!({
        "version":"1.0.0", "inputMedia":[{"s3Location":"video-gap.mkv","sections":[{"startFrame":0,"endFrame":200}]}],
        "outputTrack":[{"mediaIndex":0,"sectionIndex":0,"sourceAudio":[{"streamIndex":0,"sourceChannel":0,"outputChannel":0},{"streamIndex":0,"sourceChannel":1,"outputChannel":1}]}]
    })).unwrap();
    let error =
        gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
            .await
            .unwrap_err()
            .to_string();
    assert!(
        error.contains("video frame") && error.contains("timestamp"),
        "{error}"
    );
}

#[tokio::test]
#[ignore = "requires ffmpeg, ffprobe and libx264"]
async fn accepts_timestamped_h264_aac_input_without_losing_audio() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("encoded.mp4");
    ffmpeg(&[
        "-v",
        "error",
        "-f",
        "lavfi",
        "-i",
        "color=black:s=96x64:r=60:d=3",
        "-f",
        "lavfi",
        "-i",
        "aevalsrc=0.2*sin(2*PI*400*t)|0.3*sin(2*PI*400*t):s=48000:d=3",
        "-map",
        "0:v",
        "-map",
        "1:a",
        "-c:v",
        "libx264",
        "-c:a",
        "aac",
        file.to_str().unwrap(),
    ]);
    let data: CutList = serde_json::from_value(json!({
        "version":"1.0.0", "inputMedia":[{"s3Location":"encoded.mp4","sections":[{"startFrame":60,"endFrame":180}]}],
        "outputTrack":[{"mediaIndex":0,"sectionIndex":0,"sourceAudio":[{"streamIndex":0,"sourceChannel":0,"outputChannel":0},{"streamIndex":0,"sourceChannel":1,"outputChannel":1}]}]
    })).unwrap();
    gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
        .await
        .unwrap();
    let output = dir.path().join("rendered.mp4");
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
    let decoded = ffmpeg(&[
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
    for (channel, expected) in [(0, 0.2), (1, 0.3)] {
        let energy: f32 = (24000..24480)
            .map(|frame| {
                let offset = (frame * 2 + channel) * 4;
                let sample = f32::from_le_bytes(
                    decoded[offset..offset + 4].try_into().unwrap(),
                );
                sample * sample
            })
            .sum();
        let actual = (energy / 480.0 * 2.0).sqrt();
        assert!(
            (actual - expected).abs() < 0.012,
            "channel {channel}: expected {expected}, got {actual}"
        );
    }
}

#[tokio::test]
#[ignore = "requires ffmpeg and ffprobe"]
async fn nonzero_common_pts_and_source_trims_keep_the_audio_offset() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("offset.mkv");
    ffmpeg(&[
        "-v",
        "error",
        "-f",
        "lavfi",
        "-i",
        "color=black:s=96x64:r=60:d=4",
        "-itsoffset",
        "2",
        "-f",
        "lavfi",
        "-i",
        "aevalsrc=0.3*sin(2*PI*400*t)|0.4*sin(2*PI*400*t):s=48000:d=2",
        "-map",
        "0:v",
        "-map",
        "1:a",
        "-c:v",
        "ffv1",
        "-c:a",
        "pcm_s16le",
        "-output_ts_offset",
        "5",
        input.to_str().unwrap(),
    ]);
    let metadata = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=start_time",
            "-of",
            "json",
            input.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(metadata.status.success());
    let value: serde_json::Value =
        serde_json::from_slice(&metadata.stdout).unwrap();
    assert_eq!(value["format"]["start_time"], "5.000000");
    let mut data: CutList = serde_json::from_value(json!({
        "version":"1.0.0", "inputMedia":[{"s3Location":"offset.mkv","sections":[{"startFrame":0,"endFrame":240}]}],
        "outputTrack":[{"mediaIndex":0,"sectionIndex":0,"sourceAudio":[
            {"streamIndex":0,"sourceChannel":0,"outputChannel":0},
            {"streamIndex":0,"sourceChannel":1,"outputChannel":1}
        ]}]
    })).unwrap();
    for (start, samples) in [
        (0, vec![(0.5, 0.0), (2.5, 0.3)]),
        (60, vec![(0.5, 0.0), (1.5, 0.3)]),
        (180, vec![(0.5, 0.3)]),
    ] {
        data.input_media[0].sections[0].start_frame = start;
        gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
            .await
            .unwrap();
        let output = dir.path().join(format!("cut-{start}.mp4"));
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
        let decoded = ffmpeg(&[
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
        for (time, expected) in samples {
            let begin = (time * 48000.0) as usize;
            let energy: f32 = (begin..begin + 480)
                .map(|frame| {
                    let offset = frame * 2 * 4;
                    let sample = f32::from_le_bytes(
                        decoded[offset..offset + 4].try_into().unwrap(),
                    );
                    sample * sample
                })
                .sum();
            let actual = (energy / 480.0 * 2.0).sqrt();
            println!(
                "source trim frame={start} t={time}: expected {expected}, got {actual}"
            );
            assert!(
                (actual - expected).abs() < 0.012,
                "start={start} t={time}: expected {expected}, got {actual}"
            );
        }
    }
    data.output_track[0].transition_in = Some(
        serde_json::from_value(json!({"type":"fade","duration":60})).unwrap(),
    );
    for route in data.output_track[0].source_audio.as_mut().unwrap() {
        route.keyframes = Some(
            serde_json::from_value(
                json!([{"frame":0,"volume":0.0},{"frame":60,"volume":1.0}]),
            )
            .unwrap(),
        );
    }
    gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
        .await
        .unwrap();
    let output = dir.path().join("first-track-fade.mp4");
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
    let decoded = ffmpeg(&[
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
    let start = 24000_usize;
    let energy: f32 = (start..start + 480)
        .map(|frame| {
            let offset = frame * 2 * 4;
            let sample = f32::from_le_bytes(
                decoded[offset..offset + 4].try_into().unwrap(),
            );
            sample * sample
        })
        .sum();
    let actual = (energy / 480.0 * 2.0).sqrt();
    assert!((actual - 0.15).abs() < 0.012, "expected 0.15, got {actual}");
}

#[tokio::test]
#[ignore = "requires ffmpeg and ffprobe"]
async fn crossfade_keeps_each_source_delays_independent() {
    let dir = tempfile::tempdir().unwrap();
    for (name, delay, source) in [
        ("a.mkv", "2", "0.3*sin(2*PI*400*t)|0.4*sin(2*PI*400*t)"),
        ("b.mkv", "3", "0.1*sin(2*PI*400*t)|0.12*sin(2*PI*400*t)"),
    ] {
        let file = dir.path().join(name);
        ffmpeg(&[
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=black:s=96x64:r=60:d=8",
            "-itsoffset",
            delay,
            "-f",
            "lavfi",
            "-i",
            &format!("aevalsrc={source}:s=48000:d=5"),
            "-map",
            "0:v",
            "-map",
            "1:a",
            "-c:v",
            "ffv1",
            "-c:a",
            "pcm_s16le",
            file.to_str().unwrap(),
        ]);
    }
    let data: CutList = serde_json::from_value(json!({
        "version":"1.0.0", "inputMedia":[
            {"s3Location":"a.mkv","sections":[{"startFrame":0,"endFrame":480}]},
            {"s3Location":"b.mkv","sections":[{"startFrame":0,"endFrame":480}]}
        ],
        "outputTrack":[
            {"mediaIndex":0,"sectionIndex":0,"sourceAudio":[{"streamIndex":0,"sourceChannel":0,"outputChannel":0},{"streamIndex":0,"sourceChannel":1,"outputChannel":1}]},
            {"mediaIndex":1,"sectionIndex":0,"transitionIn":{"type":"fade","duration":300},"sourceAudio":[{"streamIndex":0,"sourceChannel":0,"outputChannel":0},{"streamIndex":0,"sourceChannel":1,"outputChannel":1}]}
        ]
    })).unwrap();
    gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
        .await
        .unwrap();
    let output = dir.path().join("fade.mp4");
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
    let decoded = ffmpeg(&[
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
    for (time, channel, expected) in [
        (1.0, 0, 0.0),
        (4.0, 0, 0.24),
        (4.0, 1, 0.32),
        (5.5, 0, 0.15),
        (6.5, 0, 0.16),
        (6.5, 1, 0.204),
        (8.1, 0, 0.1),
    ] {
        let start = (time * 48000.0) as usize;
        let energy: f32 = (start..start + 480)
            .map(|frame| {
                let offset = (frame * 2 + channel) * 4;
                let sample = f32::from_le_bytes(
                    decoded[offset..offset + 4].try_into().unwrap(),
                );
                sample * sample
            })
            .sum();
        let actual = (energy / 480.0 * 2.0).sqrt();
        println!(
            "independent fade t={time} channel={channel}: expected {expected}, got {actual}"
        );
        assert!(
            (actual - expected).abs() < 0.012,
            "t={time} channel={channel}: expected {expected}, got {actual}"
        );
    }
}

#[tokio::test]
#[ignore = "requires ffmpeg and ffprobe"]
async fn routed_content_keeps_intro_overlay_and_outro_fade() {
    let dir = tempfile::tempdir().unwrap();
    for (name, color, seconds, audio) in [
        ("intro.mkv", "blue", "1", "0.05"),
        ("content.mkv", "red", "8", "0.3"),
        ("outro.mkv", "green", "8", "0.1"),
        ("overlay.mkv", "white", "1", "0.0"),
    ] {
        let path = dir.path().join(name);
        ffmpeg(&[
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            &format!("color={color}:s=96x64:r=60:d={seconds}"),
            "-f",
            "lavfi",
            "-i",
            &format!(
                "aevalsrc={audio}*sin(2*PI*400*t)|{audio}*sin(2*PI*400*t):s=48000:d={seconds}"
            ),
            "-map",
            "0:v",
            "-map",
            "1:a",
            "-c:v",
            "ffv1",
            "-c:a",
            "pcm_s16le",
            path.to_str().unwrap(),
        ]);
    }
    let data: CutList = serde_json::from_value(json!({
        "version":"1.0.0", "inputMedia":[
            {"s3Location":"intro.mkv","sections":[{"startFrame":0,"endFrame":60}]},
            {"s3Location":"content.mkv","sections":[{"startFrame":0,"endFrame":480}]},
            {"s3Location":"outro.mkv","sections":[{"startFrame":0,"endFrame":480}]},
            {"s3Location":"overlay.mkv","sections":[{"startFrame":0,"endFrame":60}]}
        ], "outputTrack":[
            {"mediaIndex":0,"sectionIndex":0},
            {"mediaIndex":1,"sectionIndex":0,"sourceAudio":[
                {"streamIndex":0,"sourceChannel":0,"outputChannel":0,"keyframes":[{"frame":0,"volume":0.5}]},
                {"streamIndex":0,"sourceChannel":1,"outputChannel":1,"keyframes":[{"frame":0,"volume":0.5}]}
            ]},
            {"mediaIndex":2,"sectionIndex":0,"transitionIn":{"type":"fade","duration":300}}
        ], "overlayTracks":[{"startFrame":0,"mediaIndex":3,"sectionIndex":0,"type":"alpha"}]
    })).unwrap();
    gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
        .await
        .unwrap();
    let output = dir.path().join("branded.mp4");
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
    let decoded = ffmpeg(&[
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
    for (time, expected) in
        [(0.5, 0.05), (1.5, 0.15), (6.5, 0.125), (10.0, 0.1)]
    {
        let start = (time * 48000.0) as usize;
        let energy: f32 = (start..start + 480)
            .map(|frame| {
                let offset = frame * 2 * 4;
                let sample = f32::from_le_bytes(
                    decoded[offset..offset + 4].try_into().unwrap(),
                );
                sample * sample
            })
            .sum();
        let actual = (energy / 480.0 * 2.0).sqrt();
        assert!(
            (actual - expected).abs() < 0.012,
            "t={time}: expected {expected}, got {actual}"
        );
    }
    for (time, expected) in [
        ("0.5", [255_u8, 255, 255]),
        ("1.5", [255, 0, 0]),
        ("10.0", [0, 128, 0]),
    ] {
        let pixel = ffmpeg(&[
            "-v",
            "error",
            "-ss",
            time,
            "-i",
            output.to_str().unwrap(),
            "-frames:v",
            "1",
            "-vf",
            "scale=1:1",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "-",
        ]);
        assert_eq!(pixel.len(), 3);
        for (actual, expected) in pixel.into_iter().zip(expected) {
            assert!(
                (i16::from(actual) - i16::from(expected)).abs() < 20,
                "t={time}: unexpected pixel channel {actual} expected {expected}"
            );
        }
    }
}

#[tokio::test]
#[ignore = "requires ffmpeg and ffprobe"]
async fn rejects_a_short_legacy_intro_that_would_shift_corrected_audio() {
    let dir = tempfile::tempdir().unwrap();
    for (name, video_duration, audio_duration) in
        [("short-intro.mkv", "1", "0.5"), ("body.mkv", "3", "3")]
    {
        let file = dir.path().join(name);
        ffmpeg(&[
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            &format!("color=black:s=96x64:r=60:d={video_duration}"),
            "-f",
            "lavfi",
            "-i",
            &format!("aevalsrc=0.1|0.2:s=48000:d={audio_duration}"),
            "-map",
            "0:v",
            "-map",
            "1:a",
            "-c:v",
            "ffv1",
            "-c:a",
            "pcm_s16le",
            file.to_str().unwrap(),
        ]);
    }
    let data: CutList = serde_json::from_value(json!({
        "version":"1.0.0", "inputMedia":[
            {"s3Location":"short-intro.mkv","sections":[{"startFrame":0,"endFrame":60}]},
            {"s3Location":"body.mkv","sections":[{"startFrame":0,"endFrame":180}]}
        ], "outputTrack":[
            {"mediaIndex":0,"sectionIndex":0},
            {"mediaIndex":1,"sectionIndex":0,"sourceAudio":[{"streamIndex":0,"sourceChannel":0,"outputChannel":0},{"streamIndex":0,"sourceChannel":1,"outputChannel":1}]}
        ]
    })).unwrap();
    let error =
        gt_ffmpeg::edit::validate_source_audio(&data, dir.path(), 60.0)
            .await
            .unwrap_err()
            .to_string();
    assert!(
        error.contains("outputTrack[0]")
            && error.contains("audio")
            && error.contains("cover"),
        "{error}"
    );
}

#[tokio::test]
#[ignore = "requires ffmpeg"]
async fn source_envelopes_follow_output_time_through_crossfade() {
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first.mkv");
    let second = dir.path().join("second.mkv");
    for (path, a, b) in [
        (
            &first,
            "0.1*sin(2*PI*400*t)|0.2*sin(2*PI*400*t)",
            "0.3*sin(2*PI*400*t)|0.4*sin(2*PI*400*t)",
        ),
        (
            &second,
            "0.08*sin(2*PI*400*t)|0.12*sin(2*PI*400*t)",
            "0.25*sin(2*PI*400*t)|0.35*sin(2*PI*400*t)",
        ),
    ] {
        ffmpeg(&[
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=black:s=96x64:r=60:d=9",
            "-f",
            "lavfi",
            "-i",
            &format!("aevalsrc={a}:s=48000:d=9"),
            "-f",
            "lavfi",
            "-i",
            &format!("aevalsrc={b}:s=48000:d=9"),
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
            path.to_str().unwrap(),
        ]);
    }
    // First section trims one source second. The second begins at output t=3,
    // overlapping the first for five seconds. Select different audio streams.
    let mut data: CutList = serde_json::from_value(json!({
        "version":"1.0.0",
        "inputMedia":[
            {"s3Location":first.to_str().unwrap(),"sections":[{"startFrame":60,"endFrame":540}]},
            {"s3Location":second.to_str().unwrap(),"sections":[{"startFrame":0,"endFrame":480}]}
        ],
        "outputTrack":[
            {"mediaIndex":0,"sectionIndex":0,"sourceAudio":[
                {"streamIndex":0,"sourceChannel":0,"outputChannel":0,"keyframes":[{"frame":0,"volume":0.25}]},
                {"streamIndex":0,"sourceChannel":1,"outputChannel":1,"keyframes":[{"frame":0,"volume":0.25}]},
                {"streamIndex":1,"sourceChannel":0,"outputChannel":0,"keyframes":[{"frame":0,"volume":0.5}]},
                {"streamIndex":1,"sourceChannel":1,"outputChannel":1,"keyframes":[{"frame":0,"volume":0.5}]}
            ]},
            {"mediaIndex":1,"sectionIndex":0,"transitionIn":{"type":"fade","duration":300},"sourceAudio":[
                {"streamIndex":0,"sourceChannel":0,"outputChannel":0,"keyframes":[{"frame":180,"volume":0.0},{"frame":480,"volume":1.0}]},
                {"streamIndex":0,"sourceChannel":1,"outputChannel":1,"keyframes":[{"frame":180,"volume":0.0},{"frame":480,"volume":1.0}]},
                {"streamIndex":1,"sourceChannel":0,"outputChannel":0,"keyframes":[{"frame":0,"volume":0.1}]},
                {"streamIndex":1,"sourceChannel":1,"outputChannel":1,"keyframes":[{"frame":0,"volume":0.1}]}
            ]}
        ]
    })).unwrap();
    async fn render(data: &CutList, output: &std::path::Path) -> Vec<u8> {
        gt_ffmpeg::edit::validate_source_audio(
            data,
            std::path::Path::new("."),
            60.0,
        )
        .await
        .unwrap();
        let mut command = gt_ffmpeg::edit::build_ffmpeg_command(
            data,
            60.0,
            output.to_str().unwrap(),
            (96, 64),
        );
        let result = command.output().await.unwrap();
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        ffmpeg(&[
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
        ])
    }
    fn amplitude(decoded: &[u8], time: f64, channel: usize) -> f32 {
        let start = (time * 48000.0) as usize;
        let energy: f32 = (start..start + 480)
            .map(|frame| {
                let offset = (frame * 2 + channel) * 4;
                let sample = f32::from_le_bytes(
                    decoded[offset..offset + 4].try_into().unwrap(),
                );
                sample * sample
            })
            .sum();
        (energy / 480.0 * 2.0).sqrt()
    }
    let output = dir.path().join("output.mp4");
    let decoded = render(&data, &output).await;
    // Acrossfade is linear: at its midpoint each source contributes 1/2.
    for (time, channel, expected) in [
        (1.0, 0, 0.175),
        (1.0, 1, 0.25),
        (3.0, 0, 0.175),
        (3.0, 1, 0.25),
        (5.5, 0, 0.12),
        (5.5, 1, 0.1725),
        (8.0, 0, 0.105),
        (8.0, 1, 0.155),
        (9.0, 0, 0.105),
        (9.0, 1, 0.155),
    ] {
        let actual = amplitude(&decoded, time, channel);
        println!(
            "fade t={time} channel={channel}: expected {expected}, got {actual}"
        );
        assert!(
            (actual - expected).abs() < 0.012,
            "t={time} channel={channel}: expected {expected}, got {actual}"
        );
    }
    // A hard cut uses the second section's output-time gain at t=8, not its
    // local t=0 or its source timestamps.
    data.output_track[1].transition_in = None;
    let decoded = render(&data, &dir.path().join("cut.mp4")).await;
    for (time, channel, expected) in [
        (7.9, 0, 0.175),
        (7.9, 1, 0.25),
        (8.02, 0, 0.105),
        (8.02, 1, 0.155),
    ] {
        let actual = amplitude(&decoded, time, channel);
        println!(
            "cut t={time} channel={channel}: expected {expected}, got {actual}"
        );
        assert!(
            (actual - expected).abs() < 0.012,
            "cut t={time} channel={channel}: expected {expected}, got {actual}"
        );
    }
    // No route data preserves the legacy first audio stream on each input.
    data.output_track[0].source_audio = None;
    data.output_track[1].source_audio = None;
    let decoded = render(&data, &dir.path().join("legacy.mp4")).await;
    for (time, channel, expected) in
        [(1.0, 0, 0.1), (1.0, 1, 0.2), (9.0, 0, 0.08), (9.0, 1, 0.12)]
    {
        let actual = amplitude(&decoded, time, channel);
        println!(
            "legacy t={time} channel={channel}: expected {expected}, got {actual}"
        );
        assert!(
            (actual - expected).abs() < 0.012,
            "legacy t={time} channel={channel}: expected {expected}, got {actual}"
        );
    }
    data.output_track[1].transition_in = Some(
        serde_json::from_value(json!({"type":"fade","duration":300})).unwrap(),
    );
    let decoded = render(&data, &dir.path().join("legacy-fade.mp4")).await;
    for (channel, expected) in [(0, 0.09), (1, 0.16)] {
        let actual = amplitude(&decoded, 5.5, channel);
        println!(
            "legacy fade t=5.5 channel={channel}: expected {expected}, got {actual}"
        );
        assert!((actual - expected).abs() < 0.012);
    }
}
