use futures::{
    channel::{
        mpsc::{channel, Receiver, Sender},
        oneshot::{
            channel as one_shot_channel, Receiver as OneShotReceiver, Sender as OneShotSender,
        },
    },
    lock::Mutex,
    sink::SinkExt,
};
use std::{
    hash::{Hash, Hasher},
    sync::Arc,
};
use tokio::sync::watch::{
    channel as watchChannel, Receiver as WatchReceiver, Sender as WatchSender,
};

/// Whether or not a resource is available
#[derive(Debug)]
pub enum ResourceStatus {
    Dead,
}

type TaskID = usize;
pub type ResourceStatusSender = Sender<ResourceStatus>;

/// Manager for tasks and jobs
///
/// Provides context, handles resource availability changes and graceful termination
#[derive(Clone)]
pub struct TaskManager<Context> {
    task_id: TaskID,
    dependency_tx: ResourceStatusSender,
    readiness_tx: Arc<Mutex<Option<OneShotSender<()>>>>,
    termination_rx: WatchReceiver<Option<()>>,
    pub context: Context,
}

impl<Context> TaskManager<Context> {
    /// Create a new task manager for the given task and context
    pub fn new(
        task_id: TaskID,
        context: Context,
    ) -> (
        Self,
        Receiver<ResourceStatus>,
        OneShotReceiver<()>,
        WatchSender<Option<()>>,
    ) {
        let (dependency_tx, dependency_rx) = channel(16);
        let (readiness_tx, readiness_rx) = one_shot_channel();
        let (termination_tx, termination_rx) = watchChannel(None);

        let manager = Self {
            task_id,
            dependency_tx,
            readiness_tx: Arc::new(Mutex::new(Some(readiness_tx))),
            termination_rx,
            context,
        };

        (manager, dependency_rx, readiness_rx, termination_tx)
    }

    /// Create a new resource handle to notify about resource state
    pub fn create_resource_handle(&self) -> TaskResourceHandle {
        TaskResourceHandle {
            task_id: self.task_id,
            dependency_tx: self.dependency_tx.clone(),
        }
    }

    /// Future that completes when the job should gracefully shutdown
    pub fn termination_signal(&self) -> impl futures::Future<Output = ()> {
        let mut rx = self.termination_rx.clone();
        async move {
            rx.changed().await.ok();
        }
    }

    /// Check if the job should enter graceful shutdown
    pub fn termination_signal_triggered(&self) -> bool {
        *self.termination_rx.borrow() == Some(())
    }

    /// Function to indicate to the scheduler that this job is ready to fulfull its contract.
    /// This has no effect when called from within tasks.
    pub async fn ready(&self) {
        if let Some(tx) = self.readiness_tx.lock().await.take() {
            tx.send(()).ok();
        }
    }
}

/// Notification handle for a resource
///
/// Used to update the task manager about the status of a resource
#[derive(Clone)]
pub struct TaskResourceHandle {
    task_id: TaskID,
    dependency_tx: ResourceStatusSender,
}

impl TaskResourceHandle {
    #[cfg(test)]
    pub fn stub() -> Self {
        let (dependency_tx, _) = channel(0);
        Self {
            task_id: 0,
            dependency_tx,
        }
    }

    /// Notifies the task manager that a resource dependency has become unavailable
    pub async fn resource_died(&mut self) {
        // We can safely ignore this error. Most of the time the receiver is dropped right after reading the message, thus triggering a false error.
        self.dependency_tx.send(ResourceStatus::Dead).await.ok();
    }
}

impl PartialEq for TaskResourceHandle {
    fn eq(&self, other: &Self) -> bool {
        self.task_id == other.task_id
    }
}
impl Eq for TaskResourceHandle {}

impl Hash for TaskResourceHandle {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.task_id.hash(state);
    }
}
