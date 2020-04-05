use chrono::prelude::*;
use redis::{AsyncCommands, RedisResult};
use std::sync::Arc;
use std::time::Duration;
use tokio::time;
use uuid::Uuid;

use shared::{logging::LogCode, Timeout};

mod config;
mod context;
mod reclaim;
pub mod provisioner;

use crate::provisioner::{Provisioner, Type as ProvisionerType};
use crate::context::Context;
use crate::reclaim::reclaim_slots;

async fn slot_reclaimer(ctx: Arc<Context>) {
    let mut con = ctx.con.clone();
    let interval_seconds = Timeout::SlotReclaimInterval.get(&con).await as u64;
    let mut interval = time::interval(Duration::from_secs(interval_seconds));

    loop {
        if let Ok((dead, orphaned)) = reclaim_slots(&mut con, &ctx.config.orchestrator_id).await {
            println!(
                "Reclaim cycle executed.\n\tDead: {:?}\n\tOrph: {:?}",
                dead, orphaned
            );
        }

        interval.tick().await;
    }
}

async fn slot_recycler(ctx: Arc<Context>) {
    let mut con = ctx.client.get_multiplexed_tokio_connection().await.unwrap();

    let source = format!(
        "orchestrator:{}:slots.reclaimed",
        ctx.config.orchestrator_id
    );
    let destination = format!(
        "orchestrator:{}:slots.available",
        ctx.config.orchestrator_id
    );

    loop {
        let slot: RedisResult<String> = con.brpoplpush(&source, &destination, 0).await;

        if let Ok(slot) = slot {
            println!("Recycled slot: {}", slot);
        }
    }
}

#[rustfmt::skip]
async fn job_processor<P: Provisioner>(ctx: Arc<Context>, provisioner: P) {
    let mut con = ctx.client.get_multiplexed_tokio_connection().await.unwrap();

    let backlog = format!("orchestrator:{}:backlog", ctx.config.orchestrator_id);
    let pending = format!("orchestrator:{}:pending", ctx.config.orchestrator_id);

    loop {
        // While loop first to process leftover tasks from prior instance
        while let Ok(session_id) = con.lindex(&pending, -1).await {
            let session_id: String = session_id;

            let info_future = provisioner.provision_node(&session_id);
            let node_info = info_future.await;

            let status_key = format!("session:{}:status", session_id);
            let orchestrator_key = format!("session:{}:orchestrator", session_id);
            let upstream_key = format!("session:{}:upstream", session_id);
            let timestamp = Utc::now().to_rfc3339();

            let result: RedisResult<()> = redis::pipe()
                .atomic()
                .cmd("LPOP").arg(&pending)
                .cmd("HSETNX").arg(status_key).arg("pendingAt").arg(timestamp)
                .cmd("RPUSH").arg(orchestrator_key).arg(&ctx.config.orchestrator_id)
                .cmd("HMSET").arg(upstream_key)
                    .arg("host").arg(node_info.host)
                    .arg("port").arg(node_info.port)
                .query_async(&mut con)
                .await;

            if result.is_err() {
                ctx.logger.log(&session_id, LogCode::STARTFAIL, None).await.ok();
                provisioner.terminate_node(&session_id).await;
            } else {
                ctx.logger.log(&session_id, LogCode::SCHED, None).await.ok();
            }

            let _: RedisResult<()> = con.rpop(&pending).await;
        }

        let _: RedisResult<()> = con.brpoplpush(&backlog, &pending, 0).await;
    }
}

async fn slot_count_adjuster(ctx: Arc<Context>) -> RedisResult<()> {
    let mut con = ctx.client.get_multiplexed_tokio_connection().await?;
    let slots_key = format!("orchestrator:{}:slots", ctx.config.orchestrator_id);
    let reclaimed_key = format!(
        "orchestrator:{}:slots.reclaimed",
        ctx.config.orchestrator_id
    );
    let available_key = format!(
        "orchestrator:{}:slots.available",
        ctx.config.orchestrator_id
    );

    let target: usize = ctx.config.slots;
    let current: usize = con.scard(&slots_key).await?;

    if target < current {
        let delta = current - target;
        println!("Removing {} slots!", delta);
        for _ in 0..delta {
            let (_, slot_id): (String, String) = con.brpop(&available_key, 0).await?;
            let _: () = con.srem(&slots_key, &slot_id).await?;
        }
    } else if target > current {
        let delta = target - current;
        println!("Adding {} slots!", delta);
        for _ in 0..delta {
            let slot_id = Uuid::new_v4().to_hyphenated().to_string();

            let _: () = redis::pipe()
                .atomic()
                .cmd("SADD")
                .arg(&slots_key)
                .arg(&slot_id)
                .cmd("RPUSH")
                .arg(&reclaimed_key)
                .arg(&slot_id)
                .query_async(&mut con)
                .await?;
        }
    }

    let slots: Vec<String> = con.smembers(slots_key).await?;
    println!("Slots: {:?}", slots);

    Ok(())
}

pub async fn start<P: Provisioner + Send + Sync + Clone + 'static>(provisioner_type: ProvisionerType, provisioner: P) {
    let ctx = Arc::new(Context::new().await);
    let mut con = ctx.con.clone();

    let type_str = format!("{}", provisioner_type);

    // Register with backing store
    let info_key = format!("orchestrator:{}", ctx.config.orchestrator_id);
    let _: () = con
        .hset_multiple(&info_key, &[("type", type_str)])
        .await
        .unwrap();
    let _: () = con
        .sadd("orchestrators", &ctx.config.orchestrator_id)
        .await
        .unwrap();

    // Create heartbeat
    let heartbeat_key = format!("orchestrator:{}:heartbeat", ctx.config.orchestrator_id);
    ctx.heart.add_beat(heartbeat_key.clone(), 60, 120).await;

    // Start slot reclaimer
    let ctx_reclaimer = ctx.clone();
    tokio::spawn(async {
        slot_reclaimer(ctx_reclaimer).await;
    });

    // Start slot recycler (.reclaimed -> .available)
    let ctx_recycler = ctx.clone();
    tokio::spawn(async {
        slot_recycler(ctx_recycler).await;
    });

    // Start job processor
    let ctx_job_processor = ctx.clone();
    tokio::spawn(async move {
        job_processor(ctx_job_processor, provisioner).await;
    });

    // Run slot count adjuster
    let ctx_adjuster = ctx.clone();
    tokio::spawn(async {
        slot_count_adjuster(ctx_adjuster).await.unwrap();
    });

    // Run until we die!
    ctx.heart.beat(true).await;

    // Do a clean shutdown
    let _: () = con
        .srem("orchestrators", &ctx.config.orchestrator_id)
        .await
        .unwrap();
    let _: () = con.del(&info_key).await.unwrap();
    ctx.heart.stop_beat(heartbeat_key).await;
}
