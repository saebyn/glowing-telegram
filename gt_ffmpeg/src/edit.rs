use tokio::process::Command;

use types::{
    CutList, InputMedia, MediaSection, OutputTrack, OverlayTrack,
    TransitionInClass, TransitionInType,
};

// Converts a frame number to time (seconds)
fn convert_frame_to_time(frame: i64, frame_rate: f32) -> f32 {
    frame as f32 / frame_rate
}

fn create_complex_filter(filter_steps: &[String]) -> String {
    filter_steps.join(";")
}

fn data_to_filter_complex(data: &CutList, frame_rate: f32) -> String {
    let mut filter_steps = Vec::new();
    // 1) main track building
    filter_steps.extend(create_main_track(data, frame_rate));
    // 2) overlay tracks
    filter_steps.extend(create_overlay_tracks(data, frame_rate));
    create_complex_filter(&filter_steps)
}

fn create_main_track(data: &CutList, frame_rate: f32) -> Vec<String> {
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
) -> String {
    format!(
        "[{}:v]scale=w=2560:h=1440,trim=start_frame={}:end_frame={},setpts=PTS-STARTPTS,fps=fps={}[v{}]",
        media_index, start_frame, end_frame, frame_rate, output_track_index
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
        "[{}:a]atrim=start={}:end={},asetpts=PTS-STARTPTS[a{}]",
        media_index, start_sec, end_sec, output_track_index
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
            "[{}:v]trim=start_frame={}:end_frame={},colorkey=black,colorchannelmixer=aa=0.8,setpts=PTS+{}/TB[vo{}]",
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
fn build_ffmpeg_command(data: &CutList) -> Command {
    let mut cmd = Command::new("ffmpeg");

    for m in &data.input_media {
        cmd.arg("-i").arg(m.s3_location.clone());
    }

    let filter = data_to_filter_complex(data, 60.0);

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
    cmd.arg("output.mp4");

    cmd
}

#[tokio::main]
pub async fn main() {
    let data = CutList {
        version: types::CutListVersion::The100,
        input_media: vec![
            InputMedia {
                s3_location: "s3://bucket/video1.mp4".to_string(),
                sections: vec![MediaSection {
                    start_frame: 0,
                    end_frame: 100,
                }],
            },
            InputMedia {
                s3_location: "s3://bucket/video2.mp4".to_string(),
                sections: vec![MediaSection {
                    start_frame: 0,
                    end_frame: 200,
                }],
            },
        ],
        output_track: vec![
            OutputTrack {
                media_index: 0,
                section_index: 0,
                transition_in: Some(TransitionInClass {
                    transition_type: TransitionInType::Fade,
                    duration: 30,
                }),
                transition_out: None,
            },
            OutputTrack {
                media_index: 1,
                section_index: 0,
                transition_in: None,
                transition_out: None,
            },
        ],
        overlay_tracks: Some(vec![OverlayTrack {
            x: None,
            y: None,
            media_index: 0,
            section_index: 0,
            start_frame: 0,
        }]),
    };

    let cmd = build_ffmpeg_command(&data);
    println!("{:?}", cmd);
}

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
fn test_full_command() {
    let data = CutList {
        version: types::CutListVersion::The100,
        input_media: vec![
            InputMedia {
                s3_location: "/mnt/f/Video/OBS/2024-10-14 18-32-51.mkv"
                    .to_string(),
                sections: vec![MediaSection {
                    start_frame: 0,
                    end_frame: 600,
                }],
            },
            InputMedia {
                s3_location: "/mnt/f/Video/Renders/LiveOnTwitch Render 1.mov"
                    .to_string(),
                sections: vec![MediaSection {
                    start_frame: 0,
                    end_frame: 114,
                }],
            },
            InputMedia {
                s3_location: "/mnt/f/Art/introhex.mkv".to_string(),
                sections: vec![MediaSection {
                    start_frame: 0,
                    end_frame: 30,
                }],
            },
            InputMedia {
                s3_location: "/mnt/f/Art/outro_2.mov".to_string(),
                sections: vec![MediaSection {
                    start_frame: 0,
                    end_frame: 1800,
                }],
            },
            InputMedia {
                s3_location: "/mnt/f/Video/Renders/LikeReminder1 Render 1.mov"
                    .to_string(),
                sections: vec![MediaSection {
                    start_frame: 0,
                    end_frame: 300,
                }],
            },
        ],
        output_track: vec![
            OutputTrack {
                media_index: 0,
                section_index: 0,
                transition_in: None,
                transition_out: None,
            },
            OutputTrack {
                media_index: 3,
                section_index: 0,
                transition_in: Some(TransitionInClass {
                    transition_type: TransitionInType::Fade,
                    duration: 300,
                }),
                transition_out: None,
            },
        ],
        overlay_tracks: Some(vec![
            OverlayTrack {
                x: Some(1500.0),
                y: Some(300.0),
                media_index: 1,
                section_index: 0,
                start_frame: 600,
            },
            OverlayTrack {
                x: None,
                y: None,
                media_index: 2,
                section_index: 0,
                start_frame: 0,
            },
            OverlayTrack {
                x: None,
                y: None,
                media_index: 4,
                section_index: 0,
                start_frame: 1200,
            },
        ]),
    };

    let cmd = build_ffmpeg_command(&data);
    assert_eq!(
        cmd.as_std()
            .get_args()
            .map(|x| x.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join(" "),
        "-i /mnt/f/Video/OBS/2024-10-14 18-32-51.mkv -i /mnt/f/Video/Renders/LiveOnTwitch Render 1.mov -i /mnt/f/Art/introhex.mkv -i /mnt/f/Art/outro_2.mov -i /mnt/f/Video/Renders/LikeReminder1 Render 1.mov -filter_complex [0:v]scale=w=2560:h=1440,trim=start_frame=0:end_frame=600,setpts=PTS-STARTPTS,fps=fps=60[v0];[0:a]atrim=start=0:end=10,asetpts=PTS-STARTPTS[a0];[3:v]scale=w=2560:h=1440,trim=start_frame=0:end_frame=1800,setpts=PTS-STARTPTS,fps=fps=60[v1];[3:a]atrim=start=0:end=30,asetpts=PTS-STARTPTS[a1];[v0][v1]xfade=duration=5:transition=fade:offset=5[v0];[a0][a1]acrossfade=d=5[a0];[v0]concat=n=1[vmain];[a0]concat=n=1:v=0:a=1[amain];[1:v]trim=start_frame=0:end_frame=114,colorkey=black,colorchannelmixer=aa=0.8,setpts=PTS+10/TB[vo0];[vmain][vo0]overlay=eof_action=pass:x=1500:y=300[vmain];[2:v]trim=start_frame=0:end_frame=30,colorkey=black,colorchannelmixer=aa=0.8,setpts=PTS+0/TB[vo1];[vmain][vo1]overlay=eof_action=pass:x=0:y=0[vmain];[4:v]trim=start_frame=0:end_frame=300,colorkey=black,colorchannelmixer=aa=0.8,setpts=PTS+20/TB[vo2];[vmain][vo2]overlay=eof_action=pass:x=0:y=0[vmain] -map [vmain] -map [amain] -y -c:v libx264 -c:a aac -crf 18 -preset slow -pix_fmt yuv420p -profile:v high -level 4.2 -bf 2 -g 120 -b:a 192k -ar 48000 output.mp4"
    );
}
