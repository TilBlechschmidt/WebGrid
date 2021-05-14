use crate::{
    libraries::{
        helpers::keys,
        net::messaging::Message,
        resources::{PubSub, ResourceManager, ResourceManagerProvider},
    },
    with_redis_resource,
};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use futures::TryStreamExt;
use jatsl::{Job, TaskManager};
use lru::LruCache;
use rand::{
    prelude::{IteratorRandom, ThreadRng},
    thread_rng,
};
use redis::{aio::ConnectionLike, AsyncCommands, Msg};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, marker::PhantomData, sync::Arc};
use thiserror::Error;
use tokio::{
    pin, select,
    sync::{
        broadcast::{self, error::RecvError},
        Mutex,
    },
    task::yield_now,
    time::{sleep_until, Duration, Instant},
};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone)]
pub enum ServiceDescriptor {
    Api,
    Manager,
    Node(Uuid),
    Storage(Uuid),
}

impl ServiceDescriptor {
    pub fn discovery_channel(&self) -> String {
        match *self {
            Self::Api => keys::discovery("api", None),
            Self::Manager => keys::discovery("manager", None),
            Self::Node(id) => keys::discovery("node", Some(id)),
            Self::Storage(id) => keys::discovery("storage", Some(id)),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServiceDiscoveryResponse {
    pub service: ServiceDescriptor,
    pub endpoint: String,
}

impl ServiceDiscoveryResponse {
    pub fn new(service: ServiceDescriptor, endpoint: String) -> Self {
        Self { service, endpoint }
    }
}

type ServiceEndpointCache = Arc<Mutex<LruCache<ServiceDescriptor, HashSet<String>>>>;

pub struct ServiceDiscovery {
    publisher: broadcast::Sender<ServiceDiscoveryResponse>,
    cache: ServiceEndpointCache,
}

impl ServiceDiscovery {
    pub fn new<C, R>(
        channel_capacity: usize,
        cache_capacity: usize,
    ) -> (Self, ServiceDiscoveryJob<C, R>) {
        let (publisher, _) = broadcast::channel(channel_capacity);
        let cache = Arc::new(Mutex::new(LruCache::new(cache_capacity)));

        (
            Self {
                publisher: publisher.clone(),
                cache: cache.clone(),
            },
            ServiceDiscoveryJob::new(publisher, cache),
        )
    }

    pub fn start_discovery(
        &self,
        service: ServiceDescriptor,
        max_retries: u8,
    ) -> ServiceDiscoverer {
        ServiceDiscoverer::new(
            self.publisher.subscribe(),
            self.cache.clone(),
            service,
            max_retries,
        )
    }
}

pub struct ServiceDiscoverer {
    subscriber: broadcast::Receiver<ServiceDiscoveryResponse>,
    cache: ServiceEndpointCache,
    service: ServiceDescriptor,

    rng: ThreadRng,
    retries: u8,
    max_retries: u8,
}

#[derive(Error, Debug)]
pub enum ServiceDiscoveryError {
    #[error("maximum number of discovery retries exceeded")]
    RetriesExceeded,
    #[error("discovery service channel disconnected")]
    Disconnect(#[from] RecvError),
    #[error("failed to send discovery request")]
    SendFailure(#[from] redis::RedisError),
    #[error("timed out waiting for discovery")]
    Timeout,
    #[error("serialization failed")]
    SerdeFailure(#[from] bincode::Error),
}

impl ServiceDiscoverer {
    fn new(
        subscriber: broadcast::Receiver<ServiceDiscoveryResponse>,
        cache: ServiceEndpointCache,
        service: ServiceDescriptor,
        max_retries: u8,
    ) -> Self {
        Self {
            subscriber,
            cache,
            service,
            rng: thread_rng(),
            retries: 0,
            max_retries,
        }
    }

    /// Flags a given endpoint as stale, purging it from the cache
    pub async fn flag_stale(&self, endpoint: &str) {
        if let Some(endpoints) = self.cache.lock().await.get_mut(&self.service) {
            endpoints.remove(endpoint);
        }
    }

    /// Attempts to discover an endpoint
    ///
    /// Starts by looking at the cache, if that fails it sends out a discovery request.
    /// If no response is received within a certain timeframe, the process repeats.
    /// When the cache is empty and multiple active discovery attempts have been made, an error is returned.
    ///
    /// Note: This function relies on the callee making use of the `flag_stale` function to
    ///       mark endpoints that are non-functional. Not calling it will result in a poisoned cache
    ///       and the same broken endpoints being returned over and over again.
    pub async fn discover(
        &mut self,
        con: &mut (impl ConnectionLike + AsyncCommands),
    ) -> Result<String, ServiceDiscoveryError> {
        loop {
            // Bail if the maximum number of discoveries has been reached
            if self.retries > self.max_retries {
                return Err(ServiceDiscoveryError::RetriesExceeded);
            }

            // Try discovering a new endpoint, retry when we hit a timeout
            // (but increase the number of retries to set an upper limit)
            match self.discover_once(con).await {
                Ok(endpoint) => return Ok(endpoint),
                Err(ServiceDiscoveryError::Timeout) => self.retries += 1,
                Err(e) => return Err(e),
            }
        }
    }

    async fn discover_once(
        &mut self,
        con: &mut (impl ConnectionLike + AsyncCommands),
    ) -> Result<String, ServiceDiscoveryError> {
        // Try fetching a random element from cache
        if let Some(endpoints) = self.cache.lock().await.get(&self.service) {
            if let Some(endpoint) = endpoints.iter().choose(&mut self.rng) {
                return Ok(endpoint.clone());
            }
        }

        // On cache miss, send out a discovery request
        let raw_request: Vec<u8> = bincode::serialize(&Message::ServiceDiscoveryRequest)?;
        con.publish::<_, _, ()>(self.service.discovery_channel(), raw_request)
            .await?;

        // Wait for a response, but not forever
        let deadline = Instant::now() + Duration::from_millis(500);

        loop {
            let message_future = self.subscriber.recv();
            pin!(message_future);

            let discovery_response: Option<Result<ServiceDiscoveryResponse, RecvError>> = select! {
                discovery_response = message_future => Some(discovery_response),
                _ = sleep_until(deadline) => None,
            };

            match discovery_response {
                None => return Err(ServiceDiscoveryError::Timeout),
                Some(Err(e)) => return Err(e.into()),
                Some(Ok(discovery_response)) => {
                    if discovery_response.service == self.service {
                        return Ok(discovery_response.endpoint);
                    }
                }
            }
        }
    }
}

pub struct ServiceDiscoveryJob<C, R> {
    cache: ServiceEndpointCache,
    publisher: broadcast::Sender<ServiceDiscoveryResponse>,
    phantom_c: PhantomData<C>,
    phantom_r: PhantomData<R>,
}

#[derive(Error, Debug)]
enum ServiceDiscoveryJobError {
    #[error("redis notification stream ended unexpectedly")]
    UnexpectedTermination,
}

#[async_trait]
impl<R: ResourceManager + Send + Sync, C: ResourceManagerProvider<R> + Send + Sync> Job
    for ServiceDiscoveryJob<R, C>
{
    type Context = C;

    const NAME: &'static str = module_path!();

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        let mut pubsub: PubSub = with_redis_resource!(manager).into();

        pubsub
            .subscribe(&(*keys::DISCOVERY))
            .await
            .context("unable to subscribe to service discovery channel")?;

        manager.ready().await;

        let mut stream = pubsub.on_message();

        while let Ok(Some(msg)) = stream.try_next().await {
            self.process_message(msg).await?;
        }

        // Allow the job manager to terminate us so it doesn't count as a crash
        yield_now().await;

        bail!(ServiceDiscoveryJobError::UnexpectedTermination)
    }
}

impl<C, R> ServiceDiscoveryJob<C, R> {
    fn new(
        publisher: broadcast::Sender<ServiceDiscoveryResponse>,
        cache: ServiceEndpointCache,
    ) -> Self {
        Self {
            publisher,
            cache,
            phantom_c: PhantomData,
            phantom_r: PhantomData,
        }
    }

    async fn process_message(&self, msg: Msg) -> Result<()> {
        // Deserialize the response
        let raw_discovery_response: &[u8] = msg.get_payload_bytes();
        let discovery_response: ServiceDiscoveryResponse =
            bincode::deserialize(&raw_discovery_response)
                .context("encountered unexpected message data in discovery channel")?;

        // Insert the value into the cache
        let mut cache = self.cache.lock().await;
        if let Some(entry) = cache.get_mut(&discovery_response.service) {
            entry.insert(discovery_response.endpoint.clone());
        } else {
            let mut set = HashSet::new();
            set.insert(discovery_response.endpoint.clone());
            cache.put(discovery_response.service.clone(), set);
        }

        // Notify anybody waiting for the response
        self.publisher.send(discovery_response)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        libraries::{
            net::advertise::ServiceAdvertisorJob, resources::ResourceManager,
            testing::resources::TestResourceManager,
        },
        with_resource_manager,
    };
    use jatsl::{JobScheduler, TaskResourceHandle};

    // TODO Running tests in parallel needs some clever engineering. Redis PubSub is GLOBAL and does not honor SELECT :(
    //      The channels used for the tests need a prefix like unique_identifier!(). Maybe integrate that into
    //      the harness somehow?

    #[test]
    fn discovery_without_endpoint_fails() {
        with_resource_manager!(manager, {
            let mut redis = manager.redis(TaskResourceHandle::stub()).await.unwrap();
            let (discovery, _) =
                ServiceDiscovery::new::<TestResourceManager, TestResourceManager>(1, 1);
            let service = ServiceDescriptor::Api;

            let mut discoverer = discovery.start_discovery(service.clone(), 0);
            let endpoint = discoverer.discover(&mut redis).await;

            match endpoint {
                Err(ServiceDiscoveryError::RetriesExceeded) => {}
                e => panic!("Unexpected condition when discovering endpoint: {:?}", e),
            }
        });
    }

    // #[test]
    // fn passive_discovery() {
    //     with_resource_manager!(manager, {
    //         let mut redis = manager.redis(TaskResourceHandle::stub()).await.unwrap();
    //         let (discovery, job) = ServiceDiscovery::new(10, 10);

    //         let scheduler = JobScheduler::default();
    //         scheduler.spawn_job(job, manager);

    //         let service = ServiceDescriptor::Api;
    //         let endpoint = "example.com".to_string();
    //         let payload = bincode::serialize(&ServiceDiscoveryResponse::new(
    //             service.clone(),
    //             endpoint.clone(),
    //         ))
    //         .unwrap();

    //         // Give the job time to start up
    //         scheduler.wait_for_ready().await;

    //         redis
    //             .publish::<_, _, ()>(&(*keys::DISCOVERY), payload)
    //             .await
    //             .unwrap();

    //         let mut discoverer = discovery.start_discovery(service, 0);
    //         let discovered_endpoint = discoverer.discover(&mut redis).await.unwrap();

    //         assert_eq!(endpoint, discovered_endpoint);
    //     });
    // }

    #[test]
    fn active_discovery() {
        with_resource_manager!(manager, {
            let service = ServiceDescriptor::Api;
            let endpoint = "example.com".to_string();

            let mut redis = manager.redis(TaskResourceHandle::stub()).await.unwrap();
            let (discovery, job) = ServiceDiscovery::new(10, 10);

            let advertise_job = ServiceAdvertisorJob::new(service.clone(), endpoint.clone());

            let scheduler = JobScheduler::default();
            scheduler.spawn_job(job, manager.clone());
            scheduler.spawn_job(advertise_job, manager);

            // Give the job time to start up
            scheduler.wait_for_ready().await;

            let mut discoverer = discovery.start_discovery(service, 0);
            let discovered_endpoint = discoverer.discover(&mut redis).await.unwrap();

            assert_eq!(endpoint, discovered_endpoint);
        });
    }
}
