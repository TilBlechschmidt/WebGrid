use super::super::{structs::NodeError, Context};
use crate::libraries::resources::{ResourceManager, ResourceManagerProvider};
use crate::libraries::{
    lifecycle::logging::{LogCode, SessionLogger},
    tracing::global_tracer,
};
use crate::with_redis_resource;
use crate::{
    libraries::helpers::{wait_for, Timeout},
    services::node::context::StartupContext,
};
use jatsl::TaskManager;
use log::{error, info};
use opentelemetry::{
    trace::{FutureExt, Span, TraceContextExt, Tracer},
    Context as TelemetryContext,
};
use redis::{aio::ConnectionLike, AsyncCommands};
use std::io::Error as IOError;
use std::net::SocketAddr;
use std::path::Path;
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

pub async fn start_driver(manager: TaskManager<StartupContext>) -> Result<(), NodeError> {
    let span = global_tracer()
        .start_with_context("Start driver", manager.context.telemetry_context.clone());

    let mut con = with_redis_resource!(manager);
    let startup_timeout = Timeout::DriverStartup.get(&mut con).await;
    let driver = &manager.context.options.driver;
    let driver_port = manager.context.options.driver_port;
    let browser = &manager.context.options.browser;

    let mut logger = SessionLogger::new(con, "node".to_string(), manager.context.id.clone());

    logger.log(LogCode::DStart, None).await.ok();

    // Spawn the driver
    span.add_event("Spawning process".to_string(), vec![]);
    let child = subtasks::launch_driver(&mut logger, driver, browser).await?;
    *manager.context.driver_reference.driver.lock().await = Some(child);

    // Await its startup
    let telemetry_context = TelemetryContext::current_with_span(span);
    subtasks::await_driver_startup(startup_timeout, driver_port, &mut logger)
        .with_context(telemetry_context)
        .await?;
    logger.log(LogCode::DAlive, None).await.ok();

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
    use opentelemetry::trace::StatusCode;

    use super::*;

    pub async fn launch_driver<C: ConnectionLike + AsyncCommands>(
        logger: &mut SessionLogger<C>,
        driver: &Path,
        browser: &str,
    ) -> Result<Child, NodeError> {
        // Chrome and safari need some "special handling"
        let res = match browser {
            "chrome" => Command::new(driver)
                .arg("--whitelisted-ips")
                .arg("*")
                .stdout(Stdio::inherit())
                .spawn(),
            "safari" => Command::new(driver)
                .arg("--diagnose")
                .arg("-p")
                .stdout(Stdio::inherit())
                .spawn(),
            _ => Command::new(driver).stdout(Stdio::inherit()).spawn(),
        };

        match res {
            Ok(child) => Ok(child),
            Err(e) => {
                logger
                    .log(LogCode::DFailure, Some(format!("{}", e)))
                    .await
                    .ok();

                error!("Failed to start driver {}", e);
                Err(NodeError::DriverStart(e))
            }
        }
    }

    pub async fn await_driver_startup<C: ConnectionLike + AsyncCommands>(
        timeout: usize,
        driver_port: u16,
        logger: &mut SessionLogger<C>,
    ) -> Result<(), NodeError> {
        let span = global_tracer().start("Awaiting driver startup");
        info!("Awaiting driver startup");

        let socket_addr: SocketAddr = ([127, 0, 0, 1], driver_port).into();
        let url = format!("http://{}/status", socket_addr);
        let telemetry_context = TelemetryContext::current_with_span(span);

        match wait_for(&url, Duration::from_secs(timeout as u64))
            .with_context(telemetry_context.clone())
            .await
        {
            Ok(_) => {
                info!("Driver became responsive");
                Ok(())
            }
            Err(_) => {
                error!("Timeout waiting for driver startup");
                logger.log(LogCode::DTimeout, None).await.ok();
                telemetry_context.span().set_status(
                    StatusCode::Error,
                    "Timeout waiting for driver startup".to_string(),
                );

                Err(NodeError::NoDriverResponse)
            }
        }
    }
}
