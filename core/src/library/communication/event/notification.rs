use super::QueueDescriptor;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::fmt::Debug;

/// Entity to notify other service about an event that took place
pub trait Notification: Serialize + DeserializeOwned + PartialEq + Debug {
    /// Queue on which this implementation can be sent and received
    fn queue() -> QueueDescriptor;
}
