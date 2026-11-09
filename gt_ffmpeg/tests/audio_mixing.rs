//! FFmpeg-backed audio mixing check. Run explicitly with:
//! cargo test -p gt_ffmpeg --test audio_mixing -- --ignored

use serde_json::json;
use types::CutList;

#[test]
#[ignore = "requires an installed ffmpeg executable"]
fn mixing_routes_channels_and_interpolates_on_the_output_timeline() {
    let data: CutList = serde_json::from_value(json!({
        "version": "1.0.0",
        "inputMedia": [{"s3Location": "a.mkv", "sections": [{"startFrame": 0, "endFrame": 120}]}],
        "outputTrack": [{"mediaIndex": 0, "sectionIndex": 0}],
        "audioMixing": [
            {"sourceChannel": 0, "outputChannel": 1, "keyframes": [
                {"frame": 60, "volume": 1.0}, {"frame": 0, "volume": 0.0}
            ]},
            {"sourceChannel": 1, "outputChannel": 0, "keyframes": [
                {"frame": 0, "volume": 0.5}
            ]},
            {"sourceChannel": 0, "outputChannel": 0, "keyframes": [
                {"frame": 0, "volume": 0.5}
            ]}
        ]
    }))
    .unwrap();
    let command = gt_ffmpeg::edit::build_ffmpeg_command(&data, 60.0, "out.mp4", (96, 64));
    let args: Vec<_> = command.as_std().get_args().collect();
    let index = args.iter().position(|arg| *arg == "-filter_complex").unwrap();
    let filter = args[index + 1].to_str().unwrap();
    let (_, mixing) = filter.split_once(";[a255]asplit=").expect("mixing graph");
    let graph = format!("[0:a]anull[a255];[a255]asplit={mixing}");
    let output = std::process::Command::new("ffmpeg")
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "aevalsrc=0.1|0.2:s=48000:d=2",
            "-filter_complex",
            &graph,
            "-map",
            "[a255]",
            "-f",
            "f32le",
            "-acodec",
            "pcm_f32le",
            "-",
        ])
        .output()
        .expect("ffmpeg must be installed for this integration test");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let sample = |time: f64, channel: usize| {
        let offset = ((time * 48000.0) as usize * 2 + channel) * 4;
        f32::from_le_bytes(output.stdout[offset..offset + 4].try_into().unwrap())
    };
    assert!((sample(0.5, 0) - 0.15).abs() < 0.002);
    assert!((sample(0.5, 1) - 0.05).abs() < 0.003);
    assert!((sample(1.5, 1) - 0.1).abs() < 0.002);
}
