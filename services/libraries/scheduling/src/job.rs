use anyhow::Result;
use async_trait::async_trait;

use super::task_manager::TaskManager;

#[async_trait]
pub trait Job {
    type Context;
    const NAME: &'static str;
    /// Whether or not the job honors the termination signal. When this is set to false the job will be terminated externally. Default: false
    const SUPPORTS_GRACEFUL_TERMINATION: bool = false;

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn supports_graceful_termination(&self) -> bool {
        Self::SUPPORTS_GRACEFUL_TERMINATION
    }

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()>;
}
