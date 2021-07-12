use super::QueueLocation;

/// Unique identifier for a group of consumers
#[derive(Clone)]
pub enum ConsumerGroupIdentifier {
    /// Generic worker group in a work queue
    Worker,
    /// Gangway instance
    Gangway,
    /// Unknown consumer group
    Other(String),
}

impl ToString for ConsumerGroupIdentifier {
    fn to_string(&self) -> String {
        match self {
            Self::Worker => "worker".into(),
            Self::Gangway => "gangway".into(),
            Self::Other(identifier) => identifier.to_owned(),
        }
    }
}

/// Definition of a consumer group
///
/// In a message queue, a group of consumers collaborates to consume messages.
/// Each message is only delivered to one consumer within the same group, identified
/// by a [`ConsumerGroupIdentifier`]. When it is created, they start processing messages
/// from the provided [`QueueLocation`].
#[derive(Clone)]
pub struct ConsumerGroupDescriptor {
    identifier: ConsumerGroupIdentifier,
    start: QueueLocation,
}

impl ConsumerGroupDescriptor {
    /// Creates a new instance from raw parts
    pub fn new(identifier: ConsumerGroupIdentifier, start: QueueLocation) -> Self {
        Self { identifier, start }
    }

    /// Unique identifier of the group
    pub fn identifier(&self) -> &ConsumerGroupIdentifier {
        &self.identifier
    }

    /// Location from where a consumer group begins to consume messages
    ///
    /// Note that it is not guaranteed that this will be honored (e.g. when the group already exists)!
    pub fn start(&self) -> &QueueLocation {
        &self.start
    }
}

impl Default for ConsumerGroupDescriptor {
    /// Uses [`ConsumerGroupIdentifier::Worker`] and [`QueueLocation::Head`] as they are most commonly employed
    fn default() -> Self {
        Self {
            identifier: ConsumerGroupIdentifier::Worker,
            start: QueueLocation::Head,
        }
    }
}

/// Unique identifier of a consumer within a [`ConsumerGroup`](ConsumerGroupDescriptor)
pub type ConsumerIdentifier = String;
