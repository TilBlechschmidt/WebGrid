use super::{PubSubResource, PubSubResourceError};
use crate::libraries::scheduling::TaskResourceHandle;
use async_trait::async_trait;
use core::pin::Pin;
use futures::{
    future::{BoxFuture, FutureExt, Shared},
    lock::{Mutex, MutexGuard},
    stream::{once, Stream, StreamExt},
};
use lazy_static::lazy_static;
use log::{debug, warn};
use redis::{
    aio::{Connection, ConnectionLike, MultiplexedConnection, PubSub},
    Client, Cmd, Msg, Pipeline, RedisError, RedisFuture, RedisResult, Value,
};
use std::{
    collections::HashSet,
    ops::{Deref, DerefMut},
    sync::Arc,
    time::Duration,
};
use tokio::task;
use tokio::task::yield_now;
use tokio::time::{delay_for, timeout};

type SharedMultiplexedConnectionFuture = Shared<BoxFuture<'static, MultiplexedConnection>>;

lazy_static! {
    static ref SHARED_CONNECTION: Mutex<Option<Arc<SharedMultiplexedConnectionFuture>>> =
        Mutex::new(None);
    static ref SHARED_TASK_RESOURCE_HANDLES: Mutex<HashSet<TaskResourceHandle>> =
        Mutex::new(HashSet::new());
}

/// Individual redis resource created on-demand
pub type StandaloneRedisResource = RedisResource<Connection>;
/// Multiplexed redis resource shared between jobs
pub type SharedRedisResource = RedisResource<MultiplexedConnection>;

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
                debug!("Reusing existing shared connection!");
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

            delay_for(retry_interval).await;
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

            delay_for(retry_interval).await;
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
        // TODO Print the error
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

    // TODO Think about whether or not this is reasonable practice!
    // Also consider where to do this on a shared connection, it would be wasteful to do so for everyone that holds a copy (albeit maybe necessary)
    // /// Sends PING commands in regular intervals to check if the connection is alive
    // /// Usually the application runs into a read timeout when the remote end has died,
    // /// however with no commands being written this may take a long time (usually >10min).
    // /// ref: http://man7.org/linux/man-pages/man7/tcp.7.html -> tcp_retries2
    // fn spawn_ping_task(&self, interval: Duration) {
    //     let mut con = self.clone();
    //     tokio::spawn(async move {
    //         loop {
    //             if let Err(_) = redis::cmd("PING").query_async::<_, ()>(&mut con).await {
    //                 break;
    //             }

    //             sleep(interval).await;
    //         }
    //     });
    // }

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
                self.log_cmd(&cmd);
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
                self.log_pipeline(&cmd);
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

impl From<StandaloneRedisResource> for Box<dyn PubSubResource + Send> {
    fn from(w: StandaloneRedisResource) -> Box<dyn PubSubResource + Send> {
        let handle = w.handle.clone();
        let con = w.con;

        Box::new(MonitoredPubSub::new(con, handle))
    }
}

struct MonitoredPubSub {
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

    fn on_message<'a>(
        &'a mut self,
    ) -> Pin<Box<dyn Stream<Item = Result<Msg, PubSubResourceError>> + Send + 'a>> {
        let mut handle = self.handle.clone();

        let message_stream = self.pubsub.on_message().map(Ok::<Msg, PubSubResourceError>);
        let error_stream = once(async move {
            handle.resource_died().await;
            Err(PubSubResourceError::StreamClosed)
        })
        .boxed();

        message_stream.chain(error_stream).boxed()
    }
}
