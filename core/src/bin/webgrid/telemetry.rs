use anyhow::Result;
use opentelemetry::sdk::propagation::TraceContextPropagator;
use opentelemetry::sdk::trace::{self, IdGenerator, Sampler};
use opentelemetry::sdk::Resource;
use opentelemetry::trace::TraceError;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::Protocol;
use opentelemetry_semantic_conventions as semcov;
use std::time::Duration;
use tracing::Collect;
use tracing_subscriber::{prelude::*, Registry};

pub fn try_init(endpoint: &str) -> Result<()> {
    with_endpoint(endpoint)?.try_init()?;
    Ok(())
}

pub fn with_endpoint(endpoint: &str) -> Result<impl Collect + Send + Sync + 'static, TraceError> {
    global::set_text_map_propagator(TraceContextPropagator::new());

    let resource: Vec<KeyValue> = vec![
        semcov::resource::SERVICE_NAME.string(std::env::var("ID").unwrap()),
        // semcov::resource::SERVICE_NAMESPACE.string(constants::service::NAMESPACE),
        semcov::resource::SERVICE_VERSION.string(env!("WEBGRID_VERSION")),
    ];

    // if let Some(instance_id) = instance_id {
    //     resource.push(semcov::resource::SERVICE_INSTANCE_ID.string(instance_id.into()));
    // }

    let tracer: opentelemetry::sdk::trace::Tracer = opentelemetry_otlp::new_pipeline()
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

    let filter = tracing_subscriber::EnvFilter::from_default_env();

    let telemetry = tracing_opentelemetry::subscriber().with_tracer(tracer);
    let subscriber = Registry::default().with(filter).with(telemetry);

    Ok(subscriber)
}

pub fn flush() {
    global::shutdown_tracer_provider();
}
