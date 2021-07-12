use super::super::super::BoxedError;
use crate::library::EmptyResult;
use async_trait::async_trait;
use serde::Deserialize;

/// Describes a notification queue and its parameters
#[derive(Debug, PartialEq, Eq)]
pub struct QueueDescriptor {
    key: String,
    limit: usize,
}

impl QueueDescriptor {
    /// Creates a new instance from raw parts
    pub fn new(key: String, limit: usize) -> Self {
        Self { key, limit }
    }

    /// Value which may be used by queue implementations to identify a queue
    pub fn key(&self) -> &str {
        &self.key
    }

    /// Retrieves the key (ref [`key()`](QueueDescriptor::key) method) with an extension added
    pub fn key_with_extension(
        &self,
        extension: &str, /* QueueDescriptorExtension */
    ) -> String {
        format!("{}.{}", self.key, extension)
    }

    /// Maximum number of notifications to be retained in the queue
    pub fn limit(&self) -> usize {
        self.limit
    }
}

/// Adds additional information to the key of a queue, specialising it further
///
/// **A word of caution:** Use this sparingly and with well-known, commonly and frequently
/// used values _ONLY_! This is intended for those rare situations where the key of a Queue can
/// not be determined at compile-time and depends on dynamic factors. Beware that using ephemeral
/// keys leads to resource creep, memory leaks by unused queues and usually hints at the employment
/// of an anti-pattern in the architecture!
///
/// **_DO NOT TAKE THE ABOVE LIGHTLY, THINK TWICE BEFORE USING THIS!_**
pub type QueueDescriptorExtension = String;

/// Location within the queue
#[derive(Clone)]
pub enum QueueLocation {
    /// Start of the queue (not necessarily the first notification as a queue is limited in length)
    Head,
    /// End of the queue (exclusive of the last message)
    Tail,
}

/// Entry retrieved from a [`Queue`](QueueDescriptor) providing a raw payload
#[async_trait]
pub trait RawQueueEntry {
    /// Payload of the item
    fn payload(&self) -> &[u8];

    /// Acknowledge the item as processed
    async fn acknowledge(&mut self) -> EmptyResult;
}

/// Useful functions for [`QueueEntry`] implementations with default implementations
pub trait QueueEntry: RawQueueEntry {
    /// Attempts to parse the wire-format payload into a given data structure
    fn parse_payload<'a, T>(&'a self) -> Result<T, BoxedError>
    where
        T: Deserialize<'a>;
}
