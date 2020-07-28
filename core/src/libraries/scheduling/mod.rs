mod job;
mod job_scheduler;
mod status_server;
mod task_manager;

pub use job::Job;
pub use job_scheduler::JobScheduler;
pub use status_server::StatusServer;
pub use task_manager::{TaskManager, TaskResourceHandle};

use job_scheduler::JobStatus;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
