//! The shared legacy cut-list boundary must distinguish omission from null.
use serde_json::{Value, json};
use types::{CutList, Episode};

fn base() -> Value {
    json!({"version":"1.0.0","inputMedia":[{"s3Location":"fixture.mkv","sections":[{"startFrame":0,"endFrame":120}]}],"outputTrack":[{"mediaIndex":0,"sectionIndex":0}]})
}

#[test]
fn present_null_audio_fields_do_not_become_omissions() {
    let routes = json!([{"streamIndex":0,"sourceChannel":0,"outputChannel":0},{"streamIndex":0,"sourceChannel":1,"outputChannel":1}]);
    let omitted: CutList = serde_json::from_value(base()).unwrap();
    assert!(omitted.output_track[0].source_audio.is_none());
    for case in [
        "sourceAudio",
        "points",
        "keyframes",
        "points-with-keyframes",
        "keyframes-with-points",
    ] {
        let mut value = base();
        value["outputTrack"][0]["sourceAudio"] = routes.clone();
        match case {
            "sourceAudio" => {
                value["outputTrack"][0]["sourceAudio"] = json!(null)
            }
            "points" => {
                value["outputTrack"][0]["sourceAudio"][0]["points"] =
                    json!(null)
            }
            "keyframes" => {
                value["outputTrack"][0]["sourceAudio"][0]["keyframes"] =
                    json!(null)
            }
            "points-with-keyframes" => {
                value["outputTrack"][0]["sourceAudio"][0]["points"] =
                    json!(null);
                value["outputTrack"][0]["sourceAudio"][0]["keyframes"] =
                    json!([]);
            }
            _ => {
                value["outputTrack"][0]["sourceAudio"][0]["keyframes"] =
                    json!(null);
                value["outputTrack"][0]["sourceAudio"][0]["points"] =
                    json!([{"time":0,"gain":0}]);
            }
        }
        assert!(
            serde_json::from_value::<CutList>(value.clone()).is_err(),
            "accepted null {case}"
        );
        assert!(
            serde_json::from_value::<Episode>(json!({"cut_list":value}))
                .is_err(),
            "accepted null {case}"
        );
    }
    let mut valid = base();
    valid["outputTrack"][0]["sourceAudio"] = routes;
    valid["outputTrack"][0]["sourceAudio"][0]["keyframes"] = json!([]);
    let parsed: CutList = serde_json::from_value(valid).unwrap();
    assert_eq!(
        parsed.output_track[0].source_audio.as_ref().unwrap()[0]
            .keyframes
            .as_ref()
            .unwrap()
            .len(),
        0
    );
}
