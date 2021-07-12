//! Domain specific structures, implementations, and logic

/// Default queue size for all session startup related events
///
/// It should hold a number of items that equals the maximum reasonable number of sessions
/// a user might burst-create without them being processed.
pub(self) const QUEUE_SIZE_STARTUP_WORKFLOW: usize = 5_000;

mod discovery;

pub mod container;
pub mod event;
pub mod request;
pub mod webdriver;

pub use discovery::*;
