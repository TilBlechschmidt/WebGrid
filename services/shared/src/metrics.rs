use hyper::http::{Method, StatusCode};
use log::warn;
use redis::{aio::MultiplexedConnection, AsyncCommands, RedisResult};
use std::{fmt, fmt::Display};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

pub static SESSION_STARTUP_HISTOGRAM_BUCKETS: [i32; 16] = [
    2, 4, 6, 8, 10, 12, 14, 16, 32, 64, 128, 256, 512, 1024, 2048, 4096,
];

#[derive(Debug)]
pub enum SessionStatus {
    Queued,
    Pending,
    Alive,
    Terminated,
}

impl Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}

#[derive(Debug)]
pub enum MetricsEntry {
    IncomingTraffic(u64),
    OutgoingTraffic(u64),
    RequestProcessed(Method, StatusCode),
    SessionStarted(f64),
    // TODO This needs to be used somewhere.
    SessionStatusChange(SessionStatus),
}

pub struct MetricsProcessor {
    tx: UnboundedSender<MetricsEntry>,
    rx: UnboundedReceiver<MetricsEntry>,
    con: MultiplexedConnection,
}

impl MetricsProcessor {
    pub fn new(con: &MultiplexedConnection) -> Self {
        let (tx, rx) = unbounded_channel();

        Self {
            tx,
            rx,
            con: con.clone(),
        }
    }

    pub async fn process(&mut self) {
        while let Some(entry) = self.rx.recv().await {
            if let Err(e) = self.process_entry(entry).await {
                warn!("Failed to update metric: {:?}", e);
            }
        }
    }

    pub fn get_tx(&self) -> UnboundedSender<MetricsEntry> {
        self.tx.clone()
    }

    async fn process_entry(&mut self, entry: MetricsEntry) -> RedisResult<()> {
        match entry {
            MetricsEntry::IncomingTraffic(bytes) => {
                self.con
                    .hincr::<_, _, _, ()>("metrics:http:net.bytes.total", "in", bytes)
                    .await
            }
            MetricsEntry::OutgoingTraffic(bytes) => {
                self.con
                    .hincr::<_, _, _, ()>("metrics:http:net.bytes.total", "out", bytes)
                    .await
            }
            MetricsEntry::RequestProcessed(method, status) => {
                let key = format!("metrics:http:requestsTotal:{}", method.as_str().to_owned());
                self.con.hincr::<_, _, _, ()>(key, status.as_u16(), 1).await
            }
            MetricsEntry::SessionStatusChange(new_status) => {
                self.con
                    .hincr::<_, _, _, ()>("metrics:sessions:total", format!("{}", new_status), 1)
                    .await
            }
            MetricsEntry::SessionStarted(elapsed_time) => {
                self.process_session_startup_histogram_entry(elapsed_time)
                    .await
            }
        }
    }

    async fn process_session_startup_histogram_entry(&self, elapsed_time: f64) -> RedisResult<()> {
        let mut con = self.con.clone();
        let base_key = "metrics:sessions:startup.histogram";
        let buckets_key = format!("{}:buckets", base_key);
        let count_key = format!("{}:count", base_key);
        let sum_key = format!("{}:sum", base_key);

        for bucket in SESSION_STARTUP_HISTOGRAM_BUCKETS.iter() {
            let float_bucket: f64 = (*bucket).into();

            if float_bucket > elapsed_time {
                con.hincr::<_, _, _, ()>(&buckets_key, *bucket, 1).await?;
            }
        }

        con.hincr::<_, _, _, ()>(&buckets_key, "+Inf", 1).await?;
        con.incr::<_, _, ()>(count_key, 1).await?;
        con.incr::<_, _, ()>(sum_key, elapsed_time).await
    }
}
