// TODO This file is a hot mess. The code is okay but it just contains too much at once!
use crate::library::communication::discovery::pubsub::PubSubServiceDiscovererDaemon;
use crate::library::communication::discovery::{
    ServiceAdvertiser, ServiceDescriptor, ServiceEndpoint,
};
use crate::library::communication::implementation::redis::{
    PubSubResource, PubSubResourceError, RedisConnectionVariant, RedisFactory, RedisPubSubBackend,
    RedisPublisher, RedisQueueProvider, RedisResponseCollector, RedisServiceAdvertiser,
};
use crate::library::communication::request::CompositeRequestor;
use crate::library::communication::CommunicationFactory;
use crate::library::{BoxedError, EmptyResult};
use async_trait::async_trait;
use futures::stream::{once, BoxStream};
use futures::StreamExt;
use futures::{
    future::{BoxFuture, FutureExt, Shared},
    lock::{Mutex, MutexGuard},
};
use jatsl::{Job, TaskManager, TaskResourceHandle};
use lazy_static::lazy_static;
use log::{debug, trace, warn};
use redis::aio::PubSub;
use redis::Msg;
use redis::{
    aio::{Connection, ConnectionLike, MultiplexedConnection},
    Client, Cmd, Pipeline, RedisError, RedisFuture, RedisResult, Value,
};
use serde::de::DeserializeOwned;
use serde::Serialize;

use std::hash::Hash;
use std::{
    collections::HashSet,
    ops::{Deref, DerefMut},
    sync::Arc,
    time::Duration,
};
use tokio::task;
use tokio::task::yield_now;
use tokio::time::{sleep, timeout};

type SharedMultiplexedConnectionFuture = Shared<BoxFuture<'static, MultiplexedConnection>>;

lazy_static! {
    static ref SHARED_CONNECTION: Mutex<Option<Arc<SharedMultiplexedConnectionFuture>>> =
        Mutex::new(None);
    static ref SHARED_TASK_RESOURCE_HANDLES: Mutex<HashSet<TaskResourceHandle>> =
        Mutex::new(HashSet::new());
}

#[derive(Clone)]
struct HandleRegistration {
    tx: TaskResourceHandle,
    is_shared: bool,
}

impl DerefMut for HandleRegistration {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.tx
    }
}

impl Deref for HandleRegistration {
    type Target = TaskResourceHandle;

    fn deref(&self) -> &Self::Target {
        &self.tx
    }
}

impl Drop for HandleRegistration {
    fn drop(&mut self) {
        if self.is_shared {
            // If this was a shared resource lazily remove the resource handle from list of active handles
            let handle = self.tx.clone();
            task::spawn(async {
                let handle = handle;
                SHARED_TASK_RESOURCE_HANDLES.lock().await.remove(&handle);
            });
        }
    }
}

/// Redis connection that monitors for connection errors
pub struct RedisResource<C: ConnectionLike> {
    con: C,
    handle: HandleRegistration,
    logging_enabled: bool,
}

impl RedisResource<MultiplexedConnection> {
    /// Retrieves a shared redis instance or instantiates it if it doesn't exist
    pub async fn shared(handle: TaskResourceHandle, url: &str) -> RedisResult<Self> {
        let client = Client::open(url)?;

        let shared_con_lock = SHARED_CONNECTION.lock().await;

        let future = match &(*shared_con_lock) {
            Some(container_future) => {
                trace!("Reusing existing shared connection!");
                container_future.clone()
            }
            None => RedisResource::load_new_shared_handle(client, shared_con_lock),
        };

        let con = (*future).clone().await;

        SHARED_TASK_RESOURCE_HANDLES
            .lock()
            .await
            .insert(handle.clone());

        Ok(Self {
            con,
            handle: HandleRegistration {
                is_shared: true,
                tx: handle,
            },
            logging_enabled: false,
        })
    }

    fn load_new_shared_handle(
        client: Client,
        mut shared_con_lock: MutexGuard<Option<Arc<SharedMultiplexedConnectionFuture>>>,
    ) -> Arc<SharedMultiplexedConnectionFuture> {
        let future = RedisResource::connect_shared(client).boxed().shared();
        let arc_future = Arc::new(future);
        *shared_con_lock = Some(arc_future.clone());

        arc_future
    }

    async fn connect_shared(client: Client) -> MultiplexedConnection {
        let retry_interval = Duration::from_secs(2);
        let request_timeout = Duration::from_secs(4);
        let mut warn = true;

        loop {
            let con_future = client.get_multiplexed_tokio_connection();
            let timed_con_future = timeout(request_timeout, con_future);

            match timed_con_future.await {
                Ok(con_result) => match con_result {
                    Ok(connection) => return connection,
                    Err(e) => {
                        if warn {
                            warn = false;
                            warn!("Unable to connect to redis server! ({})", e)
                        }
                    }
                },
                Err(e) => {
                    if warn {
                        warn = false;
                        warn!("Timed out while connecting to redis! ({})", e)
                    }
                }
            }

            sleep(retry_interval).await;
        }
    }
}

impl RedisResource<Connection> {
    /// Creates a new standalone redis connection
    pub async fn new(handle: TaskResourceHandle, url: &str) -> RedisResult<Self> {
        let client = Client::open(url)?;
        let con = RedisResource::connect_standalone(client).await;

        Ok(Self {
            con,
            handle: HandleRegistration {
                is_shared: false,
                tx: handle,
            },
            logging_enabled: false,
        })
    }

    async fn connect_standalone(client: Client) -> Connection {
        let retry_interval = Duration::from_secs(2);
        let request_timeout = Duration::from_secs(4);
        let mut warn = true;

        loop {
            let con_future = client.get_async_connection();
            let timed_con_future = timeout(request_timeout, con_future);

            match timed_con_future.await {
                Ok(con_result) => match con_result {
                    Ok(connection) => return connection,
                    Err(e) => {
                        if warn {
                            warn = false;
                            warn!("Unable to connect to redis server! ({})", e)
                        }
                    }
                },
                Err(e) => {
                    if warn {
                        warn = false;
                        warn!("Timed out while connecting to redis! ({})", e)
                    }
                }
            }

            sleep(retry_interval).await;
        }
    }
}

impl<C: ConnectionLike> RedisResource<C> {
    /// Enables request logging
    pub fn set_logging(&mut self, enabled: bool) {
        self.logging_enabled = enabled;
    }

    /// Set the redis database index
    pub async fn select(&mut self, db: usize) -> RedisResult<()> {
        Ok(redis::cmd("SELECT")
            .arg(db)
            .query_async(&mut self.con)
            .await?)
    }

    async fn notify(&mut self, error: &RedisError) {
        debug!("{:?}", error);
        self.handle.resource_died().await;

        if self.handle.is_shared {
            // Invalidate the shared connection
            *(SHARED_CONNECTION.lock().await) = None;

            // Notify all other task's handles that are using the shared connection
            let handles = SHARED_TASK_RESOURCE_HANDLES.lock().await;
            debug!("Calling {} shared termination handles", handles.len());
            for handle in handles.iter() {
                handle.clone().resource_died().await;
            }
        }

        yield_now().await;
    }

    fn log_cmd(&self, cmd: &Cmd) {
        let packed_command: Vec<u8> = cmd.get_packed_command();
        self.print_packed_command(packed_command);
    }

    fn log_pipeline(&self, pipeline: &Pipeline) {
        let packed_command: Vec<u8> = pipeline.get_packed_pipeline();
        self.print_packed_command(packed_command);
    }

    fn print_packed_command(&self, cmd: Vec<u8>) {
        let input = String::from_utf8_lossy(&cmd);

        let mut chars = input.chars().peekable();
        let mut mode = CommandParseMode::ArgumentCount;

        print!("REDIS -> ");

        while let Some(char) = chars.next() {
            // Advance to the next line
            if let Some(next_char) = chars.peek() {
                if char == '\r' && *next_char == '\n' {
                    chars.next(); // skip the \n
                    mode = match mode {
                        CommandParseMode::ArgumentCount => CommandParseMode::ArgumentSize,
                        CommandParseMode::ArgumentSize => CommandParseMode::Argument,
                        CommandParseMode::Argument => {
                            print!(" ");
                            CommandParseMode::ArgumentSize
                        }
                    };
                    continue;
                }
            }

            // Print argument content
            if mode == CommandParseMode::Argument {
                print!("{}", char);
            }
        }

        println!();
    }
}

#[derive(Eq, PartialEq)]
enum CommandParseMode {
    ArgumentCount,
    ArgumentSize,
    Argument,
}

/// Handle a redis command result.
macro_rules! notify_if_disconnected {
    ($self:expr, $result:expr) => {
        if let Err(ref e) = $result {
            if e.is_connection_dropped()
                || e.is_io_error()
                || e.is_connection_refusal()
                || e.is_timeout()
            {
                $self.notify(e).await;
            }
        }
    };
}

impl<C: ConnectionLike + Send> ConnectionLike for RedisResource<C> {
    fn req_packed_command<'a>(&'a mut self, cmd: &'a Cmd) -> RedisFuture<'a, Value> {
        (async move {
            if self.logging_enabled {
                self.log_cmd(cmd);
            }

            let result = self.con.req_packed_command(cmd).await;

            if self.logging_enabled {
                println!("REDIS <- {:?}", result);
            }

            notify_if_disconnected!(self, result);
            result
        })
        .boxed()
    }

    fn req_packed_commands<'a>(
        &'a mut self,
        cmd: &'a Pipeline,
        offset: usize,
        count: usize,
    ) -> RedisFuture<'a, Vec<Value>> {
        (async move {
            if self.logging_enabled {
                self.log_pipeline(cmd);
            }

            let result = self.con.req_packed_commands(cmd, offset, count).await;

            if self.logging_enabled {
                println!("REDIS <- {:?}", result);
            }

            notify_if_disconnected!(self, result);
            result
        })
        .boxed()
    }

    fn get_db(&self) -> i64 {
        self.con.get_db()
    }
}

/// Redis PubSub connection monitoring the connection state
pub struct MonitoredPubSub {
    pubsub: PubSub,
    handle: HandleRegistration,
}

impl MonitoredPubSub {
    fn new(con: Connection, handle: HandleRegistration) -> Self {
        Self {
            pubsub: con.into_pubsub(),
            handle,
        }
    }
}

#[async_trait]
impl PubSubResource for MonitoredPubSub {
    async fn psubscribe(&mut self, pchannel: &str) -> RedisResult<()> {
        self.pubsub.psubscribe(pchannel).await
    }

    async fn subscribe(&mut self, channel: &str) -> RedisResult<()> {
        self.pubsub.subscribe(channel).await
    }

    fn into_on_message<'a>(self) -> BoxStream<'a, Result<Msg, PubSubResourceError>> {
        let mut handle = self.handle.clone();

        let message_stream = self
            .pubsub
            .into_on_message()
            .map(Ok::<Msg, PubSubResourceError>);
        let error_stream = once(async move {
            handle.resource_died().await;
            Err(PubSubResourceError::StreamClosed)
        })
        .boxed();

        message_stream.chain(error_stream).boxed()
    }
}

/// [`RedisFactory`] implementation providing [`jatsl`] interop
pub struct MonitoredRedisFactory {
    url: String,
    handle_provider: BoxedResourceHandleProvider,
}

impl MonitoredRedisFactory {
    /// Creates a new factory opening connections to the given URL
    pub fn new(url: String, handle_provider: BoxedResourceHandleProvider) -> Self {
        Self {
            url,
            handle_provider,
        }
    }
}

#[async_trait]
impl RedisFactory for MonitoredRedisFactory {
    type PubSub = MonitoredPubSub;

    async fn pubsub(&self) -> Result<Self::PubSub, BoxedError> {
        let handle = self.handle_provider.create_handle();
        let resource = RedisResource::new(handle.clone(), &self.url).await?;

        Ok(MonitoredPubSub::new(resource.con, resource.handle))
    }

    async fn connection(
        &self,
        variant: RedisConnectionVariant,
    ) -> Result<Box<dyn ConnectionLike + Send + Sync>, BoxedError> {
        let handle = self.handle_provider.create_handle();

        match variant {
            // TODO Implement connection pooling
            RedisConnectionVariant::Owned | RedisConnectionVariant::Pooled => {
                Ok(Box::new(RedisResource::new(handle, &self.url).await?))
            }
            RedisConnectionVariant::Multiplexed => {
                Ok(Box::new(RedisResource::shared(handle, &self.url).await?))
            }
        }
    }
}

/// Factory to provide [`TaskResourceHandle`] instances
pub trait ResourceHandleProvider {
    /// Instantiates a new [`TaskResourceHandle`]
    fn create_handle(&self) -> TaskResourceHandle;
}

/// Stub resource handle provider
///
/// Creates new instances using [`TaskResourceHandle::stub()`] for situations where you do not need redundancy or task management
pub struct DummyResourceHandleProvider {}

impl DummyResourceHandleProvider {
    /// Creates a new instance wrapped in an [`Arc`]
    pub fn new() -> Arc<Self> {
        Arc::new(Self {})
    }
}

impl ResourceHandleProvider for DummyResourceHandleProvider {
    fn create_handle(&self) -> TaskResourceHandle {
        TaskResourceHandle::stub()
    }
}

impl<C> ResourceHandleProvider for TaskManager<C> {
    fn create_handle(&self) -> TaskResourceHandle {
        self.create_resource_handle()
    }
}

/// Dynamic dispatch version of [`ResourceHandleProvider`]
pub type BoxedResourceHandleProvider = Arc<dyn ResourceHandleProvider + Send + Sync>;

/// Communication factory based on [`MonitoredRedisFactory`]
pub struct RedisCommunicationFactory {
    url: String,
    handle_provider: BoxedResourceHandleProvider,
}

impl RedisCommunicationFactory {
    /// Creates a new instance which connects to the given URL and reports status using the given handle factory
    pub fn new(url: String, handle_provider: BoxedResourceHandleProvider) -> Self {
        Self {
            url,
            handle_provider,
        }
    }

    fn factory(&self) -> MonitoredRedisFactory {
        MonitoredRedisFactory::new(self.url.clone(), self.handle_provider.clone())
    }
}

impl CommunicationFactory for RedisCommunicationFactory {
    type QueueProvider = RedisQueueProvider<MonitoredRedisFactory>;
    type NotificationPublisher = RedisPublisher<MonitoredRedisFactory>;

    type Requestor = CompositeRequestor<
        RedisPublisher<MonitoredRedisFactory>,
        RedisResponseCollector<MonitoredRedisFactory>,
    >;

    type ResponseCollector = RedisResponseCollector<MonitoredRedisFactory>;
    type ResponsePublisher = RedisPublisher<MonitoredRedisFactory>;

    type ServiceAdvertiser = RedisServiceAdvertiser<MonitoredRedisFactory>;

    fn queue_provider(&self) -> Self::QueueProvider {
        Self::QueueProvider::new(self.factory())
    }

    fn notification_publisher(&self) -> Self::NotificationPublisher {
        Self::NotificationPublisher::new(self.factory())
    }

    fn requestor(&self) -> Self::Requestor {
        Self::Requestor::new(self.notification_publisher(), self.response_collector())
    }

    fn response_collector(&self) -> Self::ResponseCollector {
        Self::ResponseCollector::new(self.factory())
    }

    fn response_publisher(&self) -> Self::ResponsePublisher {
        Self::ResponsePublisher::new(self.factory())
    }

    fn service_advertiser(&self) -> Self::ServiceAdvertiser {
        Self::ServiceAdvertiser::new(self.factory())
    }
}

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
    D: ServiceDescriptor + Send + Sync + Eq + Hash + Serialize + DeserializeOwned,
{
    const NAME: &'static str = concat!(module_path!(), "::discovery");

    async fn execute(&self, manager: jatsl::JobManager) -> EmptyResult {
        let factory = MonitoredRedisFactory::new(self.url.clone(), Arc::new(manager));
        let backend = RedisPubSubBackend::new(factory).await?;

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
        let factory = RedisCommunicationFactory::new(self.url.clone(), Arc::new(manager));
        let advertiser = factory.service_advertiser();

        advertiser
            .advertise(self.service.clone(), self.endpoint.clone())
            .await?;

        Ok(())
    }
}
