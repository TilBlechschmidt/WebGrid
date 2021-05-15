use super::super::structs::NodeError;
use crate::libraries::resources::{ResourceManager, ResourceManagerProvider};
use crate::libraries::storage::StorageHandler;
use crate::libraries::{
    lifecycle::{Heart, HeartStone},
    tracing::global_tracer,
};
use crate::with_redis_resource;
use crate::{
    libraries::helpers::{keys, Timeout},
    services::node::context::StartupContext,
};
use jatsl::TaskManager;
use log::error;
use opentelemetry::trace::{Span, Tracer};
use redis::AsyncCommands;
use std::time::Duration;

pub async fn initialize_service(
    manager: TaskManager<StartupContext>,
) -> Result<(Heart, HeartStone), NodeError> {
    let mut con = with_redis_resource!(manager);

    let mut span = global_tracer().start_with_context(
        "Initialize service",
        manager.context.telemetry_context.clone(),
    );

    let (heart, heart_stone) = Heart::with_lifetime(Duration::from_secs(
        Timeout::SessionTermination.get(&mut con).await as u64,
    ));

    if let Some(storage_directory) = &manager.context.options.storage_directory {
        span.add_event("Load storage information".to_string(), vec![]);

        let storage_id = StorageHandler::storage_id(&storage_directory)
            .await
            .map_err(|e| {
                error!("Storage unavailable: {:?}", e);
                NodeError::StorageUnavailable
            })?;
        con.set::<_, _, ()>(
            keys::session::storage(&manager.context.id.to_string()),
            storage_id.to_string(),
        )
        .await
        .map_err(|_| NodeError::StorageUnavailable)?;
    }

    Ok((heart, heart_stone))
}
