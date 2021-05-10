//! Tracing module
//!
//! This module contains all the tools necessary to enable tracing in conformance with OpenTelemetry.

pub mod constants;
mod init;
mod propagation;

pub use init::init;
pub use propagation::StringPropagator;

use opentelemetry::global::{self, BoxedTracer};

pub fn global_tracer() -> BoxedTracer {
    global::tracer("webgrid.dev/main")
}
