use serde::Deserialize;
use tokio::process::Command;
use tracing;

/*
Sample ffprobe output
{
    "streams": [
        {
            "index": 0,
            "codec_name": "h264",
            "codec_long_name": "H.264 / AVC / MPEG-4 AVC / MPEG-4 part 10",
            "profile": "High",
            "codec_type": "video",
            "codec_tag_string": "[0][0][0][0]",
            "codec_tag": "0x0000",
            "width": 2560,
            "height": 1440,
            "coded_width": 2560,
            "coded_height": 1440,
            "closed_captions": 0,
            "film_grain": 0,
            "has_b_frames": 1,
            "sample_aspect_ratio": "1:1",
            "display_aspect_ratio": "16:9",
            "pix_fmt": "yuv420p",
            "level": 51,
            "color_range": "tv",
            "color_space": "bt709",
            "color_transfer": "bt709",
            "color_primaries": "bt709",
            "chroma_location": "left",
            "field_order": "progressive",
            "refs": 1,
            "is_avc": "true",
            "nal_length_size": "4",
            "r_frame_rate": "60/1",
            "avg_frame_rate": "60/1",
            "time_base": "1/1000",
            "start_pts": 0,
            "start_time": "0.000000",
            "bits_per_raw_sample": "8",
            "extradata_size": 60,
            "disposition": {
                "default": 0,
                "dub": 0,
                "original": 0,
                "comment": 0,
                "lyrics": 0,
                "karaoke": 0,
                "forced": 0,
                "hearing_impaired": 0,
                "visual_impaired": 0,
                "clean_effects": 0,
                "attached_pic": 0,
                "timed_thumbnails": 0,
                "captions": 0,
                "descriptions": 0,
                "metadata": 0,
                "dependent": 0,
                "still_image": 0
            },
            "tags": {
                "DURATION": "00:20:04.167000000"
            }
        },
        {
            "index": 1,
            "codec_name": "aac",
            "codec_long_name": "AAC (Advanced Audio Coding)",
            "profile": "LC",
            "codec_type": "audio",
            "codec_tag_string": "[0][0][0][0]",
            "codec_tag": "0x0000",
            "sample_fmt": "fltp",
            "sample_rate": "48000",
            "channels": 2,
            "channel_layout": "stereo",
            "bits_per_sample": 0,
            "initial_padding": 0,
            "r_frame_rate": "0/0",
            "avg_frame_rate": "0/0",
            "time_base": "1/1000",
            "start_pts": 0,
            "start_time": "0.000000",
            "extradata_size": 5,
            "disposition": {
                "default": 0,
                "dub": 0,
                "original": 0,
                "comment": 0,
                "lyrics": 0,
                "karaoke": 0,
                "forced": 0,
                "hearing_impaired": 0,
                "visual_impaired": 0,
                "clean_effects": 0,
                "attached_pic": 0,
                "timed_thumbnails": 0,
                "captions": 0,
                "descriptions": 0,
                "metadata": 0,
                "dependent": 0,
                "still_image": 0
            },
            "tags": {
                "title": "track 1",
                "DURATION": "00:20:04.181000000"
            }
        },
        {
            "index": 2,
            "codec_name": "aac",
            "codec_long_name": "AAC (Advanced Audio Coding)",
            "profile": "LC",
            "codec_type": "audio",
            "codec_tag_string": "[0][0][0][0]",
            "codec_tag": "0x0000",
            "sample_fmt": "fltp",
            "sample_rate": "48000",
            "channels": 2,
            "channel_layout": "stereo",
            "bits_per_sample": 0,
            "initial_padding": 0,
            "r_frame_rate": "0/0",
            "avg_frame_rate": "0/0",
            "time_base": "1/1000",
            "start_pts": 0,
            "start_time": "0.000000",
            "extradata_size": 5,
            "disposition": {
                "default": 0,
                "dub": 0,
                "original": 0,
                "comment": 0,
                "lyrics": 0,
                "karaoke": 0,
                "forced": 0,
                "hearing_impaired": 0,
                "visual_impaired": 0,
                "clean_effects": 0,
                "attached_pic": 0,
                "timed_thumbnails": 0,
                "captions": 0,
                "descriptions": 0,
                "metadata": 0,
                "dependent": 0,
                "still_image": 0
            },
            "tags": {
                "title": "track 2",
                "DURATION": "00:20:04.181000000"
            }
        },
        {
            "index": 3,
            "codec_name": "aac",
            "codec_long_name": "AAC (Advanced Audio Coding)",
            "profile": "LC",
            "codec_type": "audio",
            "codec_tag_string": "[0][0][0][0]",
            "codec_tag": "0x0000",
            "sample_fmt": "fltp",
            "sample_rate": "48000",
            "channels": 2,
            "channel_layout": "stereo",
            "bits_per_sample": 0,
            "initial_padding": 0,
            "r_frame_rate": "0/0",
            "avg_frame_rate": "0/0",
            "time_base": "1/1000",
            "start_pts": 0,
            "start_time": "0.000000",
            "extradata_size": 5,
            "disposition": {
                "default": 0,
                "dub": 0,
                "original": 0,
                "comment": 0,
                "lyrics": 0,
                "karaoke": 0,
                "forced": 0,
                "hearing_impaired": 0,
                "visual_impaired": 0,
                "clean_effects": 0,
                "attached_pic": 0,
                "timed_thumbnails": 0,
                "captions": 0,
                "descriptions": 0,
                "metadata": 0,
                "dependent": 0,
                "still_image": 0
            },
            "tags": {
                "title": "track 3",
                "DURATION": "00:20:04.181000000"
            }
        },
        {
            "index": 4,
            "codec_name": "aac",
            "codec_long_name": "AAC (Advanced Audio Coding)",
            "profile": "LC",
            "codec_type": "audio",
            "codec_tag_string": "[0][0][0][0]",
            "codec_tag": "0x0000",
            "sample_fmt": "fltp",
            "sample_rate": "48000",
            "channels": 2,
            "channel_layout": "stereo",
            "bits_per_sample": 0,
            "initial_padding": 0,
            "r_frame_rate": "0/0",
            "avg_frame_rate": "0/0",
            "time_base": "1/1000",
            "start_pts": 0,
            "start_time": "0.000000",
            "extradata_size": 5,
            "disposition": {
                "default": 0,
                "dub": 0,
                "original": 0,
                "comment": 0,
                "lyrics": 0,
                "karaoke": 0,
                "forced": 0,
                "hearing_impaired": 0,
                "visual_impaired": 0,
                "clean_effects": 0,
                "attached_pic": 0,
                "timed_thumbnails": 0,
                "captions": 0,
                "descriptions": 0,
                "metadata": 0,
                "dependent": 0,
                "still_image": 0
            },
            "tags": {
                "title": "track 4",
                "DURATION": "00:20:04.181000000"
            }
        }
    ],
    "format": {
        "filename": ".\\2023-11-28 07-52-00.mkv",
        "nb_streams": 5,
        "nb_programs": 0,
        "format_name": "matroska,webm",
        "format_long_name": "Matroska / WebM",
        "start_time": "0.000000",
        "duration": "1204.181000",
        "size": "6090918208",
        "bit_rate": "40465134",
        "probe_score": 100,
        "tags": {
            "ENCODER": "Lavf60.3.100"
        }
    }
}
*/

fn str_to_u32<'de, D>(deserializer: D) -> Result<Option<u32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    match s.parse::<u32>() {
        Ok(u) => Ok(Some(u)),
        Err(_) => Ok(None),
    }
}

fn str_to_u64<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    match s.parse::<u64>() {
        Ok(u) => Ok(Some(u)),
        Err(_) => Ok(None),
    }
}

fn str_to_f32<'de, D>(deserializer: D) -> Result<Option<f32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: String = serde::Deserialize::deserialize(deserializer)?;
    match s.parse::<f32>() {
        Ok(u) => Ok(Some(u)),
        Err(_) => Ok(None),
    }
}

#[derive(Debug, Deserialize)]
pub struct FFProbeStream {
    pub index: u32,
    pub codec_name: Option<String>,
    pub codec_long_name: Option<String>,
    pub profile: Option<String>,
    pub codec_type: String,
    pub codec_tag_string: Option<String>,
    pub codec_tag: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub coded_width: Option<u32>,
    pub coded_height: Option<u32>,
    pub closed_captions: Option<u32>,
    pub film_grain: Option<u32>,
    pub has_b_frames: Option<u32>,
    pub sample_aspect_ratio: Option<String>,
    pub display_aspect_ratio: Option<String>,
    pub pix_fmt: Option<String>,
    pub level: Option<u32>,
    pub color_range: Option<String>,
    pub color_space: Option<String>,
    pub color_transfer: Option<String>,
    pub color_primaries: Option<String>,
    pub chroma_location: Option<String>,
    pub field_order: Option<String>,
    pub refs: Option<u32>,
    pub is_avc: Option<String>,
    pub nal_length_size: Option<String>,
    pub r_frame_rate: Option<String>,
    pub avg_frame_rate: Option<String>,
    pub time_base: Option<String>,
    pub start_pts: Option<u32>,
    pub start_time: Option<String>,
    pub bits_per_raw_sample: Option<String>,

    #[serde(default)]
    #[serde(deserialize_with = "str_to_u32")]
    pub sample_rate: Option<u32>,
    pub extradata_size: Option<u32>,
    pub disposition: Option<FFProbeDisposition>,
    pub tags: Option<FFProbeTags>,
}

#[derive(Debug, Deserialize)]
pub struct FFProbeDisposition {
    pub default: u32,
    pub dub: u32,
    pub original: u32,
    pub comment: u32,
    pub lyrics: u32,
    pub karaoke: u32,
    pub forced: u32,
    pub hearing_impaired: u32,
    pub visual_impaired: u32,
    pub clean_effects: u32,
    pub attached_pic: u32,
    pub timed_thumbnails: u32,
    pub captions: u32,
    pub descriptions: u32,
    pub metadata: u32,
    pub dependent: u32,
    pub still_image: u32,
}

#[derive(Debug, Deserialize)]
pub struct FFProbeTags {
    #[serde(rename = "DURATION")]
    pub duration: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct FFProbeFormat {
    pub filename: String,
    pub nb_streams: u32,
    pub nb_programs: u32,
    pub format_name: String,
    pub format_long_name: String,
    pub start_time: String,
    #[serde(default)]
    #[serde(deserialize_with = "str_to_f32")]
    pub duration: Option<f32>,
    #[serde(default)]
    #[serde(deserialize_with = "str_to_u64")]
    pub size: Option<u64>,
    #[serde(default)]
    #[serde(deserialize_with = "str_to_u32")]
    pub bit_rate: Option<u32>,
    pub probe_score: u32,
    pub tags: FFProbeTags,
}

#[derive(Debug, Deserialize)]
pub struct FFProbeOutput {
    pub streams: Vec<FFProbeStream>,
    pub format: FFProbeFormat,
}

pub async fn probe(path: &str) -> Result<FFProbeOutput, Box<dyn std::error::Error>> {
    tracing::info!("Probing {}", path);

    let output = match Command::new("ffprobe")
        .arg("-v")
        .arg("quiet")
        .arg("-print_format")
        .arg("json")
        .arg("-show_format")
        .arg("-show_streams")
        .arg(path)
        .output()
        .await
    {
        Ok(output) => output,
        Err(_) => return Err("Failed to execute ffprobe".into()),
    };

    let output = String::from_utf8_lossy(&output.stdout);

    match serde_json::from_str(&output) {
        Ok(output) => Ok(output),
        Err(err) => {
            tracing::error!("Failed to parse ffprobe output: {}", err);
            return Err("Failed to parse ffprobe output".into());
        }
    }
}
