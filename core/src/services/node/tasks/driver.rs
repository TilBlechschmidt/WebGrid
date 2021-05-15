use super::super::{structs::NodeError, Context};
use crate::libraries::tracing::global_tracer;
use crate::{libraries::helpers::wait_for, services::node::context::StartupContext};
use jatsl::TaskManager;
use log::{error, info};
use opentelemetry::trace::StatusCode;
use opentelemetry::{
    trace::{FutureExt, Span, TraceContextExt, Tracer},
    Context as TelemetryContext,
};
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
    let mut span = global_tracer()
        .start_with_context("Start driver", manager.context.telemetry_context.clone());

    let startup_timeout = manager.context.options.timeout_driver_startup;
    let driver = &manager.context.options.driver;
    let driver_port = manager.context.options.driver_port;
    let browser = &manager.context.options.browser;

    // Spawn the driver
    span.add_event("Spawning process".to_string(), vec![]);
    let child = subtasks::launch_driver(driver, browser).await?;
    *manager.context.driver_reference.driver.lock().await = Some(child);

    // Await its startup
    let telemetry_context = TelemetryContext::current_with_span(span);
    subtasks::await_driver_startup(startup_timeout, driver_port)
        .with_context(telemetry_context)
        .await?;

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

    pub async fn launch_driver(driver: &Path, browser: &str) -> Result<Child, NodeError> {
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
                error!("Failed to start driver {}", e);
                Err(NodeError::DriverStart(e))
            }
        }
    }

    pub async fn await_driver_startup(
        timeout: Duration,
        driver_port: u16,
    ) -> Result<(), NodeError> {
        let span = global_tracer().start("Awaiting driver startup");
        info!("Awaiting driver startup");

        let socket_addr: SocketAddr = ([127, 0, 0, 1], driver_port).into();
        let url = format!("http://{}/status", socket_addr);
        let telemetry_context = TelemetryContext::current_with_span(span);

        match wait_for(&url, timeout)
            .with_context(telemetry_context.clone())
            .await
        {
            Ok(_) => {
                info!("Driver became responsive");
                Ok(())
            }
            Err(_) => {
                error!("Timeout waiting for driver startup");
                telemetry_context.span().set_status(
                    StatusCode::Error,
                    "Timeout waiting for driver startup".to_string(),
                );

                Err(NodeError::NoDriverResponse)
            }
        }
    }
}
