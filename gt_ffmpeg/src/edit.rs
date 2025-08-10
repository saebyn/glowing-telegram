use std::fmt::{self, Display};
use std::{collections::HashMap, rc::Rc};
use tokio::process::Command;

use types::{CutList, InputMedia, OutputTrack, TransitionInType};

// Converts a frame number to time (seconds)
fn convert_frame_to_time(frame: i64, frame_rate: f32) -> f32 {
    frame as f32 / frame_rate
}

/// Represents a filter with a name and a set of options.
struct Filter {
    name: Rc<str>,
    options: HashMap<Rc<str>, Rc<str>>,
}

impl Filter {
    fn new(name: &str, options: Vec<(&str, std::string::String)>) -> Self {
        Self {
            name: name.into(),
            options: options
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        }
    }
}

impl Display for Filter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts: Vec<String> = self
            .options
            .iter()
            .map(|(k, v)| {
                if v.is_empty() {
                    k.to_string()
                } else {
                    format!("{}={}", k, v)
                }
            })
            .collect();

        parts.sort();

        write!(f, "{}={}", self.name, parts.join(":"))
    }
}

/// FilterChannel represents an input or output of a filter.
/// It is represented as a tuple of (channel_type, index, is_source).
/// channel_type is 'v' for video and 'a' for audio.
/// This corresponds to the ffmpeg filter complex syntax, such that
/// [0:v] is the first video stream that is input to the filter complex,
/// and [v0] is the first video stream that is output from the filter complex.
/// These are represented as FilterChannel::SourceVideo(0) and
/// FilterChannel::MyVideo(0) respectively.
/// These could be constructed using the new method, but there are also
/// convenience methods for video and audio streams, such as
/// FilterChannel::SourceVideo(0) and FilterChannel::MyVideo(0).
enum FilterChannel {
    SourceAudio(u8),
    SourceVideo(u8),
    MyAudio(u8),
    MyVideo(u8),
    OverlayVideo(u8),
}

impl Display for FilterChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FilterChannel::SourceAudio(index) => write!(f, "[{}:a]", index),
            FilterChannel::SourceVideo(index) => write!(f, "[{}:v]", index),
            FilterChannel::MyAudio(index) => write!(f, "[a{}]", index),
            FilterChannel::MyVideo(index) => write!(f, "[v{}]", index),
            FilterChannel::OverlayVideo(index) => write!(f, "[o{}]", index),
        }
    }
}

/// Represents a filter pipe with a set of filters and inputs.
struct FilterPipe {
    filters: Rc<[Filter]>,
    inputs: Rc<[FilterChannel]>,
    output: FilterChannel,
}

impl FilterPipe {
    fn new(
        filters: Vec<Filter>,
        inputs: Vec<FilterChannel>,
        output: FilterChannel,
    ) -> Self {
        Self {
            filters: filters.into(),
            inputs: inputs.into(),
            output,
        }
    }
}

impl Display for FilterPipe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}{}",
            self.inputs
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join(""),
            self.filters
                .iter()
                .map(|filter| filter.to_string())
                .collect::<Vec<_>>()
                .join(","),
            self.output,
        )
    }
}

/// Represents a filter graph with a set of pipes.
struct FilterGraph {
    pipes: Rc<[FilterPipe]>,
}

impl FilterGraph {
    fn new(
        cutlist: &CutList,
        frame_rate: f32,
        resolution: (u32, u32),
    ) -> Self {
        let mut pipes = Vec::new();
        // 1) main track building
        pipes.extend(create_main_track(cutlist, frame_rate, resolution));
        // 2) overlay tracks
        pipes.extend(create_overlay_tracks(cutlist, frame_rate, resolution));

        Self {
            pipes: pipes.into(),
        }
    }
}

impl Display for FilterGraph {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            self.pipes
                .iter()
                .map(|pipe| pipe.to_string())
                .collect::<Vec<_>>()
                .join(";")
        )
    }
}

fn create_main_track(
    data: &CutList,
    frame_rate: f32,
    resolution: (u32, u32),
) -> Vec<FilterPipe> {
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
) -> FilterPipe {
    FilterPipe::new(
        vec![
            Filter::new(
                "scale",
                vec![
                    ("w", resolution.0.to_string()),
                    ("h", resolution.1.to_string()),
                ],
            ),
            Filter::new(
                "trim",
                vec![
                    ("start_frame", start_frame.to_string()),
                    ("end_frame", end_frame.to_string()),
                ],
            ),
            Filter::new("setpts", vec![("PTS-STARTPTS", "".to_string())]),
            Filter::new("fps", vec![("fps", frame_rate.to_string())]),
        ],
        vec![FilterChannel::SourceVideo(media_index as u8)],
        FilterChannel::MyVideo(output_track_index as u8),
    )
}

fn add_audio_to_output_track(
    media_index: usize,
    start_frame: i64,
    end_frame: i64,
    output_track_index: usize,
    frame_rate: f32,
) -> FilterPipe {
    let start_sec = convert_frame_to_time(start_frame, frame_rate);
    let end_sec = convert_frame_to_time(end_frame, frame_rate);

    FilterPipe::new(
        vec![
            Filter::new(
                "atrim",
                vec![
                    ("start", start_sec.to_string()),
                    ("end", end_sec.to_string()),
                ],
            ),
            Filter::new("asetpts", vec![("PTS-STARTPTS", "".to_string())]),
        ],
        vec![FilterChannel::SourceAudio(media_index as u8)],
        FilterChannel::MyAudio(output_track_index as u8),
    )
}

fn assemble_track_segments(
    output_tracks: &[OutputTrack],
    input_media: &[InputMedia],
    frame_rate: f32,
) -> Vec<FilterPipe> {
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
                parts.push(FilterPipe::new(
                    vec![Filter::new(
                        "xfade",
                        vec![
                            ("duration", duration_sec.to_string()),
                            ("transition", "fade".to_string()),
                            ("offset", offset.to_string()),
                        ],
                    )],
                    vec![
                        FilterChannel::MyVideo((i - 1) as u8),
                        FilterChannel::MyVideo(i as u8),
                    ],
                    FilterChannel::MyVideo((i - 1) as u8),
                ));

                parts.push(FilterPipe::new(
                    vec![Filter::new(
                        "acrossfade",
                        vec![("d", duration_sec.to_string())],
                    )],
                    vec![
                        FilterChannel::MyAudio((i - 1) as u8),
                        FilterChannel::MyAudio(i as u8),
                    ],
                    FilterChannel::MyAudio((i - 1) as u8),
                ));
            } else {
                video_streams.push(FilterChannel::MyVideo(i as u8));
                audio_streams.push(FilterChannel::MyAudio(i as u8));
            }
        } else {
            video_streams.push(FilterChannel::MyVideo(i as u8));
            audio_streams.push(FilterChannel::MyAudio(i as u8));
        }
    }

    // Concat final streams
    if !video_streams.is_empty() {
        parts.push(FilterPipe::new(
            vec![Filter::new(
                "concat",
                vec![("n", video_streams.len().to_string())],
            )],
            video_streams,
            FilterChannel::MyVideo(255),
        ));
    }
    if !audio_streams.is_empty() {
        parts.push(FilterPipe::new(
            vec![Filter::new(
                "concat",
                vec![
                    ("n", audio_streams.len().to_string()),
                    ("v", "0".to_string()),
                    ("a", "1".to_string()),
                ],
            )],
            audio_streams,
            FilterChannel::MyAudio(255),
        ));
    }

    parts
}

fn create_overlay_tracks(
    data: &CutList,
    frame_rate: f32,
    resolution: (u32, u32),
) -> Vec<FilterPipe> {
    let mut parts = Vec::new();

    let overlay_tracks = match data.overlay_tracks {
        Some(ref tracks) => tracks,
        None => &Vec::new(),
    };

    for (i, overlay) in overlay_tracks.iter().enumerate() {
        let media_index = overlay.media_index;
        let section = &data.input_media[media_index as usize].sections
            [overlay.section_index as usize];

        let mut filters = Vec::new();

        if overlay.overlay_track_type == types::OverlayTrackType::Colorkey {
            filters.push(Filter::new(
                "colorkey",
                vec![("black", "".to_string())],
            ));
            filters.push(Filter::new(
                "colorchannelmixer",
                vec![("aa", "0.8".to_string())],
            ));
        }

        filters.extend(vec![
            Filter::new(
                "trim",
                vec![
                    ("start_frame", section.start_frame.to_string()),
                    ("end_frame", section.end_frame.to_string()),
                ],
            ),
            Filter::new(
                "scale",
                vec![
                    ("w", resolution.0.to_string()),
                    ("h", resolution.1.to_string()),
                ],
            ),
            Filter::new(
                "setpts",
                vec![(
                    format!(
                        "PTS+{}/TB",
                        convert_frame_to_time(overlay.start_frame, frame_rate)
                    )
                    .as_str(),
                    "".to_string(),
                )],
            ),
            Filter::new("format", vec![("yuva420p", "".to_string())]),
        ]);

        parts.push(FilterPipe::new(
            filters,
            vec![FilterChannel::SourceVideo(media_index as u8)],
            FilterChannel::OverlayVideo(i as u8),
        ));

        parts.push(FilterPipe::new(
            vec![Filter::new(
                "overlay",
                vec![
                    ("eof_action", "pass".to_string()),
                    ("x", overlay.x.unwrap_or(0.0).to_string()),
                    ("y", overlay.y.unwrap_or(0.0).to_string()),
                ],
            )],
            vec![
                FilterChannel::MyVideo(255),
                FilterChannel::OverlayVideo(i as u8),
            ],
            FilterChannel::MyVideo(255),
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

    let filter = FilterGraph::new(data, frame_rate, resolution).to_string();

    cmd.arg("-filter_complex").arg(filter);

    cmd.arg("-map").arg("[v255]").arg("-map").arg("[a255]");
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

    use super::build_ffmpeg_command;

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
                  "startFrame": 0,
                  "type": "alpha"
                 },
                 {
                  "mediaIndex": 6,
                  "sectionIndex": 0,
                  "startFrame": 1800,
                  "type": "colorkey"
                 },
                 {
                  "mediaIndex": 7,
                  "sectionIndex": 0,
                  "startFrame": 3600,
                  "type": "colorkey"
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
            "-i 2024-12-08/2024-12-08 08-26-45.mkv -i 2024-12-08/2024-12-08 08-46-51.mkv -i 2024-12-08/2024-12-08 09-06-51.mkv -i 2024-12-08/2024-12-08 09-26-51.mkv -i my_stock/outro_2.mov -i my_stock/introhex.mkv -i my_stock/LiveOnTwitch Render 1.mov -i my_stock/LikeReminder1 Render 1.mov -filter_complex [0:v]scale=h=1440:w=2560,trim=end_frame=72250:start_frame=20100,setpts=PTS-STARTPTS,fps=fps=60[v0];[0:a]atrim=end=1204.1666:start=335,asetpts=PTS-STARTPTS[a0];[1:v]scale=h=1440:w=2560,trim=end_frame=72000:start_frame=0,setpts=PTS-STARTPTS,fps=fps=60[v1];[1:a]atrim=end=1200:start=0,asetpts=PTS-STARTPTS[a1];[2:v]scale=h=1440:w=2560,trim=end_frame=72000:start_frame=0,setpts=PTS-STARTPTS,fps=fps=60[v2];[2:a]atrim=end=1200:start=0,asetpts=PTS-STARTPTS[a2];[3:v]scale=h=1440:w=2560,trim=end_frame=8326:start_frame=0,setpts=PTS-STARTPTS,fps=fps=60[v3];[3:a]atrim=end=138.76666:start=0,asetpts=PTS-STARTPTS[a3];[4:v]scale=h=1440:w=2560,trim=end_frame=1800:start_frame=0,setpts=PTS-STARTPTS,fps=fps=60[v4];[4:a]atrim=end=30:start=0,asetpts=PTS-STARTPTS[a4];[v3][v4]xfade=duration=5:offset=133.76666:transition=fade[v3];[a3][a4]acrossfade=d=5[a3];[v0][v1][v2][v3]concat=n=4[v255];[a0][a1][a2][a3]concat=a=1:n=4:v=0[a255];[5:v]trim=end_frame=3600:start_frame=0,scale=h=1440:w=2560,setpts=PTS+0/TB,format=yuva420p[o0];[v255][o0]overlay=eof_action=pass:x=0:y=0[v255];[6:v]colorkey=black,colorchannelmixer=aa=0.8,trim=end_frame=114:start_frame=0,scale=h=1440:w=2560,setpts=PTS+30/TB,format=yuva420p[o1];[v255][o1]overlay=eof_action=pass:x=0:y=0[v255];[7:v]colorkey=black,colorchannelmixer=aa=0.8,trim=end_frame=300:start_frame=0,scale=h=1440:w=2560,setpts=PTS+60/TB,format=yuva420p[o2];[v255][o2]overlay=eof_action=pass:x=0:y=0[v255] -map [v255] -map [a255] -y -c:v libx264 -c:a aac -crf 18 -preset slow -pix_fmt yuv420p -profile:v high -level 4.2 -bf 2 -g 120 -b:a 192k -ar 48000 -f mp4 output.mp4"
        );
    }
}
