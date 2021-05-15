use crate::{
    libraries::{
        helpers::keys,
        net::messaging::Message,
        resources::{PubSub, ResourceManager, ResourceManagerProvider},
        tracing::global_tracer,
    },
    with_redis_resource, with_shared_redis_resource,
};
use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use futures::TryStreamExt;
use jatsl::{Job, TaskManager};
use lru::LruCache;
use opentelemetry::trace::{FutureExt, Span, StatusCode, TraceContextExt, Tracer};
use opentelemetry::Context as TelemetryContext;
use rand::{prelude::IteratorRandom, thread_rng};
use redis::{aio::ConnectionLike, AsyncCommands, Msg};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fmt, marker::PhantomData, sync::Arc};
use thiserror::Error;
use tokio::{
    pin, select,
    sync::{
        broadcast::{self, error::RecvError},
        mpsc, Mutex,
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

impl fmt::Display for ServiceDescriptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
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

#[derive(Clone)]
pub struct ServiceDiscovery {
    publisher: broadcast::Sender<ServiceDiscoveryResponse>,
    request_publisher: mpsc::Sender<ServiceDescriptor>,
    cache: ServiceEndpointCache,
}

impl ServiceDiscovery {
    pub fn new<C, R>(
        channel_capacity: usize,
        cache_capacity: usize,
    ) -> (Self, ServiceDiscoveryJob<C, R>) {
        let (publisher, _) = broadcast::channel(channel_capacity);
        let (request_publisher, request_receiver) = mpsc::channel(channel_capacity);
        let cache = Arc::new(Mutex::new(LruCache::new(cache_capacity)));

        (
            Self {
                request_publisher,
                publisher: publisher.clone(),
                cache: cache.clone(),
            },
            ServiceDiscoveryJob::new(publisher, request_receiver, cache),
        )
    }

    pub fn start_discovery(
        &self,
        service: ServiceDescriptor,
        max_retries: u8,
    ) -> ServiceDiscoverer {
        ServiceDiscoverer::new(
            self.publisher.subscribe(),
            self.request_publisher.clone(),
            self.cache.clone(),
            service,
            max_retries,
        )
    }
}

pub struct ServiceDiscoverer {
    subscriber: broadcast::Receiver<ServiceDiscoveryResponse>,
    request_publisher: mpsc::Sender<ServiceDescriptor>,
    cache: ServiceEndpointCache,
    service: ServiceDescriptor,

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
    #[error("unable to send request")]
    RequestFailed(#[from] mpsc::error::SendError<ServiceDescriptor>),
}

impl ServiceDiscoverer {
    fn new(
        subscriber: broadcast::Receiver<ServiceDiscoveryResponse>,
        request_publisher: mpsc::Sender<ServiceDescriptor>,
        cache: ServiceEndpointCache,
        service: ServiceDescriptor,
        max_retries: u8,
    ) -> Self {
        Self {
            subscriber,
            request_publisher,
            cache,
            service,
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
    pub async fn discover(&mut self) -> Result<String, ServiceDiscoveryError> {
        let span = global_tracer().start("Discover service");
        let context = TelemetryContext::current_with_span(span);

        loop {
            // Bail if the maximum number of discoveries has been reached
            if self.retries > self.max_retries {
                context
                    .span()
                    .set_status(StatusCode::Error, "Retry limit exceeded".to_string());
                return Err(ServiceDiscoveryError::RetriesExceeded);
            }

            // Try discovering a new endpoint, retry when we hit a timeout
            // (but increase the number of retries to set an upper limit)
            match self.discover_once().with_context(context.clone()).await {
                Ok(endpoint) => return Ok(endpoint),
                Err(ServiceDiscoveryError::Timeout) => self.retries += 1,
                Err(e) => {
                    context.span().set_status(StatusCode::Error, e.to_string());
                    return Err(e);
                }
            }
        }
    }

    async fn discover_once(&mut self) -> Result<String, ServiceDiscoveryError> {
        let mut span = global_tracer().start("Attempt discovery");

        // Try fetching a random element from cache
        if let Some(endpoints) = self.cache.lock().await.get(&self.service) {
            let mut rng = thread_rng();
            if let Some(endpoint) = endpoints.iter().choose(&mut rng) {
                span.set_status(StatusCode::Ok, "Cache hit".to_string());
                return Ok(endpoint.clone());
            }
        }

        // On cache miss, send out a discovery request
        span.add_event("Cache miss".to_string(), vec![]);
        self.request_publisher.send(self.service.clone()).await?;

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
    request_rx: Arc<Mutex<mpsc::Receiver<ServiceDescriptor>>>,
    publisher: broadcast::Sender<ServiceDiscoveryResponse>,
    phantom_c: PhantomData<C>,
    phantom_r: PhantomData<R>,
}

#[derive(Error, Debug)]
enum ServiceDiscoveryJobError {
    #[error("redis notification stream ended unexpectedly")]
    UnexpectedTermination,
    #[error("all references to request senders have been dropped")]
    RequestSendersDeallocated,
}

#[async_trait]
impl<R: ResourceManager + Send + Sync, C: ResourceManagerProvider<R> + Send + Sync> Job
    for ServiceDiscoveryJob<R, C>
{
    type Context = C;

    const NAME: &'static str = module_path!();

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        let mut con = with_shared_redis_resource!(manager);
        let mut pubsub: PubSub = with_redis_resource!(manager).into();

        pubsub
            .subscribe(&(*keys::DISCOVERY))
            .await
            .context("unable to subscribe to service discovery channel")?;

        manager.ready().await;

        let mut stream = pubsub.on_message();

        loop {
            let mut request_rx = self.request_rx.lock().await;

            select! {
                msg = stream.try_next() => {
                    match msg {
                        Ok(Some(msg)) => self.process_message(msg).await?,
                        Ok(None) => break,
                        Err(e) => return Err(e.into()),
                    }
                }
                discovery_request = request_rx.recv() => {
                    match discovery_request {
                        Some(service) => self.process_request(service, &mut con).await?,
                        None => bail!(ServiceDiscoveryJobError::RequestSendersDeallocated)
                    }
                }
            }
        }

        // Allow the job manager to terminate us so it doesn't count as a crash
        yield_now().await;

        bail!(ServiceDiscoveryJobError::UnexpectedTermination)
    }
}

impl<C, R> ServiceDiscoveryJob<C, R> {
    fn new(
        publisher: broadcast::Sender<ServiceDiscoveryResponse>,
        request_rx: mpsc::Receiver<ServiceDescriptor>,
        cache: ServiceEndpointCache,
    ) -> Self {
        Self {
            publisher,
            request_rx: Arc::new(Mutex::new(request_rx)),
            cache,
            phantom_c: PhantomData,
            phantom_r: PhantomData,
        }
    }

    async fn process_request(
        &self,
        service: ServiceDescriptor,
        con: &mut (impl ConnectionLike + AsyncCommands),
    ) -> Result<()> {
        let raw_request: Vec<u8> = bincode::serialize(&Message::ServiceDiscoveryRequest)?;
        con.publish::<_, _, ()>(service.discovery_channel(), raw_request)
            .await
            .context("sending request to discovery channel")?;

        Ok(())
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
        self.publisher.send(discovery_response).ok();

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{libraries::net::advertise::ServiceAdvertisorJob, with_resource_manager};
    use jatsl::JobScheduler;

    // TODO Running tests in parallel needs some clever engineering. Redis PubSub is GLOBAL and does not honor SELECT :(
    //      The channels used for the tests need a prefix like unique_identifier!(). Maybe integrate that into
    //      the harness somehow?

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
            let (discovery, job) = ServiceDiscovery::new(10, 10);

            let advertise_job = ServiceAdvertisorJob::new(service.clone(), endpoint.clone());

            let scheduler = JobScheduler::default();
            scheduler.spawn_job(job, manager.clone());
            scheduler.spawn_job(advertise_job, manager);

            // Give the job time to start up
            scheduler.wait_for_ready().await;

            let mut discoverer = discovery.start_discovery(service, 0);
            let discovered_endpoint = discoverer.discover().await.unwrap();

            assert_eq!(endpoint, discovered_endpoint);
        });
    }
}
