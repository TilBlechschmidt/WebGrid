use super::{super::handle::HandleRegistration, RedisResource, SHARED_TASK_RESOURCE_HANDLES};
use futures::future::Shared;
use futures::lock::{Mutex, MutexGuard};
use futures::FutureExt;
use jatsl::TaskResourceHandle;
use juniper::BoxFuture;
use lazy_static::lazy_static;
use redis::aio::MultiplexedConnection;
use redis::{Client, RedisResult};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::{sleep, timeout};
use tracing::{debug, instrument, trace, warn};

type SharedMultiplexedConnectionFuture = Shared<BoxFuture<'static, MultiplexedConnection>>;

lazy_static! {
    pub(super) static ref SHARED_CONNECTION: Mutex<Option<Arc<SharedMultiplexedConnectionFuture>>> =
        Mutex::new(None);
}

impl RedisResource<MultiplexedConnection> {
    /// Retrieves a shared redis instance or instantiates it if it doesn't exist
    #[instrument(skip(handle))]
    pub async fn shared(handle: TaskResourceHandle, url: &str) -> RedisResult<Self> {
        debug!("Instantiating shared redis client handle");

        let client = Client::open(url)?;
        let shared_con_lock = SHARED_CONNECTION.lock().await;

        let future = match &(*shared_con_lock) {
            Some(container_future) => {
                trace!("Reusing existing shared connection");
                container_future.clone()
            }
            None => {
                trace!("Creating new shared instance");
                RedisResource::load_new_shared_handle(client, shared_con_lock)
            }
        };

        let con = (*future).clone().await;

        trace!("Inserting task resource handle");
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

    #[instrument(skip(client, shared_con_lock))]
    fn load_new_shared_handle(
        client: Client,
        mut shared_con_lock: MutexGuard<Option<Arc<SharedMultiplexedConnectionFuture>>>,
    ) -> Arc<SharedMultiplexedConnectionFuture> {
        let future = RedisResource::connect_shared(client).boxed().shared();
        let arc_future = Arc::new(future);
        *shared_con_lock = Some(arc_future.clone());

        arc_future
    }

    #[instrument(skip(client))]
    async fn connect_shared(client: Client) -> MultiplexedConnection {
        let retry_interval = Duration::from_secs(2);
        let request_timeout = Duration::from_secs(4);
        let mut attempt = 0;

        loop {
            trace!(attempt, "Connecting to redis");

            let con_future = client.get_multiplexed_tokio_connection();
            let timed_con_future = timeout(request_timeout, con_future);

            match timed_con_future.await {
                Ok(con_result) => match con_result {
                    Ok(connection) => return connection,
                    Err(error) => {
                        warn!(?error, "Failed to connect to redis")
                    }
                },
                Err(error) => {
                    warn!(?error, "Timeout connecting to redis")
                }
            }

            sleep(retry_interval).await;
            attempt += 1;
        }
    }
}
