use anyhow::Result;
use async_trait::async_trait;

use super::task_manager::TaskManager;

/// Persistent execution unit
///
/// Jobs can be dependent on resource from the `resource` library.
/// If such a resource dependency becomes unavailable the job is terminated and restarted.
///
/// In addition, jobs can support graceful shutdown and a ready state provided by the TaskManager passed to the execute function.
#[async_trait]
pub trait Job {
    type Context;

    /// Name of the job displayed in log messages
    const NAME: &'static str;
    /// Whether or not the job honors the termination signal. When this is set to false the job will be terminated externally.
    const SUPPORTS_GRACEFUL_TERMINATION: bool = false;

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn supports_graceful_termination(&self) -> bool {
        Self::SUPPORTS_GRACEFUL_TERMINATION
    }

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()>;
}
