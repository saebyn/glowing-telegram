use std::sync::Arc;

use aws_config::BehaviorVersion;
use aws_sdk_batch::Client as BatchClient;
use figment::{Figment, providers::Env};
use gt_postgres::PostgresConfig;
use lambda_runtime::{Error, LambdaEvent, run, service_fn};
use serde::{Deserialize, Deserializer, Serialize};
use thiserror::Error;
use tokio_postgres::{Client as PgClient, Transaction};

#[derive(Clone, Debug, Deserialize)]
struct Config {
    #[serde(flatten)]
    postgres: PostgresConfig,
    render_job_definition: String,
    render_job_queue: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct StartRenderRequest {
    operation: Operation,
    #[serde(deserialize_with = "deserialize_nonempty")]
    tenant_id: String,
    #[serde(deserialize_with = "deserialize_nonempty")]
    episode_id: String,
    #[serde(deserialize_with = "deserialize_nonempty")]
    render_job_id: String,
}

#[derive(Clone, Copy, Debug, Deserialize)]
enum Operation {
    #[serde(rename = "startRender")]
    StartRender,
}

#[derive(Debug, Serialize)]
struct AcceptedResponse {
    accepted: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CutListShape {
    version: String,
    input_media: Vec<serde_json::Value>,
    output_track: Vec<serde_json::Value>,
    #[serde(default, rename = "audioMixing")]
    _audio_mixing: Option<serde_json::Value>,
    #[serde(default, rename = "overlayTracks")]
    _overlay_tracks: Option<serde_json::Value>,
}

impl CutListShape {
    fn is_renderable(&self) -> bool {
        self.version == "1.0.0"
            && !self.input_media.is_empty()
            && !self.output_track.is_empty()
    }
}

#[derive(Debug, PartialEq, Eq)]
enum StartDecision {
    AlreadyRunning,
    Submit,
    FailPending,
    RejectJob,
}

fn start_decision(
    episode_status: &str,
    has_cut_list: bool,
    render_status: &str,
    existing_job_name: Option<&str>,
    expected_job_name: &str,
) -> StartDecision {
    if episode_status == "rendering"
        && has_cut_list
        && render_status == "running"
        && existing_job_name == Some(expected_job_name)
    {
        StartDecision::AlreadyRunning
    } else if render_status != "pending" {
        StartDecision::RejectJob
    } else if episode_status != "approved" || !has_cut_list {
        StartDecision::FailPending
    } else {
        StartDecision::Submit
    }
}

#[derive(Clone)]
struct AppContext {
    batch: BatchClient,
    config: Config,
    database: Arc<tokio::sync::Mutex<PgClient>>,
}

#[derive(Debug, Error)]
enum StartRenderError {
    #[error("tenant does not exist")]
    TenantNotFound,
    #[error("episode does not exist or is not approved with a cut list")]
    EpisodeNotReady,
    #[error("render job does not exist or is not pending")]
    RenderJobNotPending,
    #[error("database operation failed: {0}")]
    Database(#[from] tokio_postgres::Error),
    #[error("AWS Batch submission failed: {0}")]
    Submit(String),
    #[error("AWS Batch returned no job ID")]
    MissingBatchJobId,
    #[error(
        "database operation failed after Batch submission: {database}; Batch cleanup failed: {cleanup}"
    )]
    DatabaseAndCleanup {
        database: tokio_postgres::Error,
        cleanup: String,
    },
}

fn deserialize_nonempty<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value = String::deserialize(deserializer)?;
    if value.trim().is_empty() {
        return Err(serde::de::Error::custom("ID must not be empty"));
    }
    Ok(value)
}

fn render_job_name(render_job_id: &str) -> String {
    const PREFIX: &str = "render-";
    const MAX_JOB_NAME_LEN: usize = 128;

    let safe_id: String = render_job_id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric()
                || character == '-'
                || character == '_'
            {
                character
            } else {
                '-'
            }
        })
        .take(MAX_JOB_NAME_LEN - PREFIX.len())
        .collect();
    format!("{PREFIX}{safe_id}")
}

async fn fail_pending_render(
    transaction: Transaction<'_>,
    request: &StartRenderRequest,
    error_message: &str,
) -> Result<(), StartRenderError> {
    let updated = transaction
        .execute(
            "UPDATE render_jobs SET status = 'failed', progress = 0, \
             error_message = $2, updated_at = NOW() \
             WHERE id = $1 AND tenant_id = $3 AND episode_id = $4 \
             AND status = 'pending'",
            &[
                &request.render_job_id,
                &error_message,
                &request.tenant_id,
                &request.episode_id,
            ],
        )
        .await?;
    if updated != 1 {
        return Err(StartRenderError::RenderJobNotPending);
    }
    transaction.commit().await?;
    Ok(())
}

async fn start_render(
    context: &AppContext,
    request: &StartRenderRequest,
) -> Result<AcceptedResponse, StartRenderError> {
    let mut database = context.database.lock().await;
    let transaction = database.transaction().await?;

    let tenant = transaction
        .query_opt(
            "SELECT id FROM tenants WHERE id = $1 FOR UPDATE",
            &[&request.tenant_id],
        )
        .await?;
    if tenant.is_none() {
        return Err(StartRenderError::TenantNotFound);
    }

    let episode = transaction
        .query_opt(
            "SELECT status::text AS status, cut_list \
             FROM episodes WHERE id = $1 AND tenant_id = $2 FOR UPDATE",
            &[&request.episode_id, &request.tenant_id],
        )
        .await?;
    let Some(episode) = episode else {
        return Err(StartRenderError::EpisodeNotReady);
    };

    let render_job = transaction
        .query_opt(
            "SELECT status::text AS status, glowing_telegram_id FROM render_jobs \
             WHERE id = $1 AND tenant_id = $2 \
             AND episode_id = $3 FOR UPDATE",
            &[
                &request.render_job_id,
                &request.tenant_id,
                &request.episode_id,
            ],
        )
        .await?;
    let Some(render_job) = render_job else {
        return Err(StartRenderError::RenderJobNotPending);
    };

    let job_name = render_job_name(&request.render_job_id);
    let episode_status: String = episode.get("status");
    let cut_list: Option<serde_json::Value> = episode.get("cut_list");
    let has_cut_list = cut_list
        .and_then(|value| serde_json::from_value::<CutListShape>(value).ok())
        .is_some_and(|cut_list| cut_list.is_renderable());
    let render_status: String = render_job.get("status");
    let existing_job_name: Option<String> =
        render_job.get("glowing_telegram_id");

    match start_decision(
        &episode_status,
        has_cut_list,
        &render_status,
        existing_job_name.as_deref(),
        &job_name,
    ) {
        StartDecision::AlreadyRunning => {
            transaction.commit().await?;
            return Ok(AcceptedResponse { accepted: true });
        }
        StartDecision::FailPending => {
            let error = StartRenderError::EpisodeNotReady;
            fail_pending_render(transaction, request, &error.to_string())
                .await?;
            return Err(error);
        }
        StartDecision::RejectJob => {
            return Err(StartRenderError::RenderJobNotPending);
        }
        StartDecision::Submit => {}
    }

    let submission = match context
        .batch
        .submit_job()
        .job_name(&job_name)
        .job_queue(&context.config.render_job_queue)
        .job_definition(&context.config.render_job_definition)
        .parameters("render_job_id", &request.render_job_id)
        .send()
        .await
    {
        Ok(submission) => submission,
        Err(submit_error) => {
            let error = StartRenderError::Submit(submit_error.to_string());
            fail_pending_render(transaction, request, &error.to_string())
                .await?;
            return Err(error);
        }
    };
    let Some(batch_job_id) = submission.job_id().map(str::to_owned) else {
        let error = StartRenderError::MissingBatchJobId;
        fail_pending_render(transaction, request, &error.to_string()).await?;
        return Err(error);
    };

    let database_result = async {
        transaction
            .execute(
                "UPDATE render_jobs SET status = 'running', \
                 glowing_telegram_id = $2, progress = 0, error_message = NULL, \
                 updated_at = NOW() WHERE id = $1",
                &[&request.render_job_id, &job_name],
            )
            .await?;
        transaction
            .execute(
                "UPDATE episodes SET status = 'rendering', updated_at = NOW() \
                 WHERE id = $1",
                &[&request.episode_id],
            )
            .await?;
        transaction.commit().await
    }
    .await;

    if let Err(database_error) = database_result {
        let cleanup = context
            .batch
            .terminate_job()
            .job_id(batch_job_id)
            .reason("pipeline database transaction failed")
            .send()
            .await;
        return match cleanup {
            Ok(_) => Err(StartRenderError::Database(database_error)),
            Err(cleanup_error) => Err(StartRenderError::DatabaseAndCleanup {
                database: database_error,
                cleanup: cleanup_error.to_string(),
            }),
        };
    }

    Ok(AcceptedResponse { accepted: true })
}

async fn handler(
    context: Arc<AppContext>,
    event: LambdaEvent<StartRenderRequest>,
) -> Result<AcceptedResponse, Error> {
    let (request, _) = event.into_parts();
    let _ = request.operation;
    start_render(&context, &request)
        .await
        .map_err(|error| Error::from(error.to_string()))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .without_time()
        .init();

    let config: Config = Figment::new().merge(Env::raw()).extract()?;
    let aws_config =
        aws_config::defaults(BehaviorVersion::latest()).load().await;
    let database = gt_postgres::connect(&config.postgres, &aws_config).await?;
    let context = Arc::new(AppContext {
        batch: BatchClient::new(&aws_config),
        config,
        database: Arc::new(tokio::sync::Mutex::new(database)),
    });

    run(service_fn(move |event| handler(context.clone(), event))).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_request(
        value: &str,
    ) -> Result<StartRenderRequest, serde_json::Error> {
        serde_json::from_str(value)
    }

    #[test]
    fn accepts_only_the_canonical_request() {
        let request = parse_request(
            r#"{"operation":"startRender","tenantId":"tenant","episodeId":"episode","renderJobId":"job"}"#,
        )
        .unwrap();

        assert_eq!(request.tenant_id, "tenant");
        assert_eq!(request.episode_id, "episode");
        assert_eq!(request.render_job_id, "job");
    }

    #[test]
    fn rejects_empty_or_whitespace_ids() {
        for field in ["tenantId", "episodeId", "renderJobId"] {
            let mut value = serde_json::json!({
                "operation": "startRender",
                "tenantId": "tenant",
                "episodeId": "episode",
                "renderJobId": "job"
            });
            value[field] = serde_json::Value::String(" ".to_owned());
            assert!(
                serde_json::from_value::<StartRenderRequest>(value).is_err(),
                "accepted empty {field}"
            );
        }
    }

    #[test]
    fn rejects_other_operations_and_fields() {
        assert!(
            parse_request(
                r#"{"operation":"retryRender","tenantId":"tenant","episodeId":"episode","renderJobId":"job"}"#,
            )
            .is_err()
        );
        assert!(
            parse_request(
                r#"{"operation":"startRender","tenantId":"tenant","episodeId":"episode","renderJobId":"job","extra":true}"#,
            )
            .is_err()
        );
    }

    #[test]
    fn accepts_only_canonical_renderable_cut_lists() {
        let canonical: CutListShape =
            serde_json::from_value(serde_json::json!({
                "version": "1.0.0",
                "inputMedia": [{"s3Location": "source.mkv", "sections": []}],
                "outputTrack": [{"mediaIndex": 0, "sectionIndex": 0}]
            }))
            .unwrap();
        assert!(canonical.is_renderable());

        let simplified =
            serde_json::from_value::<CutListShape>(serde_json::json!({
                "clipIds": ["clip"],
                "trims": {}
            }));
        assert!(simplified.is_err());
    }

    #[test]
    fn competing_pending_job_is_failed_after_episode_starts_rendering() {
        assert_eq!(
            start_decision("rendering", true, "pending", None, "render-job"),
            StartDecision::FailPending
        );
        assert_eq!(
            start_decision(
                "rendering",
                true,
                "running",
                Some("render-job"),
                "render-job"
            ),
            StartDecision::AlreadyRunning
        );
    }

    #[test]
    fn job_name_is_deterministic_safe_and_contains_uuid() {
        let id = "550e8400-e29b-41d4-a716-446655440000";
        let first = render_job_name(id);

        assert_eq!(first, "render-550e8400-e29b-41d4-a716-446655440000");
        assert_eq!(first, render_job_name(id));
        assert!(first.len() <= 128);
        assert!(first.chars().all(|character| {
            character.is_ascii_alphanumeric()
                || character == '-'
                || character == '_'
        }));
    }

    #[test]
    fn job_name_sanitizes_and_limits_non_uuid_ids() {
        let name =
            render_job_name(&format!("id/with spaces/{}", "x".repeat(200)));

        assert!(name.starts_with("render-id-with-spaces-"));
        assert_eq!(name.len(), 128);
    }
}
