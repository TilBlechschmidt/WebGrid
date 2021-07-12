//! Trait implementations using [`redis`](::redis)

const RESPONSE_KEY_PREFIX: &str = "response.";
const STREAM_PAYLOAD_KEY: &str = "payload";
const STREAM_ID_NEW: &str = "*";
const STREAM_ID_HEAD: &str = "0";
const STREAM_ID_TAIL: &str = "$";
const STREAM_ID_ADDITIONS: &str = ">";

use thiserror::Error;

mod collector;
mod discovery;
mod factory;
mod publisher;
mod queue_entry;
mod queue_provider;

pub use collector::*;
pub use discovery::*;
pub use factory::*;
pub use publisher::*;
pub use queue_entry::*;
pub use queue_provider::*;

#[derive(Debug, Error)]
enum RedisQueueError {
    #[error("payload field missing from queue entry")]
    MissingPayload,
}
