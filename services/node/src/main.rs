#[macro_use]
extern crate lazy_static;
use chrono::prelude::*;
use log::{error, info};

use std::process::Command;
use std::sync::Arc;

use shared::lifecycle::{generate_session_termination_script, DeathReason};
use shared::logging::LogCode;
use shared::service_init;

mod config;
mod context;
mod driver;
mod proxy;
mod startup;
mod structs;

use crate::context::Context;
use crate::driver::start_driver;
use crate::proxy::serve_proxy;
use crate::startup::{create_local_session, resize_window};
use crate::structs::NodeError;

fn call_on_create_script(ctx: Arc<Context>) {
    match &ctx.config.on_session_create {
        Some(script) => {
            info!("Calling on_create_script {}", script);
            let parts: Vec<String> = script.split_whitespace().map(|s| s.to_string()).collect();
            let process = Command::new(parts[0].clone()).args(&parts[1..]).spawn();
            if let Err(e) = process {
                error!("Failed to execute on_create_script {:?}", e);
            }
        }
        None => {}
    }
}

async fn node_startup(ctx: Arc<Context>) -> Result<(), NodeError> {
    start_driver(ctx.clone()).await?;
    let session_id = create_local_session(ctx.clone()).await?;
    resize_window(ctx.clone(), &session_id).await?;
    serve_proxy(ctx.clone(), session_id).await;
    call_on_create_script(ctx);

    Ok(())
}

async fn terminate_session(ctx: Arc<Context>) {
    ctx.driver.stop();

    let mut con = ctx.con.clone();
    let _: Option<()> = generate_session_termination_script(false)
        .arg(ctx.config.session_id.clone())
        .arg(Utc::now().to_rfc3339())
        .invoke_async(&mut con)
        .await
        .ok();
}

#[tokio::main]
async fn main() {
    service_init();

    let ctx = Arc::new(Context::new().await);
    ctx.heart
        .add_beat(
            format!("session:{}:heartbeat.node", ctx.config.session_id),
            60,
            120,
        )
        .await;

    ctx.logger.log(LogCode::BOOT, None).await.ok();

    let ctx_startup = ctx.clone();
    tokio::spawn(async {
        let heart = ctx_startup.heart.clone();
        if let Err(e) = node_startup(ctx_startup).await {
            error!("Startup routine failed {:?}", e);
            heart.kill();
        }
    });

    // The heart will keep beating until either the session is closed, a timeout occurs or the signal handler triggers.
    match ctx.heart.beat(true).await {
        DeathReason::LifetimeExceeded => {
            info!("Lifetime was exceeded");
            ctx.logger.log(LogCode::STIMEOUT, None).await.ok();
        }
        DeathReason::Killed => {
            ctx.logger.log(LogCode::CLOSED, None).await.ok();
        }
    }

    terminate_session(ctx.clone()).await;
    ctx.logger.log(LogCode::HALT, None).await.ok();
}
