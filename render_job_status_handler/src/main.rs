use std::sync::Arc;

use aws_config::BehaviorVersion;
use figment::{Figment, providers::Env};
use gt_postgres::PostgresConfig;
use lambda_runtime::{Error, LambdaEvent, run, service_fn};
use serde::Deserialize;
use tokio_postgres::Client;

#[derive(Debug, Deserialize)]
struct BatchEvent {
    detail: BatchDetail,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BatchDetail {
    status: String,
    #[serde(default)]
    status_reason: Option<String>,
    parameters: BatchParameters,
    #[serde(default)]
    container: Option<BatchContainer>,
}

#[derive(Debug, Deserialize)]
struct BatchParameters {
    render_job_id: String,
}

#[derive(Debug, Deserialize)]
struct BatchContainer {
    #[serde(default)]
    reason: Option<String>,
}

impl BatchDetail {
    fn failure_message(&self) -> String {
        self.status_reason
            .as_deref()
            .or_else(|| self.container.as_ref()?.reason.as_deref())
            .unwrap_or("AWS Batch render job failed")
            .chars()
            .take(4_000)
            .collect()
    }
}

async fn mark_failed(
    client: &mut Client,
    detail: &BatchDetail,
) -> Result<(), Error> {
    if detail.status != "FAILED" {
        return Ok(());
    }

    let transaction = client.transaction().await?;
    let job = transaction
        .query_opt(
            "SELECT episode_id, tenant_id FROM render_jobs \
             WHERE id = $1 AND status = 'running' FOR UPDATE",
            &[&detail.parameters.render_job_id],
        )
        .await?;
    let Some(job) = job else {
        transaction.commit().await?;
        return Ok(());
    };
    let episode_id: String = job.get("episode_id");
    let tenant_id: String = job.get("tenant_id");
    let message = detail.failure_message();

    let episodes = transaction
        .execute(
            "UPDATE render_jobs SET status = 'failed', error_message = $1, \
             updated_at = NOW() WHERE id = $2 AND episode_id = $3 \
             AND tenant_id = $4 AND status = 'running'",
            &[
                &message,
                &detail.parameters.render_job_id,
                &episode_id,
                &tenant_id,
            ],
        )
        .await?;
    transaction
        .execute(
            "UPDATE episodes SET status = 'approved', updated_at = NOW() \
             WHERE id = $1 AND tenant_id = $2 AND status = 'rendering'",
            &[&episode_id, &tenant_id],
        )
        .await?;
    if episodes != 1 {
        return Err("render job episode is no longer rendering".into());
    }
    transaction.commit().await?;
    Ok(())
}

async fn handler(
    database: Arc<tokio::sync::Mutex<Client>>,
    event: LambdaEvent<BatchEvent>,
) -> Result<(), Error> {
    let mut database = database.lock().await;
    mark_failed(&mut database, &event.payload.detail).await
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .without_time()
        .init();
    let config: PostgresConfig = Figment::new().merge(Env::raw()).extract()?;
    let aws_config =
        aws_config::defaults(BehaviorVersion::latest()).load().await;
    let database = Arc::new(tokio::sync::Mutex::new(
        gt_postgres::connect(&config, &aws_config).await?,
    ));

    run(service_fn(move |event| handler(database.clone(), event))).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_batch_failure_and_prefers_status_reason() {
        let event: BatchEvent = serde_json::from_value(serde_json::json!({
            "detail": {
                "status": "FAILED",
                "statusReason": "Task failed to start",
                "parameters": { "render_job_id": "render-job" },
                "container": { "reason": "Container exited" }
            }
        }))
        .unwrap();

        assert_eq!(event.detail.parameters.render_job_id, "render-job");
        assert_eq!(event.detail.failure_message(), "Task failed to start");
    }

    #[test]
    fn falls_back_to_container_reason() {
        let detail: BatchDetail = serde_json::from_value(serde_json::json!({
            "status": "FAILED",
            "parameters": { "render_job_id": "render-job" },
            "container": { "reason": "Container exited" }
        }))
        .unwrap();

        assert_eq!(detail.failure_message(), "Container exited");
    }
}
