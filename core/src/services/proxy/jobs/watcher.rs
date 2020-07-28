use super::super::{routing_info::RoutingInfo, Context};
use crate::libraries::resources::{PubSub, ResourceManager};
use crate::libraries::scheduling::{Job, TaskManager};
use crate::with_redis_resource;
use anyhow::{bail, Context as AnyhowContext, Result};
use async_trait::async_trait;
use futures::stream::TryStreamExt;
use lazy_static::lazy_static;
use log::{error, info};
use redis::{aio::ConnectionLike, cmd, AsyncCommands, Msg, RedisError};
use regex::Regex;
use thiserror::Error;
use tokio::task::yield_now;

const PATTERN_MANAGER: &str = "__keyspace@0__:manager:*:heartbeat";
const PATTERN_SESSION: &str = "__keyspace@0__:session:*:heartbeat.node";
const PATTERN_STORAGE: &str = "__keyspace@0__:storage:*:*:host";
const PATTERN_API: &str = "__keyspace@0__:api:*:host";

lazy_static! {
    static ref REGEX_MANAGER: Regex =
        Regex::new(r"__keyspace@0__:manager:(?P<mid>[^:]+):heartbeat").unwrap();
    static ref REGEX_SESSION: Regex =
        Regex::new(r"__keyspace@0__:session:(?P<sid>[^:]+):heartbeat\.node").unwrap();
    static ref REGEX_STORAGE: Regex =
        Regex::new(r"__keyspace@0__:storage:(?P<sid>[^:]+):(?P<pid>[^:]+):host").unwrap();
    static ref REGEX_API: Regex = Regex::new(r"__keyspace@0__:api:(?P<aid>[^:]+):host").unwrap();
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

        pubsub
            .psubscribe(PATTERN_STORAGE)
            .await
            .context("unable to subscribe to storage list")?;

        pubsub
            .psubscribe(PATTERN_API)
            .await
            .context("unable to subscribe to api list")?;

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
        } else if pattern == Some(PATTERN_STORAGE.to_string()) {
            self.handle_storage_message(channel, &operation, &info, con)
                .await;
        } else if pattern == Some(PATTERN_API.to_string()) {
            self.handle_api_message(channel, &operation, &info, con)
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

    async fn handle_storage_message(
        &self,
        channel: &str,
        operation: &str,
        info: &RoutingInfo,
        con: &mut (impl ConnectionLike + AsyncCommands),
    ) {
        if let Some(caps) = REGEX_STORAGE.captures(channel) {
            let storage_id = &caps["sid"];
            let provider_id = &caps["pid"];

            match operation {
                "expire" => {
                    let data_key = format!("storage:{}:{}:host", storage_id, provider_id);

                    if let Ok(host) = con.get::<_, String>(data_key).await {
                        if info
                            .add_storage_upstream(storage_id, provider_id, &host)
                            .await
                            .is_none()
                        {
                            info!(
                                "+ Storage {} @ {} (provider: {})",
                                storage_id, host, provider_id
                            );
                        }
                    }
                }
                "expired" => {
                    info!("- Storage {} (provider: {})", storage_id, provider_id);
                    info.remove_storage_upstream(storage_id, provider_id).await;
                }
                &_ => {}
            }
        }
    }

    async fn handle_api_message(
        &self,
        channel: &str,
        operation: &str,
        info: &RoutingInfo,
        con: &mut (impl ConnectionLike + AsyncCommands),
    ) {
        if let Some(caps) = REGEX_API.captures(channel) {
            let api_id = &caps["aid"];

            match operation {
                // Manager has been added
                "expire" => {
                    let data_key = format!("api:{}:host", api_id);
                    let res = con.get::<_, String>(data_key).await;

                    if let Ok(addr) = res {
                        if info.add_api_upstream(api_id, &addr).await.is_none() {
                            info!("+ API {} @ {}", api_id, addr);
                        }
                    }
                }
                // Manager has died
                "expired" => {
                    info!("- API {}", api_id);
                    info.remove_api_upstream(api_id).await;
                }
                &_ => {}
            }
        }
    }
}
