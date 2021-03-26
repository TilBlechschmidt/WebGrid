//! Individual micro-services for the grid

#[cfg(feature = "gc")]
pub mod gc;

#[cfg(feature = "api")]
pub mod api;

#[cfg(feature = "proxy")]
pub mod proxy;

#[cfg(feature = "manager")]
pub mod manager;

#[cfg(feature = "node")]
pub mod node;

#[cfg(feature = "metrics")]
pub mod metrics;

#[cfg(feature = "orchestrator")]
pub mod orchestrator;

#[cfg(feature = "storage")]
pub mod storage;

mod options;
pub use options::SharedOptions;
