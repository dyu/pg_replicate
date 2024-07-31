use std::time::Duration;

use config_types::SinkSettings;
use sqlx::PgPool;
use tracing::{debug, info};
use tracing_log::log::error;

use crate::{
    configuration::Settings,
    k8s_client::{create_bq_service_account_key_secret, create_config_map, create_pod},
    queue::{delete_task, dequeue_task},
    startup::get_connection_pool,
};

pub async fn run_worker_until_stopped(configuration: Settings) -> Result<(), anyhow::Error> {
    let connection_pool = get_connection_pool(&configuration.database);
    let poll_duration = Duration::from_secs(configuration.worker.poll_interval_secs);
    worker_loop(connection_pool, poll_duration).await
}

async fn worker_loop(pool: PgPool, poll_duration: Duration) -> Result<(), anyhow::Error> {
    loop {
        match try_execute_task(&pool).await {
            Ok(ExecutionOutcome::EmptyQueue) => {
                debug!("no task in queue");
            }
            Ok(ExecutionOutcome::TaskCompleted) => {
                debug!("successfully executed task");
            }
            Err(e) => {
                error!("error while executing task: {e:#?}");
            }
        }
        tokio::time::sleep(poll_duration).await;
    }
}

#[allow(clippy::large_enum_variant)]
#[derive(serde::Serialize, serde::Deserialize)]
pub enum Request {
    CreateOrUpdate {
        project_ref: String,
        settings: config_types::Settings,
    },
    Delete {
        project_ref: String,
    },
}

pub async fn try_execute_task(pool: &PgPool) -> Result<ExecutionOutcome, anyhow::Error> {
    let task = dequeue_task(pool).await?;
    let Some((transaction, task)) = task else {
        return Ok(ExecutionOutcome::EmptyQueue);
    };

    let request = serde_json::from_value::<Request>(task.data)?;

    match request {
        Request::CreateOrUpdate {
            project_ref,
            settings,
        } => {
            info!(
                "creating or updating k8s objects for project ref: {}",
                project_ref
            );

            let SinkSettings::BigQuery {
                project_id: _,
                dataset_id: _,
                service_account_key,
            } = &settings.sink;
            create_bq_service_account_key_secret(service_account_key).await?;
            let base_config = "";
            let prod_config = serde_json::to_string(&settings)?;
            create_config_map(base_config, &prod_config).await?;
            create_pod().await?;
        }
        Request::Delete { project_ref } => {
            info!("deleting project ref: {}", project_ref);
        }
    }

    delete_task(transaction, task.id).await?;

    Ok(ExecutionOutcome::TaskCompleted)
}

pub enum ExecutionOutcome {
    TaskCompleted,
    EmptyQueue,
}
