use crate::domain::event::{
    SessionCreatedNotification, SessionIdentifier, SessionOperationalNotification,
    SessionStartupFailedNotification,
};
use lru::LruCache;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};

mod failure_listener;
mod operational_listener;

// TODO Those two services below are basically 1:1 copies of each other. Make a macro or smth out of them!
pub use failure_listener::FailureListenerService;
pub use operational_listener::OperationalListenerService;

/// Wrapper struct containing communication tools for the creation of sessions.
///
/// It aids the communication between the proxy job and other jobs that send data over the wire
/// to external services (mostly implementors of the [crate::library::communication] traits). This
/// decouples the two and allows the connection to external services to die while retaining proxy functionality
/// for existing sessions.
#[derive(Clone)]
pub struct SessionCreationCommunicationHandle {
    /// Sender used to transfer SessionCreatedNotifications to the job that publishes them onto the wire.
    pub creation_tx: mpsc::UnboundedSender<SessionCreatedNotification>,
    /// HashMap-like type that contains one-shot senders for each SessionIdentifier where a client is waiting for
    /// startup. However, to prevent memory leaks of orphaned sender instances where the client left, it is implemented
    /// using an LruCache (whereby the Lru functionality is unimportant, instead the eviction of overflow is relevant).
    /// This allows for old and potentially orphaned senders to be evicted once new ones come in.
    pub status_listeners: Arc<Mutex<LruCache<SessionIdentifier, oneshot::Sender<StatusResponse>>>>,
}

impl SessionCreationCommunicationHandle {
    /// Creates a new instance which may then be cloned to create more references to the underlying channels.
    /// Also returns the receiving half of the SessionCreatedNotification channel.
    pub fn new(
        request_limit: usize,
    ) -> (Self, mpsc::UnboundedReceiver<SessionCreatedNotification>) {
        let (creation_tx, creation_rx) = mpsc::unbounded_channel();
        let status_listeners = Arc::new(Mutex::new(LruCache::new(request_limit)));

        let instance = Self {
            creation_tx,
            status_listeners,
        };

        (instance, creation_rx)
    }
}

/// Grouping type for all possible notifications in response to a session creation
pub enum StatusResponse {
    /// Session has been created and is now operational
    Operational(SessionOperationalNotification),
    /// Something went wrong during startup
    Failed(SessionStartupFailedNotification),
}
