use super::super::{routing_info::RoutingInfo, Context};
use anyhow::{bail, Context as AnyhowContext, Result};
use async_trait::async_trait;
use futures::stream::TryStreamExt;
use lazy_static::lazy_static;
use log::{error, info};
use redis::{aio::ConnectionLike, cmd, AsyncCommands, Msg, RedisError};
use regex::Regex;
use resources::{with_redis_resource, PubSub, ResourceManager};
use scheduling::{Job, TaskManager};
use thiserror::Error;
use tokio::task::yield_now;

static PATTERN_MANAGER: &str = "__keyspace@0__:manager:*:heartbeat";
static PATTERN_SESSION: &str = "__keyspace@0__:session:*:heartbeat.node";

lazy_static! {
    static ref REGEX_MANAGER: Regex =
        Regex::new(r"__keyspace@0__:manager:(?P<mid>[^:]+):heartbeat").unwrap();
    static ref REGEX_SESSION: Regex =
        Regex::new(r"__keyspace@0__:session:(?P<sid>[^:]+):heartbeat\.node").unwrap();
}

#[derive(Error, Debug)]
pub enum WatcherJobError {
    #[error("error interacting with database")]
    RedisError(#[from] RedisError),
    #[error("redis server configuration verification failed")]
    InvalidKeyspaceProperties,
    #[error("redis notification stream ended unexpectedly")]
    UnexpectedTermination,
}

#[derive(Clone)]
pub struct WatcherJob {}

#[async_trait]
impl Job for WatcherJob {
    type Context = Context;

    const NAME: &'static str = module_path!();

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        let mut redis = with_redis_resource!(manager);
        let mut pubsub: PubSub = with_redis_resource!(manager).into();

        self.verify_keyspace_events_config(&mut redis).await?;

        pubsub
            .psubscribe(PATTERN_MANAGER)
            .await
            .context("unable to subscribe to manager list")?;

        pubsub
            .psubscribe(PATTERN_SESSION)
            .await
            .context("unable to subscribe to session list")?;

        manager.ready().await;

        let mut stream = pubsub.on_message();

        while let Ok(Some(msg)) = stream.try_next().await {
            self.process_message(msg, &manager.context.routing_info, &mut redis)
                .await?;
        }

        // Allow the job manager to terminate us so it doesn't count as a crash
        yield_now().await;

        bail!(WatcherJobError::UnexpectedTermination)
    }
}

impl WatcherJob {
    pub fn new() -> Self {
        Self {}
    }

    async fn verify_keyspace_events_config(
        &self,
        con: &mut impl ConnectionLike,
    ) -> Result<(), WatcherJobError> {
        let keyspace_events = cmd("CONFIG")
            .arg("GET")
            .arg("notify-keyspace-events")
            .query_async::<_, (String, String)>(con)
            .await;

        match keyspace_events {
            Ok((_key, events_value)) => {
                if !(events_value.contains('K')
                    && events_value.contains('g')
                    && events_value.contains('x'))
                {
                    error!("Redis server config does not contain the values 'Kgx' at the 'notify-keyspace-events' key");
                    Err(WatcherJobError::InvalidKeyspaceProperties)
                } else {
                    Ok(())
                }
            }
            Err(e) => Err(e.into()),
        }
    }

    async fn process_message(
        &self,
        msg: Msg,
        info: &RoutingInfo,
        con: &mut (impl ConnectionLike + AsyncCommands),
    ) -> Result<()> {
        let pattern = msg.get_pattern().ok();
        let channel: &str = msg.get_channel_name();
        let operation: String = msg.get_payload()?;

        if pattern == Some(PATTERN_MANAGER.to_string()) {
            self.handle_manager_message(channel, &operation, &info, con)
                .await;
        } else if pattern == Some(PATTERN_SESSION.to_string()) {
            self.handle_session_message(channel, &operation, &info, con)
                .await;
        }

        Ok(())
    }

    async fn handle_manager_message(
        &self,
        channel: &str,
        operation: &str,
        info: &RoutingInfo,
        con: &mut (impl ConnectionLike + AsyncCommands),
    ) {
        if let Some(caps) = REGEX_MANAGER.captures(channel) {
            let manager_id = &caps["mid"];

            match operation {
                // Manager has been added
                "expire" => {
                    let data_key = format!("manager:{}", manager_id);
                    let res = con
                        .hget::<_, _, (String, String)>(data_key, &["host", "port"])
                        .await;

                    if let Ok((host, port)) = res {
                        if info
                            .add_manager_upstream(manager_id.to_string(), &host, &port)
                            .await
                            .is_none()
                        {
                            info!("+ Manager {} @ {}:{}", manager_id, host, port);
                        }
                    }
                }
                // Manager has died
                "expired" => {
                    info!("- Manager {}", manager_id);
                    info.remove_manager_upstream(manager_id).await;
                }
                &_ => {}
            }
        }
    }

    async fn handle_session_message(
        &self,
        channel: &str,
        operation: &str,
        info: &RoutingInfo,
        con: &mut (impl ConnectionLike + AsyncCommands),
    ) {
        if let Some(caps) = REGEX_SESSION.captures(channel) {
            let session_id = &caps["sid"];

            match operation {
                // Node has become alive
                "expire" => {
                    let data_key = format!("session:{}:upstream", session_id);
                    let res = con
                        .hget::<_, _, (String, String)>(data_key, &["host", "port"])
                        .await;

                    if let Ok((host, port)) = res {
                        if info
                            .add_session_upstream(session_id.to_string(), &host, &port)
                            .await
                            .is_none()
                        {
                            info!("+ Session {} @ {}:{}", session_id, host, port);
                        }
                    }
                }
                // Node has died
                "expired" => {
                    info!("- Session {}", session_id);
                    info.remove_session_upstream(session_id).await;
                }
                &_ => {}
            }
        }
    }
}
