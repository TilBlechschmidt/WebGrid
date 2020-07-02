use crate::{structs::NodeError, Context};
use helpers::{env, wait_for, Timeout};
use lifecycle::logging::{LogCode, SessionLogger};
use log::{error, info};
use redis::{aio::ConnectionLike, AsyncCommands};
use resources::{with_redis_resource, ResourceManager};
use scheduling::TaskManager;
use std::io::Error as IOError;
use std::net::SocketAddr;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct DriverReference {
    driver: Arc<Mutex<Option<Child>>>,
}

impl DriverReference {
    pub fn new() -> Self {
        Self {
            driver: Arc::new(Mutex::new(None)),
        }
    }
}

pub async fn start_driver(manager: TaskManager<Context>) -> Result<(), NodeError> {
    let mut con = with_redis_resource!(manager);
    let startup_timeout = Timeout::DriverStartup.get(&mut con).await;

    let mut logger = SessionLogger::new(con, "node".to_string(), env::service::node::ID.clone());

    logger.log(LogCode::DSTART, None).await.ok();

    // Spawn the driver
    let child = subtasks::launch_driver(&mut logger).await?;
    *manager.context.driver_reference.driver.lock().await = Some(child);

    // Await its startup
    subtasks::await_driver_startup(startup_timeout, &mut logger).await?;
    logger.log(LogCode::DALIVE, None).await.ok();

    Ok(())
}

pub async fn stop_driver(manager: TaskManager<Context>) -> Result<(), IOError> {
    let mut driver = manager.context.driver_reference.driver.lock().await;

    if let Some(d) = driver.as_mut() {
        d.kill().ok();
    };

    *driver = None;

    Ok(())
}

mod subtasks {
    use super::*;

    pub async fn launch_driver<C: ConnectionLike + AsyncCommands>(
        logger: &mut SessionLogger<C>,
    ) -> Result<Child, NodeError> {
        let path = (*env::service::node::DRIVER).clone();

        // Chrome needs some "special handling"
        let browser = std::env::var("BROWSER").unwrap_or_default();

        let res = match browser.as_str() {
            "chrome" => Command::new(path.clone())
                .arg("--whitelisted-ips")
                .arg("*")
                .stdout(Stdio::inherit())
                .spawn(),
            "safari" => Command::new(path.clone())
                .arg("--diagnose")
                .arg("-p")
                .arg("9998")
                .stdout(Stdio::inherit())
                .spawn(),
            _ => Command::new(path.clone()).stdout(Stdio::inherit()).spawn(),
        };

        match res {
            Ok(child) => Ok(child),
            Err(e) => {
                logger
                    .log(LogCode::DFAILURE, Some(format!("{}", e)))
                    .await
                    .ok();

                error!("Failed to start driver {}", e);
                Err(NodeError::DriverStart(e))
            }
        }
    }

    pub async fn await_driver_startup<C: ConnectionLike + AsyncCommands>(
        timeout: usize,
        logger: &mut SessionLogger<C>,
    ) -> Result<(), NodeError> {
        info!("Awaiting driver startup");

        let socket_addr: SocketAddr = ([127, 0, 0, 1], *env::service::node::DRIVER_PORT).into();
        let url = format!("http://{}/status", socket_addr);

        match wait_for(&url, Duration::from_secs(timeout as u64)).await {
            Ok(_) => {
                info!("Driver became responsive");
                Ok(())
            }
            Err(_) => {
                error!("Timeout waiting for driver startup");
                logger.log(LogCode::DTIMEOUT, None).await.ok();

                Err(NodeError::NoDriverResponse)
            }
        }
    }
}
