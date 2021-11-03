use super::{super::handle::HandleRegistration, RedisResource};
use jatsl::TaskResourceHandle;
use redis::aio::Connection;
use redis::{Client, RedisResult};
use std::time::Duration;
use tokio::time::{sleep, timeout};
use tracing::{debug, instrument, trace, warn};

impl RedisResource<Connection> {
    /// Creates a new standalone redis connection
    #[instrument(skip(handle))]
    pub async fn new(handle: TaskResourceHandle, url: &str) -> RedisResult<Self> {
        debug!("Instantiating new standalone redis client");

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

    #[instrument(skip(client))]
    async fn connect_standalone(client: Client) -> Connection {
        let retry_interval = Duration::from_secs(2);
        let request_timeout = Duration::from_secs(4);
        let mut attempt = 0;

        loop {
            trace!(attempt, "Connecting to redis");

            let con_future = client.get_async_connection();
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
