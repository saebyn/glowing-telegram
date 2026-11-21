use std::fmt::{self, Display};
use std::path::Path;
use std::process::Stdio;
use std::{collections::HashMap, rc::Rc};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;

use types::{
    AudioChannelKeyframe, AudioChannelMixing, CutList, InputMedia,
    OutputTrack, SourceAudioRoute, TransitionInType,
};

/// The `volume` filter defaults to single-precision floating-point samples.
pub const MIN_NONZERO_FFMPEG_GAIN: f64 = f32::from_bits(1) as f64;
pub const MAX_FFMPEG_GAIN: f64 = f32::MAX as f64;

/// Reject gains that FFmpeg's default float volume processing cannot express.
pub fn validate_ffmpeg_gain(gain: f64) -> Result<(), &'static str> {
    if !gain.is_finite() || gain < 0.0 {
        return Err("gain must be finite and nonnegative");
    }
    if gain > MAX_FFMPEG_GAIN || (gain > 0.0 && gain < MIN_NONZERO_FFMPEG_GAIN)
    {
        return Err("gain is outside FFmpeg's nonzero 32-bit float range");
    }
    Ok(())
}

fn parse_time_base(value: Option<&str>) -> Option<f64> {
    let (n, d) = value?.split_once('/')?;
    let numerator = n.parse::<f64>().ok()?;
    let denominator = d.parse::<f64>().ok()?;
    if numerator <= 0.0 || denominator <= 0.0 {
        return None;
    }
    let value = numerator / denominator;
    (value.is_finite() && value > 0.0).then_some(value)
}

async fn collect_probe_errors(
    mut stderr: tokio::process::ChildStderr,
) -> std::io::Result<String> {
    let mut buffer = [0_u8; 4096];
    let mut captured = Vec::new();
    loop {
        let read = stderr.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        let remaining = 4096_usize.saturating_sub(captured.len());
        captured.extend_from_slice(&buffer[..read.min(remaining)]);
    }
    Ok(String::from_utf8_lossy(&captured).trim().to_owned())
}

async fn finish_probe_scan(
    child: &mut tokio::process::Child,
) -> Result<(), String> {
    let status = child
        .wait()
        .await
        .map_err(|e| format!("could not wait for ffprobe: {e}"))?;
    if !status.success() {
        return Err(format!("ffprobe exited with {status}"));
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct AudioCoverage {
    start: f64,
    end: f64,
    tolerance: f64,
}

async fn validate_audio_timestamps(
    path: &Path,
    stream: usize,
    video_start: f64,
    end: f64,
) -> Result<AudioCoverage, String> {
    let selector = format!("a:{stream}");
    let metadata = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            &selector,
            "-show_entries",
            "stream=start_time,sample_rate,time_base",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .await
        .map_err(|e| format!("could not probe audio stream {stream}: {e}"))?;
    if !metadata.status.success() {
        return Err(format!("could not probe audio stream {stream}"));
    }
    let value: serde_json::Value = serde_json::from_slice(&metadata.stdout)
        .map_err(|e| format!("invalid audio stream {stream} metadata: {e}"))?;
    let metadata = value
        .get("streams")
        .and_then(|s| s.as_array())
        .and_then(|s| s.first())
        .ok_or_else(|| format!("audio stream {stream} metadata is missing"))?;
    let start = metadata
        .get("start_time")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|v| v.is_finite())
        .ok_or_else(|| {
            format!(
                "audio stream {stream} start timestamp is missing or invalid"
            )
        })?;
    let rate = metadata
        .get("sample_rate")
        .and_then(|v| v.as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|v| v.is_finite() && *v > 0.0)
        .ok_or_else(|| {
            format!("audio stream {stream} sample rate is missing or invalid")
        })?;
    let time_base =
        parse_time_base(metadata.get("time_base").and_then(|v| v.as_str()))
            .ok_or_else(|| {
                format!(
                    "audio stream {stream} time base is missing or invalid"
                )
            })?;
    if time_base > 0.001 {
        return Err(format!(
            "audio stream {stream} time base is too coarse to establish timing"
        ));
    }
    if start < video_start - time_base {
        return Err(format!(
            "audio stream {stream} starts before the video; source timing is unsupported"
        ));
    }
    let mut child = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            &selector,
            "-show_frames",
            "-show_entries",
            "frame=pts_time,best_effort_timestamp_time,nb_samples",
            "-of",
            "compact=p=1:nk=0",
        ])
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| format!("could not scan audio stream {stream}: {e}"))?;
    let stdout = child.stdout.take().ok_or_else(|| {
        format!("could not capture audio stream {stream} frames")
    })?;
    let stderr = child.stderr.take().ok_or_else(|| {
        format!("could not capture audio stream {stream} errors")
    })?;
    let errors_task = tokio::spawn(collect_probe_errors(stderr));
    let mut lines = BufReader::new(stdout).lines();
    let mut last_end: Option<f64> = None;
    while let Some(line) = lines.next_line().await.map_err(|e| {
        format!("could not read audio stream {stream} frames: {e}")
    })? {
        if line.is_empty() {
            continue;
        }
        if line != "frame" && !line.starts_with("frame|") {
            return Err(format!(
                "audio stream {stream} frame output is malformed"
            ));
        }
        let mut timestamp = None;
        let mut pts = None;
        let mut samples = None;
        for field in line.split('|') {
            if let Some(value) =
                field.strip_prefix("best_effort_timestamp_time=")
            {
                timestamp =
                    value.parse::<f64>().ok().filter(|v| v.is_finite());
            }
            if let Some(value) = field.strip_prefix("pts_time=") {
                pts = value.parse::<f64>().ok().filter(|v| v.is_finite());
            }
            if let Some(value) = field.strip_prefix("nb_samples=") {
                samples = value.parse::<u64>().ok().filter(|v| *v > 0);
            }
        }
        let timestamp = timestamp.ok_or_else(|| {
            format!(
                "audio stream {stream} frame timestamp is missing or invalid"
            )
        })?;
        let pts = pts.ok_or_else(|| {
            format!("audio stream {stream} frame PTS is missing or invalid")
        })?;
        if (pts - timestamp).abs() > 0.000_001 {
            return Err(format!(
                "audio stream {stream} frame timestamp disagrees with PTS"
            ));
        }
        let samples = samples.ok_or_else(|| format!("audio stream {stream} frame sample count is missing or invalid"))?;
        if let Some(previous_end) = last_end {
            if (timestamp - previous_end).abs()
                > time_base + 1.0 / rate + 0.000_001
            {
                return Err(format!(
                    "audio stream {stream} has a discontinuous timestamp near {timestamp}"
                ));
            }
        } else if (timestamp - start).abs() > time_base + 1.0 / rate {
            return Err(format!(
                "audio stream {stream} first frame timestamp disagrees with its start"
            ));
        }
        last_end = Some(timestamp + samples as f64 / rate);
        if timestamp >= end {
            break;
        }
    }
    while lines
        .next_line()
        .await
        .map_err(|e| format!("could not drain audio stream {stream}: {e}"))?
        .is_some()
    {}
    finish_probe_scan(&mut child)
        .await
        .map_err(|e| format!("audio stream {stream}: {e}"))?;
    let errors = errors_task
        .await
        .map_err(|e| {
            format!("audio stream {stream} error reader failed: {e}")
        })?
        .map_err(|e| {
            format!("could not read audio stream {stream} errors: {e}")
        })?;
    if !errors.is_empty() {
        return Err(format!(
            "audio stream {stream} decoder reported: {errors}"
        ));
    }
    let end = last_end.ok_or_else(|| {
        format!("audio stream {stream} has no timestamped frames")
    })?;
    Ok(AudioCoverage {
        start,
        end,
        tolerance: time_base + 1.0 / rate + 0.000_001,
    })
}

pub async fn validate_source_audio(
    data: &CutList,
    input_dir: &Path,
    frame_rate: f32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    for (index, channel) in data.audio_mixing.iter().flatten().enumerate() {
        for keyframe in channel.keyframes.iter().flatten() {
            if keyframe.frame < 0 {
                return Err(std::io::Error::other(format!(
                    "audioMixing[{index}] has a negative keyframe frame"
                ))
                .into());
            }
            validate_ffmpeg_gain(keyframe.volume).map_err(|error| std::io::Error::other(format!("audioMixing[{index}] has an invalid keyframe gain: {error}")))?;
        }
    }
    if !data
        .output_track
        .iter()
        .any(|track| track.source_audio.is_some())
    {
        return Ok(());
    }
    validate_minimum_filter_size(data)?;
    // A short or delayed uncorrected intro/outro would shift concatenated audio
    // independently of video and invalidate every later output-time envelope.
    // Validate those tracks against their unchanged first-audio-stream path.
    let mut all_tracks = data.clone();
    let mut legacy_tracks = Vec::with_capacity(data.output_track.len());
    for track in &mut all_tracks.output_track {
        legacy_tracks.push(track.source_audio.is_none());
        if track.source_audio.is_none() {
            track.source_audio = Some(vec![
                SourceAudioRoute {
                    stream_index: 0,
                    source_channel: 0,
                    output_channel: 0,
                    keyframes: None,
                    points: None,
                },
                SourceAudioRoute {
                    stream_index: 0,
                    source_channel: 0,
                    output_channel: 1,
                    keyframes: None,
                    points: None,
                },
            ]);
        }
    }
    validate_source_audio_inner(
        &all_tracks,
        input_dir,
        frame_rate,
        &legacy_tracks,
    )
    .await?;
    validate_filter_argument_size(data, frame_rate)?;
    Ok(())
}

async fn validate_source_audio_inner(
    data: &CutList,
    input_dir: &Path,
    frame_rate: f32,
    legacy_tracks: &[bool],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if !frame_rate.is_finite() || frame_rate <= 0.0 {
        return Err(std::io::Error::other("invalid output frame rate").into());
    }
    if data
        .output_track
        .iter()
        .any(|track| track.source_audio.is_some())
    {
        if data.input_media.len() > 255 || data.output_track.len() > 255 {
            return Err(std::io::Error::other(
                "too many input or output tracks",
            )
            .into());
        }
        let mut duration = 0_i64;
        let mut previous_length = 0_i64;
        for (index, track) in data.output_track.iter().enumerate() {
            let fail = || {
                std::io::Error::other(format!(
                    "outputTrack[{index}] has an invalid timeline or section reference"
                ))
            };
            let media = usize::try_from(track.media_index)
                .ok()
                .and_then(|i| data.input_media.get(i))
                .ok_or_else(fail)?;
            let section = usize::try_from(track.section_index)
                .ok()
                .and_then(|i| media.sections.get(i))
                .ok_or_else(fail)?;
            let length = section
                .end_frame
                .checked_sub(section.start_frame)
                .filter(|length| section.start_frame >= 0 && *length > 0)
                .ok_or_else(fail)?;
            if index > 0 {
                if let Some(transition) = track
                    .transition_in
                    .as_ref()
                    .filter(|t| t.transition_type == TransitionInType::Fade)
                {
                    if transition.duration <= 0
                        || transition.duration > length
                        || transition.duration > previous_length
                    {
                        return Err(fail().into());
                    }
                    duration = duration
                        .checked_sub(transition.duration)
                        .ok_or_else(fail)?;
                }
            }
            duration = duration.checked_add(length).ok_or_else(fail)?;
            previous_length = length;
        }
    }
    let mut probes: HashMap<usize, Vec<Option<u64>>> = HashMap::new();
    let mut validated_video_frames: HashMap<usize, i64> = HashMap::new();
    let mut validated_audio_until: HashMap<
        (usize, usize),
        (f64, AudioCoverage),
    > = HashMap::new();
    for (track_index, track) in data.output_track.iter().enumerate() {
        let Some(routes) = &track.source_audio else {
            continue;
        };
        let fail = |message: String| {
            std::io::Error::other(format!(
                "outputTrack[{track_index}].sourceAudio: {message}"
            ))
        };
        if routes.is_empty()
            || !routes.iter().any(|r| r.output_channel == 0)
            || !routes.iter().any(|r| r.output_channel == 1)
        {
            return Err(fail(
                "routes must cover both stereo output channels".to_owned(),
            )
            .into());
        }
        let media_index = usize::try_from(track.media_index)
            .map_err(|_| fail("invalid media index".to_owned()))?;
        let media = data
            .input_media
            .get(media_index)
            .ok_or_else(|| fail("media index is out of bounds".to_owned()))?;
        let section_index = usize::try_from(track.section_index)
            .map_err(|_| fail("invalid section index".to_owned()))?;
        let section = media.sections.get(section_index).ok_or_else(|| {
            fail("section index is out of bounds".to_owned())
        })?;
        if section.start_frame < 0 || section.end_frame <= section.start_frame
        {
            return Err(
                fail("invalid video section frame range".to_owned()).into()
            );
        }
        if let std::collections::hash_map::Entry::Vacant(entry) =
            probes.entry(media_index)
        {
            let output = Command::new("ffprobe")
                .args([
                    "-v",
                    "error",
                    "-select_streams",
                    "a",
                    "-show_entries",
                    "stream=channels",
                    "-of",
                    "json",
                ])
                .arg(input_dir.join(&media.s3_location))
                .output()
                .await
                .map_err(|e| fail(format!("could not run ffprobe: {e}")))?;
            if !output.status.success() {
                return Err(fail(format!(
                    "ffprobe failed: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ))
                .into());
            }
            let parsed: serde_json::Value =
                serde_json::from_slice(&output.stdout).map_err(|e| {
                    fail(format!("invalid ffprobe output: {e}"))
                })?;
            let streams = parsed
                .get("streams")
                .and_then(|s| s.as_array())
                .ok_or_else(|| {
                    fail("ffprobe omitted audio streams".to_owned())
                })?;
            entry.insert(
                streams
                    .iter()
                    .map(|stream| {
                        stream.get("channels").and_then(|c| c.as_u64())
                    })
                    .collect(),
            );
        }
        let streams = &probes[&media_index];
        for (route_index, route) in routes.iter().enumerate() {
            let prefix = format!("route[{route_index}]");
            if let Some(points) = &route.points {
                if points.is_empty() || route.keyframes.is_some() {
                    return Err(fail(format!("{prefix} points must be nonempty and cannot be combined with keyframes")).into());
                }
                let mut previous = None;
                for point in points {
                    if !point.time.is_finite()
                        || point.time < 0.0
                        || previous.is_some_and(|time| point.time <= time)
                    {
                        return Err(fail(format!("{prefix} point times must be finite, nonnegative, and strictly increasing")).into());
                    }
                    validate_ffmpeg_gain(point.gain).map_err(|error| {
                        fail(format!(
                            "{prefix} has an invalid point gain: {error}"
                        ))
                    })?;
                    previous = Some(point.time);
                }
            }
            if !(0..=1).contains(&route.output_channel) {
                return Err(fail(format!(
                    "{prefix} output channel must be 0 or 1"
                ))
                .into());
            }
            let stream_index =
                usize::try_from(route.stream_index).map_err(|_| {
                    fail(format!("{prefix} has negative stream index"))
                })?;
            let channels = streams.get(stream_index).ok_or_else(|| fail(format!("{prefix} audio stream {stream_index} is missing ({} audio streams found)", streams.len())))?
                .ok_or_else(|| fail(format!("{prefix} audio stream {stream_index} has unknown channel count")))?;
            let channel =
                u64::try_from(route.source_channel).map_err(|_| {
                    fail(format!("{prefix} has negative source channel"))
                })?;
            if channel >= channels {
                return Err(fail(format!("{prefix} source channel {channel} is out of bounds for audio stream {stream_index} ({channels} channels)" )).into());
            }
            for keyframe in route.keyframes.iter().flatten() {
                if keyframe.frame < 0 {
                    return Err(fail(format!(
                        "{prefix} has an invalid keyframe"
                    ))
                    .into());
                }
                validate_ffmpeg_gain(keyframe.volume).map_err(|error| {
                    fail(format!(
                        "{prefix} has an invalid keyframe gain: {error}"
                    ))
                })?;
            }
        }
        // FFmpeg rebases each input by the container start. Our frame-based
        // trim assumes that rebased time zero is also the first video frame.
        // Refuse cases where that cannot be established instead of shifting
        // leading audio to the wrong video position.
        let video_probe = Command::new("ffprobe")
            .args([
                "-v",
                "error",
                "-select_streams",
                "v:0",
                "-show_entries",
                "stream=start_time,time_base:format=start_time",
                "-of",
                "json",
            ])
            .arg(input_dir.join(&media.s3_location))
            .output()
            .await
            .map_err(|e| fail(format!("could not probe video timing: {e}")))?;
        if !video_probe.status.success() {
            return Err(fail(
                "ffprobe failed to read video timing".to_owned(),
            )
            .into());
        }
        let timing: serde_json::Value =
            serde_json::from_slice(&video_probe.stdout).map_err(|e| {
                fail(format!("invalid video timing metadata: {e}"))
            })?;
        let parse_time = |value: Option<&serde_json::Value>| {
            value
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<f64>().ok())
                .filter(|n| n.is_finite())
        };
        let video_start = parse_time(
            timing
                .get("streams")
                .and_then(|s| s.as_array())
                .and_then(|s| s.first())
                .and_then(|s| s.get("start_time")),
        )
        .ok_or_else(|| {
            fail("video start time is missing or invalid".to_owned())
        })?;
        let container_start =
            parse_time(timing.get("format").and_then(|f| f.get("start_time")))
                .ok_or_else(|| {
                    fail(
                        "container start time is missing or invalid"
                            .to_owned(),
                    )
                })?;
        if (video_start - container_start).abs() > 0.000_001 {
            return Err(fail(format!("video start time {video_start} differs from container start time {container_start}; source timing is unsupported" )).into());
        }
        let required_video_frames = data
            .output_track
            .iter()
            .filter(|candidate| {
                candidate.media_index == track.media_index
                    && candidate.source_audio.is_some()
            })
            .filter_map(|candidate| {
                usize::try_from(candidate.section_index)
                    .ok()
                    .and_then(|index| media.sections.get(index))
            })
            .map(|section| section.end_frame)
            .max()
            .unwrap_or(section.end_frame);
        if validated_video_frames
            .get(&media_index)
            .copied()
            .unwrap_or(0)
            < required_video_frames
        {
            let time_base = parse_time_base(
                timing
                    .get("streams")
                    .and_then(|s| s.as_array())
                    .and_then(|s| s.first())
                    .and_then(|s| s.get("time_base"))
                    .and_then(|s| s.as_str()),
            )
            .ok_or_else(|| {
                fail("video time base is missing or invalid".to_owned())
            })?;
            if time_base > 0.001 || time_base >= 0.5 / f64::from(frame_rate) {
                return Err(fail(
                    "video time base is too coarse for the output frame rate"
                        .to_owned(),
                )
                .into());
            }
            let mut child = Command::new("ffprobe")
                .args([
                    "-v",
                    "error",
                    "-select_streams",
                    "v:0",
                    "-show_frames",
                    "-show_entries",
                    "frame=pts_time,best_effort_timestamp_time",
                    "-of",
                    "compact=p=1:nk=0",
                ])
                .arg(input_dir.join(&media.s3_location))
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true)
                .spawn()
                .map_err(|e| {
                    fail(format!("could not scan video frames: {e}"))
                })?;
            let stdout = child.stdout.take().ok_or_else(|| {
                fail("could not capture video frames".to_owned())
            })?;
            let stderr = child.stderr.take().ok_or_else(|| {
                fail("could not capture video scan errors".to_owned())
            })?;
            let errors_task = tokio::spawn(collect_probe_errors(stderr));
            let mut lines = BufReader::new(stdout).lines();
            let mut count = 0_i64;
            while count < required_video_frames {
                let Some(line) = lines.next_line().await.map_err(|e| {
                    fail(format!("could not read video frames: {e}"))
                })?
                else {
                    break;
                };
                if line.is_empty() {
                    continue;
                }
                if line != "frame" && !line.starts_with("frame|") {
                    return Err(fail(format!(
                        "video frame {count} output is malformed"
                    ))
                    .into());
                }
                let field = |name: &str| {
                    line.split('|')
                        .find_map(|field| field.strip_prefix(name))
                        .and_then(|s| s.parse::<f64>().ok())
                        .filter(|v| v.is_finite())
                };
                let pts = field("pts_time=").ok_or_else(|| {
                    fail(format!(
                        "video frame {count} has missing or invalid timestamp"
                    ))
                })?;
                let best_effort = field("best_effort_timestamp_time=")
                    .ok_or_else(|| fail(format!("video frame {count} has missing or invalid best-effort timestamp")))?;
                if (pts - best_effort).abs() > 0.000_001 {
                    return Err(fail(format!(
                        "video frame {count} timestamp disagrees with PTS"
                    ))
                    .into());
                }
                let expected =
                    video_start + count as f64 / f64::from(frame_rate);
                if (pts - expected).abs() > time_base + 0.000_001 {
                    return Err(fail(format!("video frame {count} timestamp {pts} does not match the {frame_rate} fps timeline" )).into());
                }
                count += 1;
            }
            if count < required_video_frames {
                return Err(fail(format!("video frame range ends at {required_video_frames} but only {count} frames were available")).into());
            }
            // Drain to EOF so a decoder failure cannot be hidden by killing
            // the probe immediately after the requested frame.
            while lines
                .next_line()
                .await
                .map_err(|e| fail(format!("could not drain video scan: {e}")))?
                .is_some()
            {}
            finish_probe_scan(&mut child)
                .await
                .map_err(|e| fail(format!("video frame scan: {e}")))?;
            let errors = errors_task
                .await
                .map_err(|e| fail(format!("video error reader failed: {e}")))?
                .map_err(|e| {
                    fail(format!("could not read video scan errors: {e}"))
                })?;
            if !errors.is_empty() {
                return Err(
                    fail(format!("video decoder reported: {errors}")).into()
                );
            }
            validated_video_frames.insert(media_index, count);
        }
        for route in routes {
            let stream_index = usize::try_from(route.stream_index)
                .map_err(|_| fail("negative audio stream index".to_owned()))?;
            let required_end_frame = data
                .output_track
                .iter()
                .filter(|candidate| {
                    candidate.media_index == track.media_index
                        && candidate.source_audio.as_ref().is_some_and(
                            |routes| {
                                routes.iter().any(|r| {
                                    r.stream_index == route.stream_index
                                })
                            },
                        )
                })
                .filter_map(|candidate| {
                    usize::try_from(candidate.section_index)
                        .ok()
                        .and_then(|index| media.sections.get(index))
                })
                .map(|section| section.end_frame)
                .max()
                .unwrap_or(section.end_frame);
            let section_end = video_start
                + required_end_frame as f64 / f64::from(frame_rate);
            let key = (media_index, stream_index);
            if validated_audio_until
                .get(&key)
                .is_none_or(|(end, _)| *end < section_end)
            {
                let coverage = validate_audio_timestamps(
                    &input_dir.join(&media.s3_location),
                    stream_index,
                    video_start,
                    section_end,
                )
                .await
                .map_err(&fail)?;
                validated_audio_until.insert(key, (section_end, coverage));
            }
            if legacy_tracks[track_index] {
                let (_, coverage) = validated_audio_until[&key];
                let start = video_start
                    + section.start_frame as f64 / f64::from(frame_rate);
                let end = video_start
                    + section.end_frame as f64 / f64::from(frame_rate);
                if coverage.start > start + coverage.tolerance
                    || coverage.end < end - coverage.tolerance
                {
                    return Err(fail(format!("legacy audio stream {stream_index} does not cover the complete video section")).into());
                }
            }
        }
    }
    Ok(())
}

fn validate_filter_argument_size(
    data: &CutList,
    frame_rate: f32,
) -> std::io::Result<()> {
    // Linux execve imposes MAX_ARG_STRLEN = 32 pages, including the NUL.
    // Estimate with maximum u32 output dimensions so the production graph
    // cannot be larger due to its resolution. Fail before launching FFmpeg.
    #[cfg(target_os = "linux")]
    {
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        if page_size <= 0 {
            return Err(std::io::Error::other(
                "could not determine the OS filter argument limit",
            ));
        }
        let limit = page_size as usize * 32;
        let command =
            build_ffmpeg_command(data, frame_rate, "", (u32::MAX, u32::MAX));
        let mut args = command.as_std().get_args();
        let filter = args
            .find(|arg| *arg == "-filter_complex")
            .and_then(|_| args.next())
            .ok_or_else(|| {
                std::io::Error::other("missing FFmpeg filter argument")
            })?;
        use std::os::unix::ffi::OsStrExt;
        if filter.as_bytes().len() >= limit {
            return Err(std::io::Error::other(format!(
                "FFmpeg filter argument is {} bytes; Linux permits fewer than {limit} bytes per argument. Reduce the generated graph or use a renderer supporting filter scripts",
                filter.as_bytes().len()
            )));
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = (data, frame_rate);
    Ok(())
}

fn validate_minimum_filter_size(data: &CutList) -> std::io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
        if page_size <= 0 {
            return Err(std::io::Error::other(
                "could not determine the OS filter argument limit",
            ));
        }
        let limit = page_size as usize * 32;
        // Every point in a nontrivial balanced curve contributes at least
        // three bytes of `if(`. Reject before cloning data or probing media.
        let minimum = data
            .output_track
            .iter()
            .flat_map(|track| track.source_audio.iter().flatten())
            .filter_map(|route| route.points.as_ref())
            .filter(|points| points.len() >= 2)
            .try_fold(0_usize, |total, points| {
                total.checked_add(points.len().saturating_mul(3))
            })
            .unwrap_or(usize::MAX);
        if minimum >= limit {
            return Err(std::io::Error::other(format!(
                "FFmpeg filter argument exceeds the Linux per-argument limit of {limit} bytes; use a renderer supporting filter scripts"
            )));
        }
    }
    #[cfg(not(target_os = "linux"))]
    let _ = data;
    Ok(())
}

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
    SourceAudioStream(u8, i64),
    SourceVideo(u8),
    MyAudio(u8),
    MyVideo(u8),
    OverlayVideo(u8),
    RouteAudio(usize, usize),
}

impl Display for FilterChannel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FilterChannel::SourceAudio(index) => write!(f, "[{}:a]", index),
            FilterChannel::SourceAudioStream(index, stream) => {
                write!(f, "[{index}:a:{stream}]")
            }
            FilterChannel::SourceVideo(index) => write!(f, "[{}:v]", index),
            FilterChannel::MyAudio(index) => write!(f, "[a{}]", index),
            FilterChannel::MyVideo(index) => write!(f, "[v{}]", index),
            FilterChannel::OverlayVideo(index) => write!(f, "[o{}]", index),
            FilterChannel::RouteAudio(track, route) => {
                write!(f, "[route{track}_{route}]")
            }
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
    let mut timeline_frame = 0_i64;
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
        if i > 0 {
            if let Some(transition) = &track.transition_in {
                if transition.transition_type == TransitionInType::Fade {
                    timeline_frame -= transition.duration;
                }
            }
        }
        if let Some(routes) = &track.source_audio {
            parts.extend(add_routed_audio(
                track.media_index as usize,
                section.start_frame,
                section.end_frame,
                i,
                frame_rate,
                timeline_frame,
                routes,
            ));
        } else {
            parts.push(add_audio_to_output_track(
                track.media_index as usize,
                section.start_frame,
                section.end_frame,
                i,
                frame_rate,
            ));
        }
        timeline_frame += section.end_frame - section.start_frame;
    }
    parts.extend(assemble_track_segments(
        &data.output_track,
        &data.input_media,
        frame_rate,
    ));
    parts
}

fn add_routed_audio(
    media_index: usize,
    start_frame: i64,
    end_frame: i64,
    track_index: usize,
    frame_rate: f32,
    timeline_frame: i64,
    routes: &[SourceAudioRoute],
) -> Vec<FilterPipe> {
    let mut parts = Vec::new();
    let start = start_frame as f64 / f64::from(frame_rate);
    let end = end_frame as f64 / f64::from(frame_rate);
    let duration = (end_frame - start_frame) as f64 / f64::from(frame_rate);
    for (i, route) in routes.iter().enumerate() {
        let time =
            format!("(t+{})", timeline_frame as f64 / f64::from(frame_rate));
        let gain_filter = if let Some(points) = &route.points {
            let expression = source_gain_expression(points, &time);
            Filter::new(
                "aeval",
                vec![("exprs", format!("'val(0)*({expression})'"))],
            )
        } else {
            let expression = volume_expression_at(
                route.keyframes.as_deref().unwrap_or(&[]),
                frame_rate,
                &time,
            );
            Filter::new(
                "volume",
                vec![
                    ("volume", format!("'{expression}'")),
                    ("eval", "frame".to_owned()),
                ],
            )
        };
        parts.push(FilterPipe::new(
            vec![
                Filter::new(
                    "atrim",
                    vec![
                        ("start", start.to_string()),
                        ("end", end.to_string()),
                    ],
                ),
                Filter::new(
                    "asetpts",
                    vec![(format!("PTS-{start}/TB").as_str(), "".to_owned())],
                ),
                Filter::new(
                    "aresample",
                    vec![
                        ("async", "1".to_owned()),
                        ("first_pts", "0".to_owned()),
                    ],
                ),
                Filter::new("apad", vec![("whole_dur", duration.to_string())]),
                Filter::new("atrim", vec![("end", duration.to_string())]),
                Filter::new(
                    "pan",
                    vec![("mono|c0", format!("c{}", route.source_channel))],
                ),
                gain_filter,
                Filter::new(
                    "pan",
                    vec![(
                        format!("stereo|c{}", route.output_channel).as_str(),
                        "c0".to_owned(),
                    )],
                ),
            ],
            vec![FilterChannel::SourceAudioStream(
                media_index as u8,
                route.stream_index,
            )],
            FilterChannel::RouteAudio(track_index, i),
        ));
    }
    parts.push(FilterPipe::new(
        vec![Filter::new(
            "amix",
            vec![
                ("inputs", routes.len().to_string()),
                ("normalize", "0".to_owned()),
            ],
        )],
        (0..routes.len())
            .map(|i| FilterChannel::RouteAudio(track_index, i))
            .collect(),
        FilterChannel::MyAudio(track_index as u8),
    ));
    parts
}

fn source_gain_expression(
    points: &[types::SourceAudioPoint],
    time: &str,
) -> String {
    fn segments(points: &[types::SourceAudioPoint], time: &str) -> String {
        if points.len() == 2 {
            let a = &points[0];
            let b = &points[1];
            return format!(
                "{}+({}-{})*({time}-{})/({}-{})",
                a.gain, b.gain, a.gain, a.time, b.time, a.time
            );
        }
        let mid = points.len() / 2;
        format!(
            "if(lt({time},{}),{},{})",
            points[mid].time,
            segments(&points[..=mid], time),
            segments(&points[mid..], time)
        )
    }
    let first = &points[0];
    let last = points.last().unwrap();
    if points.len() == 1 {
        return first.gain.to_string();
    }
    format!(
        "if(lt({time},{}),{},if(lt({time},{}),{},{}))",
        first.time,
        first.gain,
        last.time,
        segments(points, time),
        last.gain
    )
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

    let mut group_start = 0;
    let mut group_frames = 0_i64;
    for (i, track) in output_tracks.iter().enumerate() {
        let section = &input_media[track.media_index as usize].sections
            [track.section_index as usize];
        let section_frames = section.end_frame - section.start_frame;
        if let Some(transition) = &track.transition_in {
            if transition.transition_type == TransitionInType::Fade && i > 0 {
                let duration_sec =
                    convert_frame_to_time(transition.duration, frame_rate);
                let offset = convert_frame_to_time(
                    group_frames - transition.duration,
                    frame_rate,
                );
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
                        FilterChannel::MyVideo(group_start as u8),
                        FilterChannel::MyVideo(i as u8),
                    ],
                    FilterChannel::MyVideo(group_start as u8),
                ));

                parts.push(FilterPipe::new(
                    vec![Filter::new(
                        "acrossfade",
                        vec![("d", duration_sec.to_string())],
                    )],
                    vec![
                        FilterChannel::MyAudio(group_start as u8),
                        FilterChannel::MyAudio(i as u8),
                    ],
                    FilterChannel::MyAudio(group_start as u8),
                ));
                group_frames += section_frames - transition.duration;
            } else {
                group_start = i;
                group_frames = section_frames;
                video_streams.push(FilterChannel::MyVideo(i as u8));
                audio_streams.push(FilterChannel::MyAudio(i as u8));
            }
        } else {
            group_start = i;
            group_frames = section_frames;
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

/// Builds the final FFmpeg command. Callers must first await
/// [`validate_source_audio`] with the same input directory and frame rate;
/// otherwise explicit source routes can be rendered with incorrect timing.
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

    let mut filter =
        FilterGraph::new(data, frame_rate, resolution).to_string();
    if let Some(mixing) =
        data.audio_mixing.as_deref().filter(|m| !m.is_empty())
    {
        filter.push_str(&audio_mixing_filter(mixing, frame_rate));
    }

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

// Apply routing after concatenation and crossfades: keyframe positions refer to
// the output timeline, not the source media's timestamps.
fn audio_mixing_filter(
    mixing: &[AudioChannelMixing],
    frame_rate: f32,
) -> String {
    let channels = mixing
        .iter()
        .map(|m| m.output_channel)
        .max()
        .unwrap()
        .saturating_add(1);
    let mut graph = String::new();
    if mixing.len() > 1 {
        graph.push_str(&format!(";[a255]asplit={}", mixing.len()));
        for i in 0..mixing.len() {
            graph.push_str(&format!("[mix_in{i}]"));
        }
    }
    for (i, route) in mixing.iter().enumerate() {
        let input = if mixing.len() == 1 {
            "a255".to_owned()
        } else {
            format!("mix_in{i}")
        };
        let volume = volume_expression(
            route.keyframes.as_deref().unwrap_or(&[]),
            frame_rate,
        );
        graph.push_str(&format!(
            ";[{input}]pan=mono|c0=c{},volume='{}':eval=frame,pan={}c|c{}=c0[mix{i}]",
            route.source_channel, volume, channels, route.output_channel
        ));
    }
    graph.push(';');
    for i in 0..mixing.len() {
        graph.push_str(&format!("[mix{i}]"));
    }
    graph.push_str(&format!("amix=inputs={}:normalize=0[a255]", mixing.len()));
    graph
}

fn volume_expression(
    keyframes: &[AudioChannelKeyframe],
    frame_rate: f32,
) -> String {
    volume_expression_at(keyframes, frame_rate, "t")
}

fn volume_expression_at(
    keyframes: &[AudioChannelKeyframe],
    frame_rate: f32,
    time: &str,
) -> String {
    let mut keyframes: Vec<_> = keyframes.iter().collect();
    keyframes.sort_by_key(|keyframe| keyframe.frame);
    keyframes.dedup_by(|later, earlier| {
        if later.frame == earlier.frame {
            *earlier = *later;
            true
        } else {
            false
        }
    });
    let Some(last) = keyframes.last() else {
        return "1".to_owned();
    };
    let mut expression = last.volume.to_string();
    for pair in keyframes.windows(2).rev() {
        let start = pair[0].frame as f64 / f64::from(frame_rate);
        let end = pair[1].frame as f64 / f64::from(frame_rate);
        expression = format!(
            "if(lt({time},{end}),{}+({}-{})*({time}-{start})/({end}-{start}),{expression})",
            pair[0].volume, pair[1].volume, pair[0].volume
        );
    }
    let first = keyframes[0];
    format!(
        "if(lt({time},{}),{},{expression})",
        first.frame as f64 / f64::from(frame_rate),
        first.volume
    )
}

#[cfg(test)]
mod test {
    use pretty_assertions::assert_eq;
    use serde_json::json;
    use types::CutList;

    use super::{
        audio_mixing_filter, build_ffmpeg_command, volume_expression,
    };

    #[test]
    fn mixing_is_optional_and_duplicate_keyframes_use_the_last_value() {
        let mut data: CutList = serde_json::from_value(json!({
            "version": "1.0.0",
            "inputMedia": [{"s3Location": "a.mkv", "sections": [{"startFrame": 0, "endFrame": 60}]}],
            "outputTrack": [{"mediaIndex": 0, "sectionIndex": 0}]
        })).unwrap();
        let graph = |data: &CutList| {
            let cmd = build_ffmpeg_command(data, 60.0, "out.mp4", (96, 64));
            let args: Vec<_> = cmd
                .as_std()
                .get_args()
                .map(|a| a.to_string_lossy().to_string())
                .collect();
            args[args.iter().position(|a| a == "-filter_complex").unwrap() + 1]
                .clone()
        };
        let original = graph(&data);
        data.audio_mixing = Some(vec![]);
        assert_eq!(graph(&data), original);
        data.audio_mixing = Some(
            serde_json::from_value(json!([
                {"sourceChannel": 0, "outputChannel": 0}
            ]))
            .unwrap(),
        );
        assert!(
            graph(&data)
                .contains("[a255]pan=mono|c0=c0,volume='1':eval=frame")
        );

        let keys: Vec<types::AudioChannelKeyframe> =
            serde_json::from_value(json!([
                {"frame": 0, "volume": 0.0}, {"frame": 0, "volume": 0.5}
            ]))
            .unwrap();
        assert_eq!(volume_expression(&keys, 60.0), "if(lt(t,0),0.5,0.5)");
    }

    #[test]
    fn mixing_graph_routes_and_sums_channels() {
        let mixing: Vec<types::AudioChannelMixing> =
            serde_json::from_value(json!([
                {"sourceChannel": 0, "outputChannel": 1, "keyframes": [
                    {"frame": 60, "volume": 1.0}, {"frame": 0, "volume": 0.0}
                ]},
                {"sourceChannel": 1, "outputChannel": 0, "keyframes": [
                    {"frame": 0, "volume": 0.5}
                ]},
                {"sourceChannel": 0, "outputChannel": 0, "keyframes": [
                    {"frame": 0, "volume": 0.5}
                ]}
            ]))
            .unwrap();
        let graph = audio_mixing_filter(&mixing, 60.0);
        assert!(
            graph.starts_with(";[a255]asplit=3[mix_in0][mix_in1][mix_in2]")
        );
        assert!(graph.contains("[mix_in0]pan=mono|c0=c0,volume='if(lt(t,0),0,if(lt(t,1),0+(1-0)*(t-0)/(1-0),1))':eval=frame,pan=2c|c1=c0[mix0]"));
        assert!(graph.contains("[mix_in1]pan=mono|c0=c1,volume='if(lt(t,0),0.5,0.5)':eval=frame,pan=2c|c0=c0[mix1]"));
        assert!(graph.contains("[mix_in2]pan=mono|c0=c0,volume='if(lt(t,0),0.5,0.5)':eval=frame,pan=2c|c0=c0[mix2]"));
        assert!(
            graph.ends_with(
                "[mix0][mix1][mix2]amix=inputs=3:normalize=0[a255]"
            )
        );
    }

    #[test]
    fn consecutive_fades_chain_into_the_accumulated_result() {
        let data: CutList = serde_json::from_value(json!({
            "version": "1.0.0",
            "inputMedia": [{"s3Location": "a.mkv", "sections": [
                {"startFrame": 0, "endFrame": 360},
                {"startFrame": 360, "endFrame": 780},
                {"startFrame": 780, "endFrame": 1440}
            ]}],
            "outputTrack": [
                {"mediaIndex": 0, "sectionIndex": 0},
                {"mediaIndex": 0, "sectionIndex": 1,
                 "transitionIn": {"type": "fade", "duration": 120}},
                {"mediaIndex": 0, "sectionIndex": 2,
                 "transitionIn": {"type": "fade", "duration": 120}}
            ]
        }))
        .unwrap();
        let command =
            build_ffmpeg_command(&data, 60.0, "output.mp4", (96, 64));
        let args: Vec<_> = command
            .as_std()
            .get_args()
            .map(|v| v.to_string_lossy().to_string())
            .collect();
        let graph = &args
            [args.iter().position(|a| a == "-filter_complex").unwrap() + 1];
        assert!(graph.contains(
            "[v0][v1]xfade=duration=2:offset=4:transition=fade[v0]"
        ));
        assert!(graph.contains(
            "[v0][v2]xfade=duration=2:offset=9:transition=fade[v0]"
        ));
        assert!(graph.contains("[a0][a2]acrossfade=d=2[a0]"));
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
