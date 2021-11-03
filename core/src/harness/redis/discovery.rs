use super::factory::{MonitoredRedisFactory, RedisCommunicationFactory};
use crate::library::communication::discovery::pubsub::PubSubServiceDiscovererDaemon;
use crate::library::communication::discovery::{
    ServiceAdvertiser, ServiceDescriptor, ServiceEndpoint,
};
use crate::library::communication::implementation::redis::RedisPubSubBackend;
use crate::library::communication::CommunicationFactory;
use crate::library::EmptyResult;
use async_trait::async_trait;
use jatsl::Job;
use serde::{de::DeserializeOwned, Serialize};
use std::hash::Hash;
use std::sync::Arc;

/// Job which runs a [`PubSubServiceDiscovererDaemon`] on a [`RedisPubSubBackend`]
pub struct RedisServiceDiscoveryJob<D: ServiceDescriptor> {
    url: String,
    daemon: PubSubServiceDiscovererDaemon<D>,
}

impl<D: ServiceDescriptor> RedisServiceDiscoveryJob<D> {
    /// Creates a new instance from an existing daemon instance
    pub fn new(url: String, daemon: PubSubServiceDiscovererDaemon<D>) -> Self {
        Self { url, daemon }
    }
}

#[async_trait]
impl<D> Job for RedisServiceDiscoveryJob<D>
where
    D: ServiceDescriptor + Send + Sync + Eq + Hash + Serialize + DeserializeOwned + std::fmt::Debug,
{
    const NAME: &'static str = concat!(module_path!(), "::discovery");

    async fn execute(&self, manager: jatsl::JobManager) -> EmptyResult {
        let manager = Arc::new(manager);
        let factory = MonitoredRedisFactory::new(self.url.clone(), manager.clone());
        let backend = RedisPubSubBackend::new(factory).await?;

        manager.ready().await;

        self.daemon.daemon_loop(backend).await;

        Ok(())
    }
}

/// Job which advertises a given service using the redis [`ServiceAdvertiser`] implementation
pub struct RedisServiceAdvertisementJob<D: ServiceDescriptor> {
    url: String,
    service: D,
    endpoint: ServiceEndpoint,
}

impl<D: ServiceDescriptor> RedisServiceAdvertisementJob<D> {
    /// Creates a new instance for a given service and endpoint
    pub fn new(url: String, service: D, endpoint: ServiceEndpoint) -> Self {
        Self {
            url,
            service,
            endpoint,
        }
    }
}

#[async_trait]
impl<D> Job for RedisServiceAdvertisementJob<D>
where
    D: ServiceDescriptor + Send + Sync + Eq + Hash + Serialize + DeserializeOwned,
{
    const NAME: &'static str = module_path!();

    async fn execute(&self, manager: jatsl::JobManager) -> EmptyResult {
        let manager = Arc::new(manager);
        let factory = RedisCommunicationFactory::new(self.url.clone(), manager.clone());
        let advertiser = factory.service_advertiser();

        advertiser
            .advertise(
                self.service.clone(),
                self.endpoint.clone(),
                Some(|| manager.ready()),
            )
            .await?;

        Ok(())
    }
}
