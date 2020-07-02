use crate::Context;
use anyhow::{bail, Result};
use async_trait::async_trait;
use futures::stream::TryStreamExt;
use lazy_static::lazy_static;
use log::info;
use regex::Regex;
use resources::{with_redis_resource, PubSub, ResourceManager};
use scheduling::{Job, TaskManager};
use tokio::task::yield_now;

static PATTERN_SESSION: &str = "__keyspace@0__:session:*:heartbeat.node";

lazy_static! {
    static ref REGEX_SESSION: Regex =
        Regex::new(r"__keyspace@0__:session:(?P<sid>[^:]+):heartbeat\.node").unwrap();
}

#[derive(Clone)]
pub struct NodeWatcherJob {}

#[async_trait]
impl Job for NodeWatcherJob {
    type Context = Context;

    const NAME: &'static str = module_path!();

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        let mut pubsub: PubSub = with_redis_resource!(manager).into();
        pubsub.psubscribe(PATTERN_SESSION).await?;

        manager.ready().await;

        let mut stream = pubsub.on_message();

        while let Ok(Some(msg)) = stream.try_next().await {
            let channel: &str = msg.get_channel_name();
            let operation: String = msg.get_payload()?;

            if operation != "expired" {
                continue;
            }

            if let Some(caps) = REGEX_SESSION.captures(channel) {
                let session_id = &caps["sid"];
                info!("Cleaning up dead node {}", session_id);
                manager.context.provisioner.terminate_node(session_id).await;
            }
        }

        // Allow the job manager to terminate us so it doesn't count as a crash
        yield_now().await;

        bail!("redis notification stream ended unexpectedly")
    }
}

impl NodeWatcherJob {
    pub fn new() -> Self {
        Self {}
    }
}
