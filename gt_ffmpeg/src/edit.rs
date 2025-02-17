use tokio::process::Command;

use types::{CutList, InputMedia, OutputTrack, TransitionInType};

// Converts a frame number to time (seconds)
fn convert_frame_to_time(frame: i64, frame_rate: f32) -> f32 {
    frame as f32 / frame_rate
}

fn create_complex_filter(filter_steps: &[String]) -> String {
    filter_steps.join(";")
}

fn data_to_filter_complex(
    data: &CutList,
    frame_rate: f32,
    resolution: (u32, u32),
) -> String {
    let mut filter_steps = Vec::new();
    // 1) main track building
    filter_steps.extend(create_main_track(data, frame_rate, resolution));
    // 2) overlay tracks
    filter_steps.extend(create_overlay_tracks(data, frame_rate));
    create_complex_filter(&filter_steps)
}

fn create_main_track(
    data: &CutList,
    frame_rate: f32,
    resolution: (u32, u32),
) -> Vec<String> {
    let mut parts = Vec::new();
    for (i, track) in data.output_track.iter().enumerate() {
        let media_item = &data.input_media[track.media_index as usize];
        let section = &media_item.sections[track.section_index as usize];
        // Add video
        parts.push(add_video_to_output_track(
            track.media_index as usize,
            section.start_frame,
            section.end_frame,
            i,
            frame_rate,
            resolution,
        ));
        // Add audio
        parts.push(add_audio_to_output_track(
            track.media_index as usize,
            section.start_frame,
            section.end_frame,
            i,
            frame_rate,
        ));
    }
    parts.extend(assemble_track_segments(
        &data.output_track,
        &data.input_media,
        frame_rate,
    ));
    parts
}

fn add_video_to_output_track(
    media_index: usize,
    start_frame: i64,
    end_frame: i64,
    output_track_index: usize,
    frame_rate: f32,
    resolution: (u32, u32),
) -> String {
    format!(
        "[{media_index}:v]scale=w={width}:h={height},trim=start_frame={start_frame}:end_frame={end_frame},setpts=PTS-STARTPTS,fps=fps={frame_rate}[v{output_track_index}]",
        media_index = media_index,
        start_frame = start_frame,
        end_frame = end_frame,
        frame_rate = frame_rate,
        output_track_index = output_track_index,
        width = resolution.0,
        height = resolution.1
    )
}

fn add_audio_to_output_track(
    media_index: usize,
    start_frame: i64,
    end_frame: i64,
    output_track_index: usize,
    frame_rate: f32,
) -> String {
    let start_sec = convert_frame_to_time(start_frame, frame_rate);
    let end_sec = convert_frame_to_time(end_frame, frame_rate);
    format!(
        "[{media_index}:a]atrim=start={start_sec}:end={end_sec},asetpts=PTS-STARTPTS[a{output_track_index}]",
        media_index = media_index,
        start_sec = start_sec,
        end_sec = end_sec,
        output_track_index = output_track_index
    )
}

fn assemble_track_segments(
    output_tracks: &[OutputTrack],
    input_media: &[InputMedia],
    frame_rate: f32,
) -> Vec<String> {
    let mut parts = Vec::new();
    let mut video_streams = Vec::new();
    let mut audio_streams = Vec::new();

    for (i, track) in output_tracks.iter().enumerate() {
        if let Some(transition) = &track.transition_in {
            if transition.transition_type == TransitionInType::Fade && i > 0 {
                let duration_sec =
                    convert_frame_to_time(transition.duration, frame_rate);
                let prev_section = &input_media
                    [output_tracks[i - 1].media_index as usize]
                    .sections
                    [output_tracks[i - 1].section_index as usize];
                let prev_len_sec = convert_frame_to_time(
                    prev_section.end_frame - prev_section.start_frame,
                    frame_rate,
                );
                let offset = prev_len_sec - duration_sec;
                // xfade
                parts.push(format!(
                    "[v{0}][v{1}]xfade=duration={duration_sec}:transition=fade:offset={offset}[v{0}]",
                    i - 1,
                    i
                ));
                parts.push(format!(
                    "[a{0}][a{1}]acrossfade=d={duration_sec}[a{0}]",
                    i - 1,
                    i
                ));
            } else {
                video_streams.push(format!("v{}", i));
                audio_streams.push(format!("a{}", i));
            }
        } else {
            video_streams.push(format!("v{}", i));
            audio_streams.push(format!("a{}", i));
        }
    }

    // Concat final streams
    if !video_streams.is_empty() {
        parts.push(format!(
            "[{}]concat=n={}[vmain]",
            video_streams.join("]["),
            video_streams.len()
        ));
    }
    if !audio_streams.is_empty() {
        parts.push(format!(
            "[{}]concat=n={}:v=0:a=1[amain]",
            audio_streams.join("]["),
            audio_streams.len()
        ));
    }

    parts
}

fn create_overlay_tracks(data: &CutList, frame_rate: f32) -> Vec<String> {
    let mut parts = Vec::new();

    let overlay_tracks = match data.overlay_tracks {
        Some(ref tracks) => tracks,
        None => &Vec::new(),
    };

    for (i, overlay) in overlay_tracks.iter().enumerate() {
        let media_index = overlay.media_index;
        let section = &data.input_media[media_index as usize].sections
            [overlay.section_index as usize];
        parts.push(format!(
            "[{}:v]trim=start_frame={}:end_frame={},colorkey=black,colorchannelmixer=aa=0.8,setpts=PTS+{}/TB,format=yuva420p[vo{}]",
            media_index,
            section.start_frame,
            section.end_frame,
            convert_frame_to_time(overlay.start_frame, frame_rate),
            i
        ));
        parts.push(format!(
            "[vmain][vo{}]overlay=eof_action=pass:x={}:y={}[vmain]",
            i,
            overlay.x.unwrap_or(0.0),
            overlay.y.unwrap_or(0.0)
        ));
    }
    parts
}

// Builds the final ffmpeg command
pub fn build_ffmpeg_command(
    data: &CutList,
    frame_rate: f32,
    output_file: &str,
    resolution: (u32, u32),
) -> Command {
    let mut cmd = Command::new("ffmpeg");

    for m in &data.input_media {
        cmd.arg("-i").arg(m.s3_location.clone());
    }

    let filter = data_to_filter_complex(data, frame_rate, resolution);

    cmd.arg("-filter_complex").arg(filter);

    cmd.arg("-map").arg("[vmain]").arg("-map").arg("[amain]");
    cmd.arg("-y")
        .arg("-c:v")
        .arg("libx264")
        .arg("-c:a")
        .arg("aac");
    cmd.arg("-crf").arg("18").arg("-preset").arg("slow");
    cmd.arg("-pix_fmt")
        .arg("yuv420p")
        .arg("-profile:v")
        .arg("high");
    cmd.arg("-level")
        .arg("4.2")
        .arg("-bf")
        .arg("2")
        .arg("-g")
        .arg("120");
    cmd.arg("-b:a").arg("192k").arg("-ar").arg("48000");
    cmd.arg("-f").arg("mp4");
    cmd.arg(output_file);

    cmd
}

#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use types::CutList;

    use super::{build_ffmpeg_command, create_complex_filter};

    #[test]
    fn test_create_complex_filter() {
        let filter = create_complex_filter(&[
            "scale=w=2560:h=1440:trim=start_frame=0:end_frame=100".to_string(),
            "atrim=start=0:end=1".to_string(),
        ]);
        assert_eq!(
            filter,
            "scale=w=2560:h=1440:trim=start_frame=0:end_frame=100;atrim=start=0:end=1"
        );
    }

    #[test]
    fn test_real_example_1() {
        let data = json!(
            {
                "inputMedia": [
                 {
                  "s3Location": "2024-12-08/2024-12-08 08-26-45.mkv",
                  "sections": [
                   {
                    "endFrame": 72250,
                    "startFrame": 20100
                   }
                  ]
                 },
                 {
                  "s3Location": "2024-12-08/2024-12-08 08-46-51.mkv",
                  "sections": [
                   {
                    "endFrame": 72000,
                    "startFrame": 0
                   }
                  ]
                 },
                 {
                  "s3Location": "2024-12-08/2024-12-08 09-06-51.mkv",
                  "sections": [
                   {
                    "endFrame": 72000,
                    "startFrame": 0
                   }
                  ]
                 },
                 {
                  "s3Location": "2024-12-08/2024-12-08 09-26-51.mkv",
                  "sections": [
                   {
                    "endFrame": 8326,
                    "startFrame": 0
                   }
                  ]
                 },
                 {
                  "s3Location": "my_stock/outro_2.mov",
                  "sections": [
                   {
                    "endFrame": 1800,
                    "startFrame": 0
                   }
                  ]
                 },
                 {
                  "s3Location": "my_stock/introhex.mkv",
                  "sections": [
                   {
                    "endFrame": 3600,
                    "startFrame": 0
                   }
                  ]
                 },
                 {
                  "s3Location": "my_stock/LiveOnTwitch Render 1.mov",
                  "sections": [
                   {
                    "endFrame": 114,
                    "startFrame": 0
                   }
                  ]
                 },
                 {
                  "s3Location": "my_stock/LikeReminder1 Render 1.mov",
                  "sections": [
                   {
                    "endFrame": 300,
                    "startFrame": 0
                   }
                  ]
                 }
                ],
                "outputTrack": [
                 {
                  "mediaIndex": 0,
                  "sectionIndex": 0
                 },
                 {
                  "mediaIndex": 1,
                  "sectionIndex": 0
                 },
                 {
                  "mediaIndex": 2,
                  "sectionIndex": 0
                 },
                 {
                  "mediaIndex": 3,
                  "sectionIndex": 0
                 },
                 {
                  "mediaIndex": 4,
                  "sectionIndex": 0,
                  "transitionIn": {
                   "duration": 300,
                   "type": "fade"
                  }
                 }
                ],
                "overlayTracks": [
                 {
                  "mediaIndex": 5,
                  "sectionIndex": 0,
                  "startFrame": 0
                 },
                 {
                  "mediaIndex": 6,
                  "sectionIndex": 0,
                  "startFrame": 1800,
                  "x": 1500,
                  "y": 300
                 },
                 {
                  "mediaIndex": 7,
                  "sectionIndex": 0,
                  "startFrame": 3600,
                  "x": 1500,
                  "y": 300
                 }
                ],
                "version": "1.0.0"
               }
        );

        let data: CutList = serde_json::from_value(data).unwrap();

        let cmd =
            build_ffmpeg_command(&data, 60.0, "output.mp4", (2560, 1440));

        assert_eq!(
            cmd.as_std()
                .get_args()
                .map(|x| x.to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join(" "),
            "-i 2024-12-08/2024-12-08 08-26-45.mkv -i 2024-12-08/2024-12-08 08-46-51.mkv -i 2024-12-08/2024-12-08 09-06-51.mkv -i 2024-12-08/2024-12-08 09-26-51.mkv -i my_stock/outro_2.mov -i my_stock/introhex.mkv -i my_stock/LiveOnTwitch Render 1.mov -i my_stock/LikeReminder1 Render 1.mov -filter_complex [0:v]scale=w=2560:h=1440,trim=start_frame=20100:end_frame=72250,setpts=PTS-STARTPTS,fps=fps=60[v0];[0:a]atrim=start=335:end=1204.1666,asetpts=PTS-STARTPTS[a0];[1:v]scale=w=2560:h=1440,trim=start_frame=0:end_frame=72000,setpts=PTS-STARTPTS,fps=fps=60[v1];[1:a]atrim=start=0:end=1200,asetpts=PTS-STARTPTS[a1];[2:v]scale=w=2560:h=1440,trim=start_frame=0:end_frame=72000,setpts=PTS-STARTPTS,fps=fps=60[v2];[2:a]atrim=start=0:end=1200,asetpts=PTS-STARTPTS[a2];[3:v]scale=w=2560:h=1440,trim=start_frame=0:end_frame=8326,setpts=PTS-STARTPTS,fps=fps=60[v3];[3:a]atrim=start=0:end=138.76666,asetpts=PTS-STARTPTS[a3];[4:v]scale=w=2560:h=1440,trim=start_frame=0:end_frame=1800,setpts=PTS-STARTPTS,fps=fps=60[v4];[4:a]atrim=start=0:end=30,asetpts=PTS-STARTPTS[a4];[v3][v4]xfade=duration=5:transition=fade:offset=133.76666[v3];[a3][a4]acrossfade=d=5[a3];[v0][v1][v2][v3]concat=n=4[vmain];[a0][a1][a2][a3]concat=n=4:v=0:a=1[amain];[5:v]trim=start_frame=0:end_frame=3600,colorkey=black,colorchannelmixer=aa=0.8,setpts=PTS+0/TB,format=yuva420p[vo0];[vmain][vo0]overlay=eof_action=pass:x=0:y=0[vmain];[6:v]trim=start_frame=0:end_frame=114,colorkey=black,colorchannelmixer=aa=0.8,setpts=PTS+30/TB,format=yuva420p[vo1];[vmain][vo1]overlay=eof_action=pass:x=1500:y=300[vmain];[7:v]trim=start_frame=0:end_frame=300,colorkey=black,colorchannelmixer=aa=0.8,setpts=PTS+60/TB,format=yuva420p[vo2];[vmain][vo2]overlay=eof_action=pass:x=1500:y=300[vmain] -map [vmain] -map [amain] -y -c:v libx264 -c:a aac -crf 18 -preset slow -pix_fmt yuv420p -profile:v high -level 4.2 -bf 2 -g 120 -b:a 192k -ar 48000 -f mp4 output.mp4"
        );
    }
}
