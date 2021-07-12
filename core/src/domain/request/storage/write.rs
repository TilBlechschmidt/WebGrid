use crate::library::communication::event::{Notification, QueueDescriptor};
use crate::library::communication::request::{Request, ResponseLocation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const QUEUE_KEY: &str = "storage.write";
const QUEUE_SIZE: usize = 10_000;

/// Request to write an object to persistent storage
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageWriteRequest {
    /// Unique identifier of the session the object is associated with
    pub session_id: Uuid,

    /// Object location within the session storage namespace
    pub path: String,

    /// MIME type of the object to be written
    pub mime: String,

    response_location: ResponseLocation,
}

/// Response to a [`StorageWriteRequest`]
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageWriteResponse {
    /// URL to which the object can be written using an HTTP PUT request
    pub location: String,
}

impl Notification for StorageWriteRequest {
    fn queue() -> QueueDescriptor {
        QueueDescriptor::new(QUEUE_KEY.into(), QUEUE_SIZE)
    }
}

impl Request for StorageWriteRequest {
    // TODO It might be possible that we have to return an error (e.g. permission denied)
    type Response = StorageWriteResponse;

    fn reply_to(&self) -> ResponseLocation {
        // TODO This clone feels unnecessary ...
        self.response_location.clone()
    }
}
