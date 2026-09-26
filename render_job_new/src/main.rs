use std::collections::BTreeSet;
use std::env;
use std::path::Path;

use anyhow::{Context, Result, anyhow, bail, ensure};
use aws_config::BehaviorVersion;
use aws_sdk_s3::primitives::ByteStream;
use figment::{Figment, providers::Env};
use gt_ffmpeg::edit::build_ffmpeg_command;
use serde::Deserialize;
use tempfile::{NamedTempFile, TempDir};
use tokio::task::JoinSet;
use tokio_postgres::Client as PgClient;
use types::{
    AudioChannelKeyframe, AudioChannelMixing, CutList, CutListVersion,
    InputMedia, MediaSection, OutputTrack, OverlayTrack, OverlayTrackType,
    TransitionInClass, TransitionInType, TransitionOutClass,
};
use uuid::Uuid;

const FRAME_RATE: f32 = 60.0;
const FRAMES_PER_SECOND: i64 = 60;
const RESOLUTION: (u32, u32) = (2560, 1440);

#[derive(Deserialize)]
struct Config {
    input_bucket: String,
    output_bucket: String,
    media_domain: String,
    #[serde(flatten)]
    database: gt_postgres::PostgresConfig,
}

#[derive(Debug)]
struct RenderJob {
    id: String,
    batch_job_id: String,
    episode_id: String,
    tenant_id: String,
    cut_list: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StreamosaicCutList {
    version: String,
    input_media: Vec<StreamosaicInputMedia>,
    output_track: Vec<StreamosaicOutputTrack>,
    #[serde(default)]
    audio_mixing: Option<Vec<StreamosaicAudioMixing>>,
    #[serde(default)]
    overlay_tracks: Option<Vec<StreamosaicOverlayTrack>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StreamosaicInputMedia {
    s3_location: String,
    sections: Vec<StreamosaicMediaSection>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StreamosaicMediaSection {
    start_frame: i64,
    end_frame: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StreamosaicOutputTrack {
    media_index: i64,
    section_index: i64,
    #[serde(default)]
    transition_in: Option<StreamosaicTransition>,
    #[serde(default)]
    transition_out: Option<StreamosaicTransition>,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum StreamosaicTransitionType {
    Cut,
    Fade,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StreamosaicTransition {
    #[serde(rename = "type")]
    transition_type: StreamosaicTransitionType,
    duration_frames: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StreamosaicAudioMixing {
    source_channel: i64,
    output_channel: i64,
    #[serde(default)]
    keyframes: Option<Vec<StreamosaicAudioKeyframe>>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StreamosaicAudioKeyframe {
    frame: i64,
    volume: f64,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StreamosaicOverlayType {
    Alpha,
    Colorkey,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StreamosaicOverlayTrack {
    media_index: i64,
    section_index: i64,
    start_frame: i64,
    #[serde(rename = "type")]
    overlay_type: StreamosaicOverlayType,
    #[serde(default)]
    x: Option<f64>,
    #[serde(default)]
    y: Option<f64>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let render_job_id = exactly_one_argument()?;
    let batch_job_id =
        env::var("AWS_BATCH_JOB_ID").context("AWS_BATCH_JOB_ID is not set")?;
    let config: Config = Figment::new()
        .merge(Env::raw())
        .extract()
        .context("load configuration from environment")?;
    ensure!(!config.input_bucket.is_empty(), "INPUT_BUCKET is empty");
    ensure!(!config.output_bucket.is_empty(), "OUTPUT_BUCKET is empty");

    let aws_config =
        aws_config::defaults(BehaviorVersion::latest()).load().await;
    let s3 = aws_sdk_s3::Client::new(&aws_config);
    let mut postgres = gt_postgres::connect(&config.database, &aws_config)
        .await
        .context("connect to Postgres")?;
    let job =
        load_render_job(&mut postgres, &render_job_id, &batch_job_id).await?;

    if let Err(error) = process(&config, &s3, &mut postgres, &job).await {
        let message = format!("{error:#}");
        if let Err(update_error) =
            mark_failed(&mut postgres, &job, &message).await
        {
            tracing::error!(%update_error, "could not persist render failure");
            return Err(error.context(format!(
                "also failed to persist failure state: {update_error:#}"
            )));
        }
        return Err(error);
    }

    Ok(())
}

fn exactly_one_argument() -> Result<String> {
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "render_job_new".to_owned());
    let Some(render_job_id) = args.next() else {
        bail!("usage: {program} <renderJobId>");
    };
    ensure!(args.next().is_none(), "usage: {program} <renderJobId>");
    ensure!(!render_job_id.trim().is_empty(), "renderJobId is empty");
    Ok(render_job_id)
}

async fn load_render_job(
    client: &mut PgClient,
    render_job_id: &str,
    batch_job_id: &str,
) -> Result<RenderJob> {
    let transaction = client.transaction().await?;
    let exists = transaction
        .query_opt(
            "SELECT id FROM render_jobs WHERE id = $1 FOR UPDATE",
            &[&render_job_id],
        )
        .await
        .context("lock render job")?;
    ensure!(
        exists.is_some(),
        "render job {render_job_id} does not exist"
    );

    let row = transaction
        .query_opt(
            r"
            SELECT r.id, r.episode_id, r.tenant_id, e.cut_list
            FROM render_jobs AS r
            JOIN episodes AS e
              ON e.id = r.episode_id
             AND e.tenant_id = r.tenant_id
            WHERE r.id = $1
              AND r.status = 'running'
              AND r.glowing_telegram_id = $2
              AND e.status = 'rendering'
              AND e.cut_list IS NOT NULL
            ",
            &[&render_job_id, &batch_job_id],
        )
        .await
        .context("query running render job and episode")?
        .ok_or_else(|| {
            anyhow!(
                "render job {render_job_id} is not assigned to Batch job {batch_job_id}, is not running, has no matching rendering episode, or has no cut list"
            )
        })?;

    let job = RenderJob {
        id: row.get("id"),
        batch_job_id: batch_job_id.to_owned(),
        episode_id: row.get("episode_id"),
        tenant_id: row.get("tenant_id"),
        cut_list: row.get("cut_list"),
    };
    transaction.commit().await?;
    Ok(job)
}

async fn process(
    config: &Config,
    s3: &aws_sdk_s3::Client,
    postgres: &mut PgClient,
    job: &RenderJob,
) -> Result<()> {
    let streamosaic: StreamosaicCutList =
        serde_json::from_value(job.cut_list.clone())
            .context("deserialize Streamosaic cut list")?;
    let (cut_list, duration_frames) = normalize_cut_list(streamosaic)?;
    let output_key = output_key(&job.tenant_id, &job.episode_id, &job.id)?;
    let rendered_url = media_url(&config.media_domain, &output_key)?;

    let input_dir = tempfile::tempdir().context("create input directory")?;
    download_inputs(s3, &config.input_bucket, &cut_list, &input_dir).await?;

    let output = NamedTempFile::new().context("create output file")?;
    run_ffmpeg(&cut_list, input_dir.path(), output.path()).await?;
    upload_output(s3, &config.output_bucket, &output_key, output.path())
        .await?;

    let render_seconds = duration_seconds(duration_frames)?;
    mark_completed(postgres, job, &rendered_url, render_seconds).await
}

async fn download_inputs(
    s3: &aws_sdk_s3::Client,
    bucket: &str,
    cut_list: &CutList,
    input_dir: &TempDir,
) -> Result<()> {
    let keys: BTreeSet<_> = cut_list
        .input_media
        .iter()
        .map(|media| media.s3_location.clone())
        .collect();
    let mut downloads = JoinSet::new();

    for key in keys {
        let s3 = s3.clone();
        let bucket = bucket.to_owned();
        let destination = input_dir.path().join(&key);
        downloads.spawn(async move {
            let parent = destination.parent().ok_or_else(|| {
                anyhow!("input key has no parent directory: {key}")
            })?;
            tokio::fs::create_dir_all(parent)
                .await
                .with_context(|| format!("create directory for {key}"))?;
            let object = s3
                .get_object()
                .bucket(bucket)
                .key(&key)
                .send()
                .await
                .with_context(|| format!("download s3 input {key}"))?;
            let mut source = object.body.into_async_read();
            let mut file = tokio::fs::File::create(&destination)
                .await
                .with_context(|| format!("create local input {key}"))?;
            tokio::io::copy(&mut source, &mut file)
                .await
                .with_context(|| format!("write local input {key}"))?;
            Result::<()>::Ok(())
        });
    }

    while let Some(download) = downloads.join_next().await {
        download.context("input download task panicked")??;
    }
    Ok(())
}

async fn run_ffmpeg(
    cut_list: &CutList,
    input_dir: &Path,
    output: &Path,
) -> Result<()> {
    let output = output
        .to_str()
        .ok_or_else(|| anyhow!("output path is not valid UTF-8"))?;
    let mut command =
        build_ffmpeg_command(cut_list, FRAME_RATE, output, RESOLUTION);
    command.current_dir(input_dir);
    let result = command
        .output()
        .await
        .context("run ffmpeg render command")?;
    if !result.status.success() {
        let stderr = String::from_utf8_lossy(&result.stderr);
        bail!(
            "ffmpeg exited with {}: {}",
            result.status,
            stderr.trim().chars().take(4_000).collect::<String>()
        );
    }
    Ok(())
}

async fn upload_output(
    s3: &aws_sdk_s3::Client,
    bucket: &str,
    key: &str,
    output: &Path,
) -> Result<()> {
    s3.put_object()
        .bucket(bucket)
        .key(key)
        .content_type("video/mp4")
        .body(
            ByteStream::from_path(output)
                .await
                .context("open rendered output for upload")?,
        )
        .send()
        .await
        .with_context(|| {
            format!("upload rendered output to s3://{bucket}/{key}")
        })?;
    Ok(())
}

async fn mark_completed(
    client: &mut PgClient,
    job: &RenderJob,
    rendered_url: &str,
    render_seconds: i32,
) -> Result<()> {
    let transaction = client.transaction().await?;
    transaction
        .query_one(
            "SELECT id FROM episodes WHERE id = $1 AND tenant_id = $2 FOR UPDATE",
            &[&job.episode_id, &job.tenant_id],
        )
        .await?;
    let render_jobs = transaction
        .execute(
            r"
            UPDATE render_jobs
            SET status = 'completed', progress = 100, error_message = NULL,
                updated_at = NOW()
            WHERE id = $1 AND episode_id = $2 AND tenant_id = $3
              AND status = 'running' AND glowing_telegram_id = $4
            ",
            &[&job.id, &job.episode_id, &job.tenant_id, &job.batch_job_id],
        )
        .await?;
    ensure!(render_jobs == 1, "render job is no longer running");

    let episodes = transaction
        .execute(
            r"
            UPDATE episodes
            SET status = 'rendered', rendered_hls_url = $1, updated_at = NOW()
            WHERE id = $2 AND tenant_id = $3 AND status = 'rendering'
              AND NOT EXISTS (SELECT 1 FROM render_jobs
                  WHERE episode_id = $2 AND tenant_id = $3
                    AND id <> $4 AND status = 'running')
            ",
            &[&rendered_url, &job.episode_id, &job.tenant_id, &job.id],
        )
        .await?;
    ensure!(episodes == 1, "episode is no longer rendering");

    transaction
        .execute(
            r"
            INSERT INTO usage_records
                (id, tenant_id, resource_type, render_job_id, render_seconds)
            VALUES ($1, $2, 'render_minutes', $3, $4)
            ",
            &[
                &Uuid::now_v7().to_string(),
                &job.tenant_id,
                &job.id,
                &render_seconds,
            ],
        )
        .await?;
    transaction.commit().await?;
    Ok(())
}

async fn mark_failed(
    client: &mut PgClient,
    job: &RenderJob,
    error: &str,
) -> Result<()> {
    let error: String = error.chars().take(4_000).collect();
    let transaction = client.transaction().await?;
    transaction
        .query_one(
            "SELECT id FROM episodes WHERE id = $1 AND tenant_id = $2 FOR UPDATE",
            &[&job.episode_id, &job.tenant_id],
        )
        .await?;
    let render_jobs = transaction
        .execute(
            r"
            UPDATE render_jobs
            SET status = 'failed', error_message = $1, updated_at = NOW()
            WHERE id = $2 AND episode_id = $3 AND tenant_id = $4
              AND status = 'running' AND glowing_telegram_id = $5
            ",
            &[
                &error,
                &job.id,
                &job.episode_id,
                &job.tenant_id,
                &job.batch_job_id,
            ],
        )
        .await?;
    ensure!(render_jobs == 1, "render job is no longer running");

    let episodes = transaction
        .execute(
            r"
            UPDATE episodes
            SET status = 'approved', updated_at = NOW()
            WHERE id = $1 AND tenant_id = $2 AND status = 'rendering'
              AND NOT EXISTS (SELECT 1 FROM render_jobs
                  WHERE episode_id = $1 AND tenant_id = $2
                    AND id <> $3 AND status = 'running')
            ",
            &[&job.episode_id, &job.tenant_id, &job.id],
        )
        .await?;
    ensure!(episodes == 1, "episode is no longer rendering");
    transaction.commit().await?;
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn normalize_cut_list(source: StreamosaicCutList) -> Result<(CutList, i64)> {
    ensure!(source.version == "1.0.0", "unsupported cut list version");
    ensure!(
        !source.input_media.is_empty(),
        "inputMedia must not be empty"
    );
    ensure!(
        source.input_media.len() <= 255,
        "too many input media items"
    );
    ensure!(
        !source.output_track.is_empty(),
        "outputTrack must not be empty"
    );
    ensure!(
        source.output_track.len() <= 255,
        "too many output track items"
    );
    ensure!(
        source.overlay_tracks.as_ref().map_or(0, Vec::len) <= 255,
        "too many overlay track items"
    );

    for (media_index, media) in source.input_media.iter().enumerate() {
        validate_s3_key(&media.s3_location).with_context(|| {
            format!("invalid inputMedia[{media_index}].s3Location")
        })?;
        ensure!(
            !media.sections.is_empty(),
            "inputMedia[{media_index}].sections must not be empty"
        );
        for (section_index, section) in media.sections.iter().enumerate() {
            ensure!(
                section.start_frame >= 0
                    && section.end_frame > section.start_frame,
                "invalid inputMedia[{media_index}].sections[{section_index}] frame range"
            );
        }
    }

    let mut duration_frames = 0_i64;
    for (index, track) in source.output_track.iter().enumerate() {
        let section = referenced_section(
            &source,
            track.media_index,
            track.section_index,
        )
        .with_context(|| format!("invalid outputTrack[{index}] reference"))?;
        let section_frames = section.end_frame - section.start_frame;
        duration_frames = duration_frames
            .checked_add(section_frames)
            .ok_or_else(|| anyhow!("timeline duration overflow"))?;
        validate_transition(
            track.transition_in.as_ref(),
            section_frames,
            index,
            "transitionIn",
        )?;
        validate_transition(
            track.transition_out.as_ref(),
            section_frames,
            index,
            "transitionOut",
        )?;

        if index > 0 {
            if let Some(transition) = &track.transition_in {
                if transition.transition_type
                    == StreamosaicTransitionType::Fade
                {
                    let previous = &source.output_track[index - 1];
                    let previous_section = referenced_section(
                        &source,
                        previous.media_index,
                        previous.section_index,
                    )?;
                    ensure!(
                        transition.duration_frames
                            <= previous_section.end_frame
                                - previous_section.start_frame,
                        "outputTrack[{index}].transitionIn exceeds previous section"
                    );
                    duration_frames -= transition.duration_frames;
                }
            }
        }
    }
    ensure!(duration_frames > 0, "timeline duration must be positive");

    if let Some(overlays) = &source.overlay_tracks {
        for (index, overlay) in overlays.iter().enumerate() {
            referenced_section(
                &source,
                overlay.media_index,
                overlay.section_index,
            )
            .with_context(|| {
                format!("invalid overlayTracks[{index}] reference")
            })?;
            ensure!(
                overlay.start_frame >= 0,
                "overlayTracks[{index}].startFrame is negative"
            );
            ensure!(
                overlay.x.is_none_or(f64::is_finite),
                "overlayTracks[{index}].x is not finite"
            );
            ensure!(
                overlay.y.is_none_or(f64::is_finite),
                "overlayTracks[{index}].y is not finite"
            );
        }
    }

    if let Some(mixing) = &source.audio_mixing {
        for (index, channel) in mixing.iter().enumerate() {
            ensure!(
                channel.source_channel >= 0 && channel.output_channel >= 0,
                "audioMixing[{index}] has a negative channel"
            );
            for keyframe in channel.keyframes.iter().flatten() {
                ensure!(
                    keyframe.frame >= 0
                        && keyframe.volume.is_finite()
                        && keyframe.volume >= 0.0,
                    "audioMixing[{index}] has an invalid keyframe"
                );
            }
        }
    }

    let cut_list = CutList {
        version: CutListVersion::The100,
        input_media: source
            .input_media
            .into_iter()
            .map(|media| InputMedia {
                s3_location: media.s3_location,
                sections: media
                    .sections
                    .into_iter()
                    .map(|section| MediaSection {
                        start_frame: section.start_frame,
                        end_frame: section.end_frame,
                    })
                    .collect(),
            })
            .collect(),
        output_track: source
            .output_track
            .into_iter()
            .map(|track| OutputTrack {
                media_index: track.media_index,
                section_index: track.section_index,
                transition_in: track.transition_in.map(|transition| {
                    TransitionInClass {
                        transition_type: transition.transition_type.into(),
                        duration: transition.duration_frames,
                    }
                }),
                transition_out: track.transition_out.map(|transition| {
                    TransitionOutClass {
                        transition_type: transition.transition_type.into(),
                        duration: transition.duration_frames,
                    }
                }),
            })
            .collect(),
        audio_mixing: source.audio_mixing.map(|mixing| {
            mixing
                .into_iter()
                .map(|channel| AudioChannelMixing {
                    source_channel: channel.source_channel,
                    output_channel: channel.output_channel,
                    keyframes: channel.keyframes.map(|keyframes| {
                        keyframes
                            .into_iter()
                            .map(|keyframe| AudioChannelKeyframe {
                                frame: keyframe.frame,
                                volume: keyframe.volume,
                            })
                            .collect()
                    }),
                })
                .collect()
        }),
        overlay_tracks: source.overlay_tracks.map(|overlays| {
            overlays
                .into_iter()
                .map(|overlay| OverlayTrack {
                    media_index: overlay.media_index,
                    section_index: overlay.section_index,
                    start_frame: overlay.start_frame,
                    overlay_track_type: overlay.overlay_type.into(),
                    x: overlay.x,
                    y: overlay.y,
                })
                .collect()
        }),
    };

    Ok((cut_list, duration_frames))
}

fn referenced_section(
    source: &StreamosaicCutList,
    media_index: i64,
    section_index: i64,
) -> Result<&StreamosaicMediaSection> {
    let media_index =
        usize::try_from(media_index).context("negative mediaIndex")?;
    let section_index =
        usize::try_from(section_index).context("negative sectionIndex")?;
    source
        .input_media
        .get(media_index)
        .and_then(|media| media.sections.get(section_index))
        .ok_or_else(|| anyhow!("media or section index is out of range"))
}

fn validate_transition(
    transition: Option<&StreamosaicTransition>,
    section_frames: i64,
    track_index: usize,
    field: &str,
) -> Result<()> {
    if let Some(transition) = transition {
        ensure!(
            transition.duration_frames >= 0
                && transition.duration_frames <= section_frames,
            "outputTrack[{track_index}].{field}.durationFrames is invalid"
        );
        if transition.transition_type == StreamosaicTransitionType::Fade {
            ensure!(
                transition.duration_frames > 0,
                "outputTrack[{track_index}].{field} fade duration must be positive"
            );
        }
    }
    Ok(())
}

fn validate_s3_key(key: &str) -> Result<()> {
    ensure!(!key.is_empty(), "key is empty");
    ensure!(
        !key.contains('\\') && !key.contains('\0'),
        "key contains an unsafe character"
    );
    ensure!(
        key.split('/')
            .all(|part| !part.is_empty() && part != "." && part != ".."),
        "key must be a safe relative path"
    );
    Ok(())
}

fn output_key(
    tenant_id: &str,
    episode_id: &str,
    render_job_id: &str,
) -> Result<String> {
    for (name, value) in [
        ("tenantId", tenant_id),
        ("episodeId", episode_id),
        ("renderJobId", render_job_id),
    ] {
        ensure!(
            !value.is_empty()
                && value != "."
                && value != ".."
                && !value.contains(['/', '\\', '\0']),
            "{name} is not safe for an S3 key"
        );
    }
    Ok(format!(
        "new/renders/{tenant_id}/{episode_id}/{render_job_id}.mp4"
    ))
}

fn media_url(media_domain: &str, key: &str) -> Result<String> {
    let domain = media_domain.trim().trim_matches('/');
    ensure!(!domain.is_empty(), "MEDIA_DOMAIN is empty");
    ensure!(
        !domain.contains("://") && !domain.contains(['/', '\\']),
        "MEDIA_DOMAIN must be a host name"
    );
    Ok(format!("https://{domain}/{key}"))
}

fn duration_seconds(duration_frames: i64) -> Result<i32> {
    ensure!(duration_frames > 0, "timeline duration must be positive");
    let seconds = duration_frames
        .checked_add(FRAMES_PER_SECOND - 1)
        .ok_or_else(|| anyhow!("timeline duration overflow"))?
        / FRAMES_PER_SECOND;
    i32::try_from(seconds).context("timeline duration exceeds database range")
}

impl From<StreamosaicTransitionType> for TransitionInType {
    fn from(value: StreamosaicTransitionType) -> Self {
        match value {
            StreamosaicTransitionType::Cut => Self::Cut,
            StreamosaicTransitionType::Fade => Self::Fade,
        }
    }
}

impl From<StreamosaicOverlayType> for OverlayTrackType {
    fn from(value: StreamosaicOverlayType) -> Self {
        match value {
            StreamosaicOverlayType::Alpha => Self::Alpha,
            StreamosaicOverlayType::Colorkey => Self::Colorkey,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn cut_list_json() -> serde_json::Value {
        json!({
            "version": "1.0.0",
            "inputMedia": [{
                "s3Location": "new/tenant/stream/clip.mp4",
                "sections": [
                    { "startFrame": 60, "endFrame": 180 },
                    { "startFrame": 240, "endFrame": 420 }
                ]
            }],
            "outputTrack": [
                { "mediaIndex": 0, "sectionIndex": 0 },
                {
                    "mediaIndex": 0,
                    "sectionIndex": 1,
                    "transitionIn": { "type": "fade", "durationFrames": 30 },
                    "transitionOut": { "type": "cut", "durationFrames": 0 }
                }
            ]
        })
    }

    #[test]
    fn normalizes_duration_frames_and_computes_timeline() {
        let source: StreamosaicCutList =
            serde_json::from_value(cut_list_json()).unwrap();
        let (normalized, frames) = normalize_cut_list(source).unwrap();

        assert_eq!(frames, 270);
        assert_eq!(
            normalized.output_track[1]
                .transition_in
                .as_ref()
                .unwrap()
                .duration,
            30
        );
        assert_eq!(
            normalized.output_track[1]
                .transition_out
                .as_ref()
                .unwrap()
                .duration,
            0
        );
    }

    #[test]
    fn rejects_unsafe_keys_and_invalid_references() {
        let mut value = cut_list_json();
        value["inputMedia"][0]["s3Location"] = json!("../secret.mp4");
        let source: StreamosaicCutList =
            serde_json::from_value(value).unwrap();
        assert!(normalize_cut_list(source).is_err());

        let mut value = cut_list_json();
        value["outputTrack"][0]["sectionIndex"] = json!(9);
        let source: StreamosaicCutList =
            serde_json::from_value(value).unwrap();
        assert!(normalize_cut_list(source).is_err());
    }

    #[test]
    fn creates_deterministic_key_and_https_url() {
        let key = output_key("tenant", "episode", "job").unwrap();
        assert_eq!(key, "new/renders/tenant/episode/job.mp4");
        assert_eq!(
            media_url("media.example.com", &key).unwrap(),
            "https://media.example.com/new/renders/tenant/episode/job.mp4"
        );
        assert!(output_key("../tenant", "episode", "job").is_err());
        assert!(media_url("https://media.example.com", &key).is_err());
    }

    #[test]
    fn rounds_partial_timeline_seconds_up_at_sixty_fps() {
        assert_eq!(duration_seconds(60).unwrap(), 1);
        assert_eq!(duration_seconds(61).unwrap(), 2);
        assert_eq!(duration_seconds(3_600).unwrap(), 60);
        assert!(duration_seconds(0).is_err());
    }
}
