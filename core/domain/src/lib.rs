//! Domain specific structures, implementations, and logic

#![deny(missing_docs)]
// Disable the lint for now as it has a high false-positive rate
#![allow(unknown_lints, clippy::nonstandard_macro_braces)]

/// Default queue size for all session startup related events
///
/// It should hold a number of items that equals the maximum reasonable number of sessions
/// a user might burst-create without them being processed.
pub(self) const QUEUE_SIZE_STARTUP_WORKFLOW: usize = 5_000;

mod discovery;
mod session;

pub mod container;
pub mod event;
pub mod request;
pub mod webdriver;

pub use discovery::*;
pub use session::SessionMetadata;

/// Creates a storage path within a sessions namespace
pub fn storage_path(session_id: event::SessionIdentifier, filename: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(session_id.to_string()).join(filename.trim_start_matches('/'))
}
