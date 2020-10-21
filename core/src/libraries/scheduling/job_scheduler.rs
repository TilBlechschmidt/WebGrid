use crate::libraries::helpers::Backoff;
use futures::{
    channel::{mpsc::Receiver, oneshot::Receiver as OneShotReceiver},
    future::{abortable, AbortHandle, Aborted},
    lock::Mutex,
    prelude::*,
};
use log::{debug, error, info, warn};
use std::{
    collections::HashMap,
    fmt,
    ops::Deref,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{sync::watch::Sender as WatchSender, task, task::JoinHandle, time::delay_for};

use super::job::Job;
use super::task_manager::{ResourceStatus, TaskManager};

static TASK_ID_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// State in which a job currently resides
#[derive(Debug)]
pub enum JobStatus {
    /// Job has started and is ready to fulfill contracts. Contains graceful termination handle if supported.
    Ready(Option<WatchSender<Option<()>>>),
    /// Job has never started and is in the process of getting ready
    Startup,
    /// Job was restarted due to a missing dependency and is getting ready
    Restarting,
    /// Job has exited with an error and is currently waiting before it retries
    CrashLoopBackOff,
    /// Job has exceeded its crash loop limit (clean shutdown or forced termination cause a removal of the job from the status list)
    Terminated,
    /// Job has exited cleanly
    Finished,
}

impl fmt::Display for JobStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            JobStatus::Ready(_) => write!(f, "Ready"),
            _ => write!(f, "{:?}", self),
        }
    }
}

impl PartialEq for JobStatus {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (&JobStatus::Startup, &JobStatus::Startup) => true,
            (&JobStatus::Restarting, &JobStatus::Restarting) => true,
            (&JobStatus::CrashLoopBackOff, &JobStatus::CrashLoopBackOff) => true,
            (&JobStatus::Terminated, &JobStatus::Terminated) => true,
            (&JobStatus::Ready(_), &JobStatus::Ready(_)) => true,
            _ => false,
        }
    }
}

impl Eq for JobStatus {}

impl JobStatus {
    fn is_gracefully_terminatable(&self) -> bool {
        matches!(*self, JobStatus::Ready(Some(_)))
    }
}

/// Job and task lifecycle handler
#[derive(Default)]
pub struct JobScheduler {
    pub(crate) status: Arc<Mutex<HashMap<String, JobStatus>>>,
    termination_handles: Arc<Mutex<HashMap<String, AbortHandle>>>,
}

impl JobScheduler {
    fn add_dependency_watcher(
        mut rx: Receiver<ResourceStatus>,
        abort_handle: AbortHandle,
    ) -> AbortableJoinHandle<()> {
        spawn_abortable(async move {
            #[allow(clippy::never_loop)]
            while let Some(status) = rx.next().await {
                match status {
                    ResourceStatus::Dead => {
                        abort_handle.abort();
                        break;
                    }
                };
            }
        })
    }

    fn add_status_watcher(
        readiness_rx: OneShotReceiver<()>,
        termination_tx: Option<WatchSender<Option<()>>>,
        status_map: Arc<Mutex<HashMap<String, JobStatus>>>,
        job_name: String,
    ) -> AbortableJoinHandle<()> {
        spawn_abortable(async move {
            if readiness_rx.await.is_ok() {
                // TODO Reset job crash backoff counter to zero after a successful start
                JobScheduler::change_status(
                    &status_map,
                    &job_name,
                    JobStatus::Ready(termination_tx),
                )
                .await;
            }
        })
    }

    async fn change_status(
        status_map: &Arc<Mutex<HashMap<String, JobStatus>>>,
        job_name: &str,
        status: JobStatus,
    ) {
        info!("{:<16} {}", format!("{}", status), job_name);
        status_map.lock().await.insert(job_name.to_owned(), status);
    }

    /// Run a new task with the given context on the default scheduler
    ///
    /// This method makes the given future abortable and provides access to dependencies and terminates it if required dependencies become unavailable.
    pub fn spawn_task<T, F: 'static + Send, O: 'static + Send, Context>(
        task: &T,
        ctx: Context,
    ) -> JoinHandle<Result<O, Aborted>>
    where
        F: Future<Output = O>,
        T: Fn(TaskManager<Context>) -> F,
    {
        let task_id = TASK_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
        let (manager, rx, _, _) = TaskManager::new(task_id, ctx);
        let (future, abort_handle) = abortable(task(manager));
        let dependency_handle = JobScheduler::add_dependency_watcher(rx, abort_handle);

        task::spawn(async move {
            let result = future.await;
            dependency_handle.cancel();
            result
        })
    }

    async fn manage_job_lifecycle<J: 'static + Job + Send>(
        job: J,
        ctx: J::Context,
        status_map: Arc<Mutex<HashMap<String, JobStatus>>>,
    ) where
        J::Context: Send + Clone,
    {
        let job_name = job.name().to_owned();
        let mut backoff = Backoff::default();

        // TODO Handle non-unique job names!

        JobScheduler::change_status(&status_map, &job_name, JobStatus::Startup).await;
        loop {
            let job_instance_id = TASK_ID_COUNTER.fetch_add(1, Ordering::SeqCst);
            let (manager, dependency_rx, readiness_rx, termination_tx) =
                TaskManager::new(job_instance_id, ctx.clone());

            let wrapped_termination_tx = if job.supports_graceful_termination() {
                Some(termination_tx)
            } else {
                None
            };

            // Create an instance and wrap it in two abortables for dependency loss and external termination
            let instance = job.execute(manager);
            let (dependent_future, dependency_abort_handle) = abortable(instance);

            let dependency_handle =
                JobScheduler::add_dependency_watcher(dependency_rx, dependency_abort_handle);
            let status_handle = JobScheduler::add_status_watcher(
                readiness_rx,
                wrapped_termination_tx,
                status_map.clone(),
                job_name.clone(),
            );

            let result = dependent_future.await;

            dependency_handle.cancel();
            status_handle.cancel();

            // Match for resource lock abort
            match result {
                // Match for return value
                Ok(return_value) => match return_value {
                    Ok(_) => {
                        JobScheduler::change_status(&status_map, &job_name, JobStatus::Finished)
                            .await;
                        status_map.lock().await.remove(&job_name);
                        break;
                    }
                    Err(e) => {
                        error!("{} crashed: {:?}", job_name.clone(), e);
                        JobScheduler::change_status(
                            &status_map,
                            &job_name,
                            JobStatus::CrashLoopBackOff,
                        )
                        .await;

                        if let Some(sleep_duration) = backoff.next() {
                            debug!("{} backing off for {:?}", &job_name, sleep_duration);
                            delay_for(sleep_duration).await;
                        } else {
                            error!("{} exceeded its retry limit!", &job_name);
                            JobScheduler::change_status(
                                &status_map,
                                &job_name,
                                JobStatus::Terminated,
                            )
                            .await;
                            // TODO Call process termination closure provided to the manager
                            return;
                        }
                    }
                },
                Err(_) => warn!("{} lost a resource lock", &job_name),
            }

            JobScheduler::change_status(&status_map, &job_name, JobStatus::Restarting).await;
        }
    }

    /// Manage a new job
    ///
    /// This method respawns the job if it crashes, provides access to dependencies, keeps track of its lifecycle and restarts it if dependencies become unavailable.
    pub fn spawn_job<J: 'static + Job + Send>(&self, job: J, ctx: J::Context)
    where
        J::Context: Send + Clone,
    {
        let status_map = self.status.clone();
        let termination_handles = self.termination_handles.clone();
        let job_name = job.name().to_owned();

        task::spawn(async move {
            let (job_lifecycle, termination_handle) = abortable(
                JobScheduler::manage_job_lifecycle(job, ctx, status_map.clone()),
            );

            termination_handles
                .lock()
                .await
                .insert(job_name.clone(), termination_handle);

            if job_lifecycle.await.is_err() {
                JobScheduler::change_status(&status_map, &job_name, JobStatus::Terminated).await;
            }

            termination_handles.lock().await.remove(&job_name);
            status_map.lock().await.remove(&job_name);
        });
    }

    /// Gracefully terminates all managed jobs that support it
    pub async fn terminate_jobs(&self) {
        // 1. Send termination signal to jobs that support graceful shutdown and terminate ones that don't (or ones that aren't running)
        {
            let status = self.status.lock().await;

            for (job_name, status) in status.iter() {
                if let JobStatus::Ready(Some(graceful_handle)) = status {
                    graceful_handle.broadcast(Some(())).ok();
                } else if let Some(forceful_handle) =
                    self.termination_handles.lock().await.get(job_name)
                {
                    forceful_handle.abort();
                }
            }
        }

        // 2. Give alive jobs some time to gracefully terminate (if applicable)
        // TODO Make duration an environment variable or property
        for _ in 0..60000 {
            {
                let termination_handles = self.termination_handles.lock().await;
                let status = self.status.lock().await;

                // Filter out handles that are associated with non-ready jobs
                // Reason: If a job is gracefully terminatable but enters a crashed state during graceful termination
                //          it would block the termination for the grace period. However, it is more reasonable to just ignore it.
                let graceful_handles: Vec<&String> = termination_handles
                    .keys()
                    .filter(|job_name| {
                        if let Some(job_status) = status.get(*job_name) {
                            job_status.is_gracefully_terminatable()
                        } else {
                            false
                        }
                    })
                    .collect();

                if graceful_handles.is_empty() {
                    break;
                }
            }

            delay_for(Duration::from_millis(10)).await;
        }

        // 3. Call termination handle for all remaining jobs
        for (job_name, handle) in self.termination_handles.lock().await.iter() {
            warn!("{} ignored graceful termination request", job_name);
            handle.abort()
        }
    }
}

/// Schedule jobs on a given scheduler with some context
#[macro_export]
macro_rules! schedule {
    ($scheduler:expr, $context:expr, { $($job:ident$(,)? )+ }) => {
        $(
            $scheduler.spawn_job($job.clone(), $context.clone());
        )+
    };
}

pub struct AbortableJoinHandle<O> {
    join_handle: JoinHandle<Result<O, Aborted>>,
    abort_handle: AbortHandle,
}

impl<O> AbortableJoinHandle<O> {
    pub fn cancel(&self) {
        self.abort_handle.abort()
    }
}

impl<O> Deref for AbortableJoinHandle<O> {
    type Target = JoinHandle<Result<O, Aborted>>;

    fn deref(&self) -> &Self::Target {
        &self.join_handle
    }
}

pub fn spawn_abortable<F: 'static + Send, O: 'static + Send>(fut: F) -> AbortableJoinHandle<O>
where
    F: Future<Output = O>,
{
    let (future, abort_handle) = abortable(fut);
    AbortableJoinHandle {
        join_handle: task::spawn(future),
        abort_handle,
    }
}
