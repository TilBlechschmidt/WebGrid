use log::{error, info};
use std::io::Error as IOError;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use shared::{lifecycle::wait_for, logging::LogCode, Timeout};

use crate::context::Context;
use crate::NodeError;

pub struct DriverManager {
    path: String,
    driver: Mutex<Option<Child>>,
}

impl DriverManager {
    pub fn new(path: String) -> Self {
        DriverManager {
            path,
            driver: Mutex::new(None),
        }
    }

    pub fn start(&self) -> Result<(), IOError> {
        // Chrome needs some "special handling"
        let driver = if std::env::var("BROWSER").unwrap_or_default() == "chrome" {
            Command::new(self.path.clone())
                .arg("--whitelisted-ips")
                .arg("*")
                .stdout(Stdio::inherit())
                .spawn()
        } else {
            Command::new(self.path.clone())
                .stdout(Stdio::inherit())
                .spawn()
        };

        match driver {
            Ok(child) => {
                *self.driver.lock().unwrap() = Some(child);
                Ok(())
            }
            Err(err) => Err(err),
        }
    }

    pub fn stop(&self) {
        let mut driver = self.driver.lock().unwrap();

        if let Some(d) = driver.as_mut() {
            d.kill().ok();
        };

        *driver = None;
    }
}

async fn await_driver_startup(ctx: Arc<Context>) -> Result<(), NodeError> {
    let timeout = Timeout::DriverStartup.get(&ctx.con).await;

    info!("Awaiting driver startup");

    match wait_for(
        &ctx.get_driver_url("/status"),
        Duration::from_secs(timeout as u64),
    )
    .await
    {
        Ok(_) => {
            info!("Driver became responsive");
            Ok(())
        }
        Err(_) => {
            error!("Timeout waiting for driver startup");
            ctx.logger.log(LogCode::DTIMEOUT, None).await.ok();
            Err(NodeError::NoDriverResponse)
        }
    }
}

pub async fn start_driver(ctx: Arc<Context>) -> Result<(), NodeError> {
    ctx.logger.log(LogCode::DSTART, None).await.ok();

    info!("Starting local driver");

    match ctx.driver.start() {
        Ok(_) => {
            await_driver_startup(ctx.clone()).await?;
            ctx.logger.log(LogCode::DALIVE, None).await.ok();
            Ok(())
        }
        Err(e) => {
            error!("Failed to start driver {}", e);
            ctx.logger
                .log(LogCode::DFAILURE, Some(format!("{}", e)))
                .await
                .ok();
            Err(NodeError::DriverStart(e))
        }
    }
}
