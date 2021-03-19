use super::entry::MetricsEntry;
use super::SESSION_STARTUP_HISTOGRAM_BUCKETS;
use crate::libraries::scheduling::{Job, TaskManager};
use crate::{
    libraries::resources::{ResourceManager, ResourceManagerProvider},
    with_shared_redis_resource,
};
use anyhow::Result;
use async_trait::async_trait;
use log::warn;
use redis::{aio::ConnectionLike, AsyncCommands, RedisResult};
use std::{marker::PhantomData, sync::Arc};
use tokio::sync::{
    mpsc::{error::SendError, unbounded_channel, UnboundedReceiver, UnboundedSender},
    Mutex,
};

#[derive(Clone)]
pub struct MetricsProcessor<C, R> {
    tx: UnboundedSender<MetricsEntry>,
    rx: Arc<Mutex<UnboundedReceiver<MetricsEntry>>>,
    phantom_c: PhantomData<C>,
    phantom_r: PhantomData<R>,
}

impl<C, R> Default for MetricsProcessor<C, R> {
    fn default() -> Self {
        let (tx, rx) = unbounded_channel();

        Self {
            tx,
            rx: Arc::new(Mutex::new(rx)),
            phantom_c: PhantomData,
            phantom_r: PhantomData,
        }
    }
}

impl<C, R> MetricsProcessor<C, R> {
    pub fn submit(&self, entry: MetricsEntry) -> Result<(), SendError<MetricsEntry>> {
        self.tx.send(entry)
    }

    async fn process<Redis: AsyncCommands + ConnectionLike>(&self, con: &mut Redis) {
        let rx = self.rx.clone();
        let mut rx_lock = rx.lock().await;

        while let Some(entry) = rx_lock.recv().await {
            if let Err(e) = self.process_entry(con, entry).await {
                warn!("Failed to update metric: {:?}", e);
            }
        }
    }

    async fn process_entry<Redis: AsyncCommands + ConnectionLike>(
        &self,
        con: &mut Redis,
        entry: MetricsEntry,
    ) -> RedisResult<()> {
        match entry {
            MetricsEntry::IncomingTraffic(bytes) => {
                con.hincr::<_, _, _, ()>("metrics:http:net.bytes.total", "in", bytes)
                    .await
            }
            MetricsEntry::OutgoingTraffic(bytes) => {
                con.hincr::<_, _, _, ()>("metrics:http:net.bytes.total", "out", bytes)
                    .await
            }
            MetricsEntry::RequestProcessed(method, status) => {
                let key = format!("metrics:http:requestsTotal:{}", method.as_str().to_owned());
                con.hincr::<_, _, _, ()>(key, status.as_u16(), 1).await
            }
            MetricsEntry::SessionStatusChange(new_status) => {
                con.hincr::<_, _, _, ()>("metrics:sessions:total", format!("{}", new_status), 1)
                    .await
            }
            MetricsEntry::SessionStarted(elapsed_time) => {
                self.process_session_startup_histogram_entry(con, elapsed_time)
                    .await
            }
        }
    }

    async fn process_session_startup_histogram_entry<Redis: AsyncCommands + ConnectionLike>(
        &self,
        con: &mut Redis,
        elapsed_time: f64,
    ) -> RedisResult<()> {
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

#[async_trait]
impl<R: ResourceManager + Send + Sync, C: ResourceManagerProvider<R> + Send + Sync> Job
    for MetricsProcessor<C, R>
{
    type Context = C;

    const NAME: &'static str = module_path!();

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        let mut con = with_shared_redis_resource!(manager);

        manager.ready().await;
        self.process(&mut con).await;

        Ok(())
    }
}
