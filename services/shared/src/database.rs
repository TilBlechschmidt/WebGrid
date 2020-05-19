use log::warn;
use redis::{aio::ConnectionManager, IntoConnectionInfo};
use std::time::Duration;
use tokio::time::{delay_for, timeout};

pub async fn connect(url: String) -> ConnectionManager {
    let retry_interval = Duration::from_secs(2);
    let request_timeout = Duration::from_secs(4);
    let connection_info = url.into_connection_info().unwrap();

    loop {
        let manager_future = ConnectionManager::new(connection_info.clone());
        let result = timeout(request_timeout, manager_future).await;

        match result {
            Ok(manager_result) => match manager_result {
                Ok(manager) => return manager,
                Err(e) => warn!("Unable to connect to redis server! ({})", e),
            },
            Err(e) => warn!("Timed out while connecting to redis! ({})", e),
        }

        delay_for(retry_interval).await;
    }
}
