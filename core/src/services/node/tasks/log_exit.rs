use std::pin::Pin;

use super::super::{structs::NodeError, Context};
use crate::libraries::lifecycle::logging::{LogCode, SessionLogger};
use crate::libraries::lifecycle::DeathReason;
use crate::libraries::resources::{ResourceManager, ResourceManagerProvider};
use crate::libraries::scheduling::TaskManager;
use crate::with_shared_redis_resource;
use futures::Future;

pub fn log_exit(
    death_reason: DeathReason,
) -> impl Fn(TaskManager<Context>) -> Pin<Box<dyn Future<Output = Result<(), NodeError>> + Send>> {
    move |manager: TaskManager<Context>| {
        let death_reason = death_reason.clone();

        Box::pin(async move {
            let con = with_shared_redis_resource!(manager);
            let mut logger =
                SessionLogger::new(con, "node".to_string(), manager.context.id.to_owned());

            let log_code = match death_reason {
                DeathReason::LifetimeExceeded => LogCode::STIMEOUT,
                DeathReason::Terminated => LogCode::CLOSED,
                DeathReason::Killed(_) => LogCode::CLOSED,
            };

            logger.log(log_code, None).await.ok();

            Ok(())
        })
    }
}
