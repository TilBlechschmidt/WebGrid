//! Structures for database heartbeats

use crate::libraries::resources::ResourceManagerProvider;
use crate::libraries::{
    helpers::keys,
    resources::{PubSub, ResourceManager},
};
use crate::{with_redis_resource, with_shared_redis_resource};
use anyhow::{Context, Result};
use async_trait::async_trait;
use chrono::Utc;
use futures::{lock::Mutex, StreamExt};
use jatsl::{Job, TaskManager};
use log::debug;
use redis::AsyncCommands;
use std::{collections::HashMap, marker::PhantomData, sync::Arc, time::Duration};
use tokio::{pin, select, time::sleep};

/// State change of a heartbeat
enum BeatChange {
    /// Addition of a heartbeat by key and expiration time
    Add(String, usize),
    /// Removal of a heartbeat key
    Expire(String),
}

/// Content of a heartbeat
pub enum BeatValue {
    Timestamp,
    Constant(String),
}

/// Job which keeps heartbeats in the database up-to-date
///
/// This handler has to be executed by a job scheduler to operate.
#[derive(Clone)]
pub struct HeartBeat<C, R> {
    value: Arc<BeatValue>,
    /// Pending changes
    changes: Arc<Mutex<Vec<BeatChange>>>,
    /// Currently active beats, their interval and expiration in seconds
    beats: Arc<Mutex<HashMap<String, (usize, usize)>>>,
    /// If this is set, all beats will be refreshed during the next interval
    force_refresh: Arc<Mutex<bool>>,
    phantom_c: PhantomData<C>,
    phantom_r: PhantomData<R>,
}

impl<C, R> Default for HeartBeat<C, R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<C, R> HeartBeat<C, R> {
    /// Creates a new handler with timestamp based heartbeats
    pub fn new() -> Self {
        HeartBeat::with_value(BeatValue::Timestamp)
    }

    /// Creates a new handler with a custom value type
    pub fn with_value(value: BeatValue) -> Self {
        Self {
            value: Arc::new(value),
            changes: Arc::new(Mutex::new(Vec::new())),
            beats: Arc::new(Mutex::new(HashMap::new())),
            force_refresh: Arc::new(Mutex::new(false)),
            phantom_c: PhantomData,
            phantom_r: PhantomData,
        }
    }

    /// Force a refresh cycle on the next iteration
    pub async fn force_refresh(&self) {
        *self.force_refresh.lock().await = true;
    }

    /// Add a new beat with a specified interval and expiration time
    pub async fn add_beat(&self, key: &str, interval_secs: usize, expiration_secs: usize) {
        debug!("Added heartbeat {}", key);

        self.beats
            .lock()
            .await
            .insert(key.to_owned(), (interval_secs, expiration_secs));

        self.changes
            .lock()
            .await
            .push(BeatChange::Add(key.to_owned(), expiration_secs));
    }

    /// Remove a heartbeat
    pub async fn stop_beat(&self, key: &str) {
        debug!("Removed heartbeat {}", key);

        self.beats.lock().await.remove(key);
        self.changes
            .lock()
            .await
            .push(BeatChange::Expire(key.to_owned()));
    }
}

#[async_trait]
impl<R: ResourceManager + Send + Sync, C: ResourceManagerProvider<R> + Send + Sync> Job
    for HeartBeat<C, R>
{
    type Context = C;

    const NAME: &'static str = module_path!();
    const SUPPORTS_GRACEFUL_TERMINATION: bool = true;

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        let mut redis = with_shared_redis_resource!(manager);
        let mut pubsub: PubSub = with_redis_resource!(manager).into();

        pubsub.subscribe(&*keys::HEARTBEAT_REFRESH_CHANNEL).await?;
        let force_refresh_stream = pubsub.on_message();
        pin!(force_refresh_stream);

        manager.ready().await;

        let interval = 1;
        let mut passed_time: usize = 0;
        let mut terminating = false;

        loop {
            let value = match &(*self.value) {
                BeatValue::Timestamp => Utc::now().to_rfc3339(),
                BeatValue::Constant(value) => value.to_owned(),
            };

            let mut force_refresh = false;

            // Load and consume the force_refresh value from self
            {
                let mut refresh = self.force_refresh.lock().await;
                if *refresh {
                    force_refresh = true;
                    *refresh = false;
                }
            }

            // Shut down gracefully if termination signal has been triggered
            if manager.termination_signal_triggered() {
                let keys: Vec<String> = self
                    .beats
                    .lock()
                    .await
                    .keys()
                    .map(|s| s.to_owned())
                    .collect();

                for key in keys {
                    self.stop_beat(&key).await;
                }

                terminating = true;
            }

            // Process changes
            while let Some(change) = self.changes.lock().await.pop() {
                match change {
                    BeatChange::Add(key, expiration_secs) => {
                        redis
                            .set_ex::<_, _, ()>(key.clone(), value.clone(), expiration_secs)
                            .await
                            .context(format!("unable to add beat at {}", key))?;
                    }
                    BeatChange::Expire(key) => {
                        redis
                            .expire::<_, ()>(key.clone(), 1)
                            .await
                            .context(format!("unable to expire beat at {}", key))?;
                    }
                }
            }

            // Update beats
            for (key, (refresh_time, expiration_time)) in self.beats.lock().await.iter() {
                if passed_time % refresh_time == 0 || force_refresh {
                    redis
                        .set_ex::<_, _, ()>(key, value.clone(), *expiration_time)
                        .await
                        .context(format!("unable to update beat at {}", key))?;
                }
            }

            // Exit if this iteration is a graceful termination
            if terminating {
                return Ok(());
            }

            // Wait for the specified interval or someone requesting a forced refresh
            let sleep_future = sleep(Duration::from_secs(interval as u64));
            let needs_refresh = select! {
                _ = force_refresh_stream.next() => true,
                _ = sleep_future => false,
            };

            if needs_refresh {
                self.force_refresh().await;
            }

            passed_time += interval;
        }
    }
}
