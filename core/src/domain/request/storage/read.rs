use crate::library::communication::event::{Notification, QueueDescriptor};
use crate::library::communication::request::{Request, ResponseLocation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const QUEUE_KEY: &str = "storage.read";
const QUEUE_SIZE: usize = 100;

/// Request to read an object from persistent storage
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageReadRequest {
    /// Unique identifier of the session the object is associated with
    pub session_id: Uuid,

    /// Object location within the session storage namespace
    pub path: String,

    response_location: ResponseLocation,
}

/// Response to a [`StorageReadRequest`]
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageReadResponse {
    /// URL from which the object can be fetched using an HTTP GET request
    pub location: String,
}

impl Notification for StorageReadRequest {
    fn queue() -> QueueDescriptor {
        QueueDescriptor::new(QUEUE_KEY.into(), QUEUE_SIZE)
    }
}

impl Request for StorageReadRequest {
    // TODO It might be possible that we have to return an error (e.g. object not found)
    type Response = StorageReadResponse;

    fn reply_to(&self) -> ResponseLocation {
        // TODO This clone feels unnecessary ...
        self.response_location.clone()
    }
}
