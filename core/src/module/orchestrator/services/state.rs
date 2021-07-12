use crate::domain::event::SessionIdentifier;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{AcquireError, Mutex, OwnedSemaphorePermit, Semaphore};

/// Keeps track of deployed sessions and manages permits for new ones
#[derive(Clone)]
pub struct ProvisioningState {
    /// Provides permits for new sessions
    semaphore: Arc<Semaphore>,
    /// Holds the semaphore permits held by each session managed by this provisioner
    managed: Arc<Mutex<HashMap<SessionIdentifier, OwnedSemaphorePermit>>>,
}

impl ProvisioningState {
    /// Creates a new instance with the given number of initially available permits
    pub fn new(permits: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(permits)),
            managed: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Acquires a permit for a new session with the given identifier.
    /// If all permits have been used up, it waits asynchronously until one is released.
    ///
    /// Internally, this process relies on a [`Semaphore`].
    pub async fn acquire_permit(&self, session: SessionIdentifier) -> Result<(), AcquireError> {
        let permit = self.semaphore.clone().acquire_owned().await?;
        self.managed.lock().await.insert(session, permit);
        Ok(())
    }

    /// Releases a permit held by a session with the given identifier.
    /// If no permit for the given [`SessionIdentifier`] exists, this function returns none.
    pub async fn release_permit(&self, session: &SessionIdentifier) -> Option<()> {
        self.managed.lock().await.remove(session).map(|_| ())
    }

    /// Releases all permits held by sessions that are not in the `alive_sessions` list passed in
    pub async fn release_dead_sessions(&self, alive_sessions: Vec<SessionIdentifier>) {
        let mut managed = self.managed.lock().await;
        let mut dead = Vec::new();

        for managed_session in managed.keys() {
            if !alive_sessions.contains(managed_session) {
                dead.push(managed_session.to_owned());
            }
        }

        for id in dead {
            managed.remove(&id);
        }
    }

    /// Returns the number of currently available permits
    #[cfg(test)]
    pub fn available_permits(&self) -> usize {
        self.semaphore.available_permits()
    }
}

#[cfg(test)]
mod does {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn provide_the_correct_number_of_permits() {
        let permits = 10;
        let state = ProvisioningState::new(permits);

        for _ in 0..permits {
            state.acquire_permit(Uuid::new_v4()).await.unwrap();
        }

        assert_eq!(state.available_permits(), 0);
    }

    #[tokio::test]
    async fn return_permits() {
        let id = Uuid::new_v4();
        let state = ProvisioningState::new(1);

        state.acquire_permit(id).await.unwrap();
        assert_eq!(state.available_permits(), 0);

        assert!(state.release_permit(&id).await.is_some());
        assert_eq!(state.available_permits(), 1);
    }

    #[tokio::test]
    async fn release_dead_permits() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let state = ProvisioningState::new(2);

        state.acquire_permit(id1).await.unwrap();
        state.acquire_permit(id2).await.unwrap();
        assert_eq!(state.available_permits(), 0);

        state.release_dead_sessions(vec![id2]).await;
        assert_eq!(state.available_permits(), 1);
    }
}
