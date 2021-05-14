use super::super::Context;
use crate::libraries::resources::ResourceManagerProvider;
use crate::libraries::{
    helpers::{keys, lua},
    resources::ResourceManager,
};
use crate::with_shared_redis_resource;
use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use jatsl::{Job, TaskManager};
use log::{debug, info, warn};
use redis::{AsyncCommands, Script};
use std::time::Duration;
use tokio::{task::yield_now, time::interval};

pub struct GarbageCollectorJob {
    session_retention_duration: chrono::Duration,
}

#[async_trait]
impl Job for GarbageCollectorJob {
    type Context = Context;

    const NAME: &'static str = module_path!();

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        manager.ready().await;
        yield_now().await;

        self.log_retention_duration();

        let mut interval = interval(Duration::from_secs(600));

        loop {
            interval.tick().await;
            debug!("Running garbage collector cycle");

            let terminated_count = self.terminate_dead_sessions(manager.clone()).await?;
            let purged_count = self.purge_old_sessions(manager.clone()).await?;

            info!(
                "Terminated {} and purged {} sessions",
                terminated_count, purged_count
            );

            let purged_orchestrator_count = self.purge_old_orchestrators(manager.clone()).await?;
            info!("Purged {} orchestrators", purged_orchestrator_count);
        }
    }
}

impl GarbageCollectorJob {
    pub fn new(session_retention_duration: i64) -> Self {
        Self {
            session_retention_duration: chrono::Duration::seconds(session_retention_duration),
        }
    }

    fn log_retention_duration(&self) {
        let days = self.session_retention_duration.num_days();
        let hours = self.session_retention_duration.num_hours();
        let minutes = self.session_retention_duration.num_minutes();

        if days > 0 {
            info!("Retaining session metadata for {} days", days);
        } else if hours > 0 {
            info!("Retaining session metadata for {} hours", hours);
        } else if minutes > 0 {
            info!("Retaining session metadata for {} minutes", minutes);
        } else {
            info!(
                "Retaining session metadata for {} seconds",
                self.session_retention_duration.num_seconds()
            );
        }
    }

    async fn terminate_dead_sessions(&self, manager: TaskManager<Context>) -> Result<usize> {
        // Fetch active sessions
        let mut con = with_shared_redis_resource!(manager);
        let active_sessions: Vec<String> = con.smembers(&*keys::session::LIST_ACTIVE).await?;

        // Preload the session termination script
        let script_content = format!(
            r#"
            local sessionID = ARGV[1];
            local currentTime = ARGV[2];
            {loadOrchestratorID}
            {terminateSession}
        "#,
            loadOrchestratorID = lua::fetch_orchestrator_from_session(),
            terminateSession = lua::terminate_session(),
        );
        let termination_script = Script::new(&script_content);

        let mut terminated_count = 0;
        for session_id in active_sessions.into_iter() {
            let has_manager_heartbeat: bool = con
                .exists(keys::session::heartbeat::manager(&session_id))
                .await?;

            let has_node_heartbeat: bool = con
                .exists(keys::session::heartbeat::node(&session_id))
                .await?;

            // If the session has no heartbeat / responsible operator, it is dead.
            if !(has_manager_heartbeat || has_node_heartbeat) {
                debug!("Terminating dead session: {}", session_id);
                terminated_count += 1;

                if let Err(e) = termination_script
                    .arg(&session_id)
                    .arg(Utc::now().to_rfc3339())
                    .invoke_async::<_, ()>(&mut con)
                    .await
                {
                    warn!("Failed to terminate dead session {}: {}", session_id, e);
                }
            }
        }

        Ok(terminated_count)
    }

    async fn purge_old_sessions(&self, manager: TaskManager<Context>) -> Result<usize> {
        // Fetch terminated sessions
        let mut con = with_shared_redis_resource!(manager);
        let terminated_sessions: Vec<String> =
            con.smembers(&*keys::session::LIST_TERMINATED).await?;

        // Preload the deletion script
        let deletion_script = Script::new(&lua::delete_session());

        // Go through all terminated sessions
        let mut purged_count = 0;
        for session_id in terminated_sessions.into_iter() {
            // Determine its age (since termination)
            let termination_time: String = con
                .hget(keys::session::status(&session_id), "terminatedAt")
                .await?;

            let parsed_termination_time = chrono::DateTime::parse_from_rfc3339(&termination_time)?;
            let age = Utc::now().signed_duration_since(parsed_termination_time);

            // If we crossed the threshold, purge it!
            if age > self.session_retention_duration {
                debug!("Purging old session: {}", session_id);
                purged_count += 1;

                if let Err(e) = deletion_script
                    .arg(&session_id)
                    .invoke_async::<_, ()>(&mut con)
                    .await
                {
                    warn!("Failed to purge old session {}: {}", session_id, e);
                }
            }
        }

        Ok(purged_count)
    }

    async fn purge_old_orchestrators(&self, manager: TaskManager<Context>) -> Result<usize> {
        // Fetch orchestrators
        let mut con = with_shared_redis_resource!(manager);
        let orchestrators: Vec<String> = con.smembers(&*keys::orchestrator::LIST).await?;

        let deletion_script = Script::new(&lua::delete_orchestrator());

        let mut purged_count = 0;
        for orchestrator_id in orchestrators.into_iter() {
            if let Ok(retain) = con
                .exists::<_, bool>(keys::orchestrator::retain(&orchestrator_id))
                .await
            {
                // If the retain key does not exist, purge the associated orchestrator
                if !retain {
                    purged_count += 1;

                    if let Err(e) = deletion_script
                        .arg(&orchestrator_id)
                        .invoke_async::<_, ()>(&mut con)
                        .await
                    {
                        warn!(
                            "Failed to purge old orchestrator {}: {}",
                            orchestrator_id, e
                        );
                    }
                }
            }
        }

        Ok(purged_count)
    }
}
