use futures::lock::Mutex;
use jatsl::TaskResourceHandle;
use lazy_static::lazy_static;
use std::collections::HashSet;
use std::ops::{Deref, DerefMut};
use tokio::task;

lazy_static! {
    pub(super) static ref SHARED_TASK_RESOURCE_HANDLES: Mutex<HashSet<TaskResourceHandle>> =
        Mutex::new(HashSet::new());
}

#[derive(Clone)]
pub(super) struct HandleRegistration {
    pub tx: TaskResourceHandle,
    pub is_shared: bool,
}

impl DerefMut for HandleRegistration {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tx
    }
}

impl Deref for HandleRegistration {
    type Target = TaskResourceHandle;

    fn deref(&self) -> &Self::Target {
        &self.tx
    }
}

impl Drop for HandleRegistration {
    fn drop(&mut self) {
        if self.is_shared {
            // If this was a shared resource lazily remove the resource handle from list of active handles
            let handle = self.tx.clone();
            task::spawn(async {
                let handle = handle;
                SHARED_TASK_RESOURCE_HANDLES.lock().await.remove(&handle);
            });
        }
    }
}
