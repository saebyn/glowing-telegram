//! Legacy Episode and offline CutList deserialization must not drop corrections.
use serde_json::{Value, json};
use types::{CutList, Episode};

fn valid_cut_list() -> Value {
    json!({
        "version": "1.0.0",
        "inputMedia": [{"s3Location":"fixture.mkv", "sections":[{"startFrame":0,"endFrame":120}]}],
        "outputTrack": [{"mediaIndex":0,"sectionIndex":0}]
    })
}

#[test]
fn null_seconds_points_are_rejected_by_direct_and_episode_deserializers() {
    let mut malformed = valid_cut_list();
    malformed["outputTrack"][0]["sourceAudio"] = json!([
        {"streamIndex":0,"sourceChannel":0,"outputChannel":0,"points":null},
        {"streamIndex":0,"sourceChannel":1,"outputChannel":1}
    ]);
    assert!(serde_json::from_value::<CutList>(malformed.clone()).is_err());
    assert!(
        serde_json::from_value::<Episode>(json!({"cut_list":malformed}))
            .is_err()
    );
    let mut malformed = valid_cut_list();
    malformed["outputTrack"][0]["sourceAudio"] = json!([
        {"streamIndex":0,"sourceChannel":0,"outputChannel":0,"points":null,"keyframes":[]},
        {"streamIndex":0,"sourceChannel":1,"outputChannel":1}
    ]);
    assert!(serde_json::from_value::<CutList>(malformed.clone()).is_err());
    assert!(
        serde_json::from_value::<Episode>(json!({"cut_list":malformed}))
            .is_err()
    );
}

#[test]
fn present_null_source_audio_and_route_keyframes_do_not_become_absent() {
    for (field, value) in [
        ("sourceAudio", json!(null)),
        (
            "keyframes",
            json!([{"streamIndex":0,"sourceChannel":0,"outputChannel":0,"keyframes":null}]),
        ),
        (
            "keyframes-with-points",
            json!([{"streamIndex":0,"sourceChannel":0,"outputChannel":0,"keyframes":null,"points":[{"time":0,"gain":0}]}]),
        ),
    ] {
        let mut malformed = valid_cut_list();
        malformed["outputTrack"][0]["sourceAudio"] = value;
        assert!(
            serde_json::from_value::<CutList>(malformed.clone()).is_err(),
            "{field}"
        );
        assert!(
            serde_json::from_value::<Episode>(json!({"cut_list":malformed}))
                .is_err(),
            "{field}"
        );
    }
}

#[test]
fn legacy_audio_mixing_optional_nulls_and_extensions_remain_compatible() {
    let mut data = valid_cut_list();
    data["audioMixing"] = json!(null);
    assert!(
        serde_json::from_value::<CutList>(data.clone())
            .unwrap()
            .audio_mixing
            .is_none()
    );
    data["audioMixing"] = json!([{"sourceChannel":0,"outputChannel":0,"keyframes":null,"legacyExtension":true}]);
    let direct: CutList = serde_json::from_value(data.clone()).unwrap();
    assert!(direct.audio_mixing.unwrap()[0].keyframes.is_none());
    data["audioMixing"][0]["keyframes"] =
        json!([{"frame":0,"volume":0.5,"legacyExtension":true}]);
    assert!(serde_json::from_value::<CutList>(data.clone()).is_ok());
    assert!(
        serde_json::from_value::<Episode>(json!({"cut_list":data})).is_ok()
    );
}

#[test]
fn source_correction_fields_reject_non_array_values() {
    for field in ["sourceAudio", "points", "keyframes"] {
        for invalid in
            [json!(false), json!(3), json!({}), json!("not an array")]
        {
            let mut value = valid_cut_list();
            if field == "sourceAudio" {
                value["outputTrack"][0]["sourceAudio"] = invalid;
            } else {
                value["outputTrack"][0]["sourceAudio"] = json!([{"streamIndex":0,"sourceChannel":0,"outputChannel":0}]);
                value["outputTrack"][0]["sourceAudio"][0][field] = invalid;
            }
            assert!(
                serde_json::from_value::<CutList>(value.clone()).is_err(),
                "accepted invalid {field}"
            );
            assert!(
                serde_json::from_value::<Episode>(json!({"cut_list":value}))
                    .is_err(),
                "accepted invalid {field}"
            );
        }
    }
}

#[tokio::test]
async fn assembled_mixing_rejects_gains_outside_the_renderable_range() {
    let mut data: CutList = serde_json::from_value(valid_cut_list()).unwrap();
    data.audio_mixing = Some(vec![
        serde_json::from_value(json!({"sourceChannel":0,"outputChannel":0}))
            .unwrap(),
    ]);
    for gain in [
        0.0,
        f32::from_bits(1) as f64,
        2.1,
        9_007_199_254_740_991.0,
        f32::MAX as f64,
    ] {
        data.audio_mixing.as_mut().unwrap()[0].keyframes =
            Some(vec![types::AudioChannelKeyframe {
                frame: 0,
                volume: gain,
            }]);
        gt_ffmpeg::edit::validate_source_audio(
            &data,
            std::path::Path::new("."),
            60.0,
        )
        .await
        .unwrap();
    }
    for gain in [-0.1, 1e39, 1e-50, f64::NAN, f64::INFINITY] {
        data.audio_mixing.as_mut().unwrap()[0].keyframes =
            Some(vec![types::AudioChannelKeyframe {
                frame: 0,
                volume: gain,
            }]);
        let error = gt_ffmpeg::edit::validate_source_audio(
            &data,
            std::path::Path::new("."),
            60.0,
        )
        .await
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("audioMixing[0]") && error.contains("gain"),
            "{gain}: {error}"
        );
    }
    for frame in [0, 1] {
        data.audio_mixing.as_mut().unwrap()[0].keyframes =
            Some(vec![types::AudioChannelKeyframe { frame, volume: 0.5 }]);
        gt_ffmpeg::edit::validate_source_audio(
            &data,
            std::path::Path::new("."),
            60.0,
        )
        .await
        .unwrap();
    }
    data.audio_mixing.as_mut().unwrap()[0].keyframes =
        Some(vec![types::AudioChannelKeyframe {
            frame: -1,
            volume: 0.5,
        }]);
    let error = gt_ffmpeg::edit::validate_source_audio(
        &data,
        std::path::Path::new("."),
        60.0,
    )
    .await
    .unwrap_err()
    .to_string();
    assert!(
        error.contains("audioMixing[0]") && error.contains("frame"),
        "{error}"
    );
}

#[test]
fn misspelled_section_audio_fails_in_both_deserialization_paths() {
    let mut malformed = valid_cut_list();
    malformed["outputTrack"][0]["sourceAudios"] = json!([
        {"streamIndex":0,"sourceChannel":0,"outputChannel":0,"keyframes":[{"frame":0,"volume":0.0}]}
    ]);
    let direct =
        serde_json::from_value::<CutList>(malformed.clone()).unwrap_err();
    assert!(direct.to_string().contains("sourceAudios"), "{direct}");
    // The DynamoDB-backed legacy renderer first deserializes an Episode,
    // then converts its CutListClass to the same CutList used by FFmpeg.
    let legacy =
        serde_json::from_value::<Episode>(json!({"cut_list":malformed}))
            .unwrap_err();
    assert!(legacy.to_string().contains("sourceAudios"), "{legacy}");
}

#[test]
fn unrecognized_route_attributes_fail_in_both_deserialization_paths() {
    for field in ["keyframe", "unexpectedRouteProperty"] {
        let mut malformed = valid_cut_list();
        malformed["outputTrack"][0]["sourceAudio"] = json!([
            {"streamIndex":0,"sourceChannel":0,"outputChannel":0}
        ]);
        malformed["outputTrack"][0]["sourceAudio"][0][field] =
            json!([{"frame":0,"volume":0.0}]);
        let direct =
            serde_json::from_value::<CutList>(malformed.clone()).unwrap_err();
        assert!(direct.to_string().contains(field), "{direct}");
        let legacy =
            serde_json::from_value::<Episode>(json!({"cut_list":malformed}))
                .unwrap_err();
        assert!(legacy.to_string().contains(field), "{legacy}");
    }
}

#[test]
fn misspelled_top_level_mixing_cannot_silently_disappear() {
    let mut malformed = valid_cut_list();
    malformed["audioMixng"] = json!([{"sourceChannel":0,"outputChannel":0}]);
    let direct =
        serde_json::from_value::<CutList>(malformed.clone()).unwrap_err();
    assert!(direct.to_string().contains("audioMixng"), "{direct}");
    let legacy =
        serde_json::from_value::<Episode>(json!({"cut_list":malformed}))
            .unwrap_err();
    assert!(legacy.to_string().contains("audioMixng"), "{legacy}");
}

#[test]
fn declared_legacy_and_routed_fields_remain_accepted() {
    let legacy = valid_cut_list();
    let direct: CutList = serde_json::from_value(legacy.clone()).unwrap();
    let episode: Episode =
        serde_json::from_value(json!({"cut_list":legacy})).unwrap();
    let from_episode: CutList = episode.cut_list.unwrap().into();
    assert_eq!(direct, from_episode);
    assert!(direct.output_track[0].source_audio.is_none());

    let mut routed = valid_cut_list();
    routed["outputTrack"][0]["transitionIn"] =
        json!({"type":"fade","duration":30});
    routed["outputTrack"][0]["transitionOut"] =
        json!({"type":"cut","duration":0});
    routed["outputTrack"][0]["sourceAudio"] = json!([
        {"streamIndex":1,"sourceChannel":0,"outputChannel":0,"keyframes":[{"frame":0,"volume":0.5}]},
        {"streamIndex":1,"sourceChannel":1,"outputChannel":1}
    ]);
    routed["audioMixing"] = json!([{ "sourceChannel":0,"outputChannel":0,"keyframes":[{"frame":0,"volume":1.0}] }]);
    routed["overlayTracks"] = json!([{ "mediaIndex":0,"sectionIndex":0,"startFrame":0,"type":"alpha","x":0,"y":0 }]);
    let direct: CutList = serde_json::from_value(routed.clone()).unwrap();
    let episode: Episode =
        serde_json::from_value(json!({"cut_list":routed})).unwrap();
    let from_episode: CutList = episode.cut_list.unwrap().into();
    assert_eq!(direct, from_episode);
    assert_eq!(
        direct.output_track[0].source_audio.as_ref().unwrap()[0]
            .keyframes
            .as_ref()
            .unwrap()[0]
            .volume,
        0.5
    );
}
