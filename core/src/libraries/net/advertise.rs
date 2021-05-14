use super::discovery::{ServiceDescriptor, ServiceDiscoveryResponse};
use crate::{
    libraries::{
        helpers::keys,
        net::messaging::Message,
        resources::{PubSub, ResourceManager, ResourceManagerProvider},
    },
    with_redis_resource, with_shared_redis_resource,
};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use futures::TryStreamExt;
use jatsl::{Job, TaskManager};
use redis::AsyncCommands;
use std::marker::PhantomData;
use tokio::task::yield_now;

pub struct ServiceAdvertisorJob<R, C> {
    response: ServiceDiscoveryResponse,
    phantom_r: PhantomData<R>,
    phantom_c: PhantomData<C>,
}

impl<R, C> ServiceAdvertisorJob<R, C> {
    pub fn new(service: ServiceDescriptor, endpoint: String) -> Self {
        Self {
            response: ServiceDiscoveryResponse::new(service, endpoint),
            phantom_r: PhantomData,
            phantom_c: PhantomData,
        }
    }
}

#[async_trait]
impl<R: ResourceManager + Send + Sync, C: ResourceManagerProvider<R> + Send + Sync> Job
    for ServiceAdvertisorJob<R, C>
{
    type Context = C;

    const NAME: &'static str = module_path!();

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        let mut con = with_shared_redis_resource!(manager);
        let mut pubsub: PubSub = with_redis_resource!(manager).into();

        pubsub
            .subscribe(&self.response.service.discovery_channel())
            .await
            .context("unable to subscribe to service discovery channel")?;

        manager.ready().await;

        let mut stream = pubsub.on_message();

        while let Ok(Some(msg)) = stream.try_next().await {
            let raw_discovery_request: &[u8] = msg.get_payload_bytes();
            let discovery_request: Message = bincode::deserialize(&raw_discovery_request)
                .context("encountered unexpected message data in discovery channel")?;

            if let Message::ServiceDiscoveryRequest = discovery_request {
                let raw_response = bincode::serialize(&self.response)?;
                con.publish::<_, _, ()>(&(*keys::DISCOVERY), raw_response)
                    .await?;
            }
        }

        // Allow the job manager to terminate us so it doesn't count as a crash
        yield_now().await;

        bail!("Service advertising stream unexpectedly crash")
    }
}
