mod factory;
mod notification_publisher;
mod requestor;

// Unimplemented modules
mod discovery;
mod queue_provider;
mod response_collector;
mod response_publisher;

use queue_provider::MockQueueProvider;

pub use factory::*;
pub use notification_publisher::*;
pub use requestor::*;

#[derive(Clone, PartialEq, Eq)]
pub enum ExpectationMode {
    /// No validity checks of any sort, just a dummy
    Ignore,
    /// Only allows expected items and requires all of them
    ExpectOnlyProvided,
    /// Allows intermittent noise but still requires all expected
    /// items to eventually be published
    AllowNoise,
}
