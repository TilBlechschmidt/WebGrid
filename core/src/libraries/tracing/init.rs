use std::time::Duration;

use opentelemetry::{
    global,
    sdk::{
        propagation::TraceContextPropagator,
        trace::{self, IdGenerator, Sampler},
        Resource,
    },
    trace::TraceError,
    KeyValue,
};
use opentelemetry_otlp::Protocol;
use opentelemetry_semantic_conventions as semcov;

use crate::libraries::tracing::constants;

pub fn init<T>(
    endpoint: &Option<String>,
    service: T,
    instance_id: Option<T>,
) -> Result<(), TraceError>
where
    T: Into<String>,
{
    if let Some(endpoint) = endpoint {
        global::set_text_map_propagator(TraceContextPropagator::new());

        let mut resource: Vec<KeyValue> = vec![
            semcov::resource::SERVICE_NAME.string(service.into()),
            semcov::resource::SERVICE_NAMESPACE.string(constants::service::NAMESPACE),
            semcov::resource::SERVICE_VERSION.string(env!("WEBGRID_VERSION")),
        ];

        if let Some(instance_id) = instance_id {
            resource.push(semcov::resource::SERVICE_INSTANCE_ID.string(instance_id.into()));
        }

        opentelemetry_otlp::new_pipeline()
            .with_endpoint(endpoint)
            .with_protocol(Protocol::Grpc)
            .with_timeout(Duration::from_secs(3))
            .with_trace_config(
                trace::config()
                    .with_sampler(Sampler::AlwaysOn)
                    .with_id_generator(IdGenerator::default())
                    .with_max_events_per_span(64)
                    .with_max_attributes_per_span(16)
                    .with_max_events_per_span(16)
                    .with_resource(Resource::new(resource)),
            )
            .with_tonic()
            .install_batch(opentelemetry::runtime::Tokio)?;
    }

    Ok(())
}
