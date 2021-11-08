use super::QueueDescriptor;
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

/// Entity to notify other service about an event that took place
pub trait Notification: Serialize + DeserializeOwned + PartialEq + Debug {
    /// Queue on which this implementation can be sent and received
    fn queue() -> QueueDescriptor;
}

/// Frame around a notification containing additional context and information
#[derive(Serialize, Deserialize, Debug)]
pub struct NotificationFrame<T> {
    timestamp: DateTime<Utc>,
    notification: T,
}

impl<T> NotificationFrame<T> {
    /// Creates a new instance with a publication time of `Utc::now()`
    pub fn new(notification: T) -> Self {
        Self {
            timestamp: Utc::now(),
            notification,
        }
    }

    /// Consumes the `NotificationFrame`, returning the wrapped [`Notification`]
    pub fn into_inner(self) -> T {
        self.notification
    }

    /// Returns the instant at which the notification was originally published
    pub fn publication_time(&self) -> &DateTime<Utc> {
        &self.timestamp
    }
}

impl<T> Deref for NotificationFrame<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.notification
    }
}

impl<T> DerefMut for NotificationFrame<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.notification
    }
}
