use super::super::{structs::NodeError, Context};
use crate::libraries::{
    helpers::keys,
    resources::{ResourceManager, ResourceManagerProvider},
    tracing::StringPropagator,
};
use crate::with_redis_resource;
use jatsl::TaskManager;
use opentelemetry::{trace::TraceContextExt, Context as TelemetryContext};
use redis::AsyncCommands;

pub async fn initialize_tracing(
    manager: TaskManager<Context>,
) -> Result<TelemetryContext, NodeError> {
    let mut con = with_redis_resource!(manager);

    let raw_telemetry_context: String = con
        .hget(
            keys::session::telemetry::creation(&manager.context.id.to_string()),
            "context",
        )
        .await
        .unwrap_or_default();

    let telemetry_context = TelemetryContext::current_with_span(StringPropagator::deserialize(
        &raw_telemetry_context,
        "Node startup",
    ));

    Ok(telemetry_context)
}
