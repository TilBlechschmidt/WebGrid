use super::BoxedError;
use futures::StreamExt;
#[cfg(target_os = "linux")]
use heim::process::os::linux::ProcessExt;
use heim::process::{self, Process, ProcessError, ProcessResult};
use heim::units::information::byte;
use heim::units::time::second;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::sync::Mutex;
use tokio::time::sleep;
use tracing::{info, warn};

/// Metrics about a process accumulated over a certain timespan
#[allow(missing_docs)]
#[derive(Debug, Default, Serialize, Deserialize, Clone, PartialEq)]
pub struct AccumulatedPerformanceMetrics {
    /// Label given to the process by the observer
    #[serde(skip)]
    pub label: String,

    // Intermediary fields to calculate derived metrics (e.g. mean)
    #[serde(skip)]
    memory_sample_count: u64,
    #[serde(skip)]
    memory_rss_total: u64,
    #[serde(skip)]
    memory_vms_total: u64,

    // Mean memory
    pub memory_rss_mean: u64,
    pub memory_vms_mean: u64,

    // Max memory
    pub memory_rss_max: u64,
    pub memory_vms_max: u64,

    // Process time
    pub cpu_time_usr: f64,
    pub cpu_time_sys: f64,
    pub wall_time: f64,

    // Disk I/O
    pub disk_read: u64,
    pub disk_write: u64,
}

impl AccumulatedPerformanceMetrics {
    fn new(label: String) -> Self {
        Self {
            label,
            ..Default::default()
        }
    }

    fn add(&mut self, sample: PerformanceMetrics) {
        self.memory_sample_count += 1;
        self.memory_rss_total += sample.memory_rss;
        self.memory_vms_total += sample.memory_vms;

        self.memory_rss_mean = self.memory_rss_total / self.memory_sample_count;
        self.memory_vms_mean = self.memory_vms_total / self.memory_sample_count;

        self.memory_rss_max = self.memory_rss_max.max(sample.memory_rss);
        self.memory_vms_max = self.memory_vms_max.max(sample.memory_vms);

        self.cpu_time_usr = sample.cpu_time_usr;
        self.cpu_time_sys = sample.cpu_time_sys;
        self.wall_time = sample.wall_time;

        self.disk_read = sample.disk_read;
        self.disk_write = sample.disk_write;
    }
}

#[derive(Debug, Default)]
struct PerformanceMetrics {
    memory_rss: u64,
    memory_vms: u64,

    cpu_time_usr: f64,
    cpu_time_sys: f64,
    wall_time: f64,

    disk_read: u64,
    disk_write: u64,
}

/// Tool to collect [`AccumulatedPerformanceMetrics`] for a given process
pub struct PerformanceMonitor;

#[derive(Debug, Error)]
enum PerformanceMonitorError {
    #[error("memory field not found")]
    MemFieldNotFound,
}

/// Type of target to monitor the performance of
pub enum PerformanceMonitoringTarget {
    /// Fetches information about the current cgroup. Requires a Linux host with cgroupfs v2 enabled. May not support all fields.
    CurrentCgroup,
    /// Currently running process, supported on all operating systems.
    Process(Process),
}

impl PerformanceMonitor {
    async fn sample_cgroup() -> Result<PerformanceMetrics, BoxedError> {
        let cpu_sys_path = "/sys/fs/cgroup/cpuacct/cpuacct.usage_sys";
        let cpu_usr_path = "/sys/fs/cgroup/cpuacct/cpuacct.usage_user";
        let cpu_factor = 1.0 / 1_000_000_000.0; // values are in nanoseconds, we expect seconds

        let mem_stat_path = "/sys/fs/cgroup/memory/memory.stat";
        let mem_field = "total_rss ";

        let cpu_time_sys_raw = tokio::fs::read_to_string(cpu_sys_path).await?;
        let cpu_time_sys = cpu_time_sys_raw.trim().parse::<f64>()? * cpu_factor;

        let cpu_time_usr_raw = tokio::fs::read_to_string(cpu_usr_path).await?;
        let cpu_time_usr = cpu_time_usr_raw.trim().parse::<f64>()? * cpu_factor;

        let memory_rss_raw = tokio::fs::read_to_string(mem_stat_path).await?;
        let memory_rss = memory_rss_raw
            .lines()
            .filter(|l| l.contains(mem_field))
            .map(|l| l.strip_prefix(mem_field))
            .flatten()
            .collect::<Vec<_>>()
            .first()
            .ok_or(PerformanceMonitorError::MemFieldNotFound)?
            .trim()
            .parse::<u64>()?;

        let cgroup_root_process = process::get(1).await?;
        let create_time_raw = cgroup_root_process.create_time().await?;
        let create_time = Duration::from_secs_f64(create_time_raw.get::<second>());
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?;
        let wall_time = (current_time - create_time).as_secs_f64();

        Ok(PerformanceMetrics {
            memory_rss,
            memory_vms: 0,

            cpu_time_usr,
            cpu_time_sys,
            wall_time,

            disk_read: 0,
            disk_write: 0,
        })
    }

    async fn sample(process: &Process) -> Result<PerformanceMetrics, BoxedError> {
        let memory = process.memory().await?;
        let cpu_time = process.cpu_time().await?;
        let create_time_raw = process.create_time().await?;
        let io_counters = process.io_counters().await?;
        #[cfg(target_os = "linux")]
        let _net_io_counters = process.net_io_counters().await?;

        let memory_rss = memory.rss().get::<byte>();
        let memory_vms = memory.vms().get::<byte>();

        let cpu_time_usr = cpu_time.user().get::<second>();
        let cpu_time_sys = cpu_time.system().get::<second>();

        let create_time = Duration::from_secs_f64(create_time_raw.get::<second>());
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?;
        let wall_time = (current_time - create_time).as_secs_f64();

        let disk_read = io_counters.bytes_read().get::<byte>();
        let disk_write = io_counters.bytes_written().get::<byte>();

        Ok(PerformanceMetrics {
            memory_rss,
            memory_vms,

            cpu_time_usr,
            cpu_time_sys,
            wall_time,

            disk_read,
            disk_write,
        })
    }

    /// Spawns a background worker that periodically samples a process
    /// and adds the collected information to the [`AccumulatedPerformanceMetrics`] instance returned.
    pub fn observe(
        target: PerformanceMonitoringTarget,
        label: String,
        interval: Duration,
    ) -> Arc<Mutex<AccumulatedPerformanceMetrics>> {
        let metrics = Arc::new(Mutex::new(AccumulatedPerformanceMetrics::new(
            label.clone(),
        )));
        let metrics_handle = metrics.clone();

        tokio::spawn(async move {
            info!(?label, "Profiling process");

            loop {
                let result = match &target {
                    PerformanceMonitoringTarget::Process(p) => PerformanceMonitor::sample(p).await,
                    PerformanceMonitoringTarget::CurrentCgroup => {
                        PerformanceMonitor::sample_cgroup().await
                    }
                };

                match result {
                    Ok(sample) => {
                        let mut metrics = metrics.lock().await;
                        metrics.add(sample);
                    }
                    Err(error) => {
                        warn!(?label, ?error, "Encountered error while sampling process");
                        break;
                    }
                }

                if let PerformanceMonitoringTarget::Process(process) = &target {
                    if !process.is_running().await.unwrap_or(false) {
                        break;
                    }
                }

                sleep(interval).await;
            }

            info!(?label, "Stopped profiling process");
        });

        metrics_handle
    }

    /// Same as `observe` but attempts to fetch the process by its id
    pub async fn observe_by_pid(
        pid: i32,
        label: String,
        interval: Duration,
    ) -> ProcessResult<Arc<Mutex<AccumulatedPerformanceMetrics>>> {
        let process = process::get(pid).await?;
        Ok(PerformanceMonitor::observe(
            PerformanceMonitoringTarget::Process(process),
            label,
            interval,
        ))
    }

    /// Same as `observe` but samples the currently running process
    pub async fn observe_self(
        label: String,
        interval: Duration,
    ) -> ProcessResult<Arc<Mutex<AccumulatedPerformanceMetrics>>> {
        let pid = std::process::id() as i32;
        println!("OWN PID: {}", pid);
        PerformanceMonitor::observe_by_pid(pid, label, interval).await
    }

    /// Same as `observe` but attempts to fetch the process by its name
    pub async fn observe_by_name(
        name: String,
        label: String,
        interval: Duration,
    ) -> ProcessResult<Arc<Mutex<AccumulatedPerformanceMetrics>>> {
        let stream = heim::process::processes().await?;
        tokio::pin!(stream);

        while let Some(result) = stream.next().await {
            if let Ok(process) = result {
                if let Ok(actual_name) = process.name().await {
                    if actual_name == name {
                        return Ok(PerformanceMonitor::observe(
                            PerformanceMonitoringTarget::Process(process),
                            label,
                            interval,
                        ));
                    }
                }
            }
        }

        Err(ProcessError::NoSuchProcess(-1))
    }

    /// Attempts to recursively find all child-processes of a given parent
    pub async fn recursively_find_child_processes_of_pid(
        parent_pid: i32,
    ) -> ProcessResult<Vec<Process>> {
        // We assume that the stream of processes is always in ascending pid order — that way we do not have to do multiple passes
        let stream = heim::process::processes().await?;
        tokio::pin!(stream);

        let mut children_pids = Vec::new();
        let mut children = Vec::new();

        while let Some(result) = stream.next().await {
            if let Ok(process) = result {
                if let Ok(parent) = process.parent_pid().await {
                    if parent == parent_pid || children_pids.contains(&parent) {
                        children_pids.push(process.pid());
                        children.push(process);
                    }
                }
            }
        }

        Ok(children)
    }
}
