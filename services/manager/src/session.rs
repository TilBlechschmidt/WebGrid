use shared::lifecycle::wait_for;
use shared::logging::LogCode;
use shared::Timeout;

use chrono::prelude::*;
use futures::future::*;
use redis::{aio::MultiplexedConnection, pipe, AsyncCommands, Client, RedisResult};
use regex::Regex;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

use crate::structures::*;
use crate::context::Context;

async fn create_session(
    con: &MultiplexedConnection,
    capabilities: &str,
    ip: &str,
    user_agent: &str,
) -> Result<String, RequestError> {
    let mut con = con.clone();
    let session_id = Uuid::new_v4().to_hyphenated().to_string();
    let session_prefix = format!("session:{}", session_id);
    let status_key = format!("{}:status", session_prefix);
    let capabilities_key = format!("{}:capabilities", session_prefix);
    let downstream_key = format!("{}:downstream", session_prefix);
    let now = Utc::now().to_rfc3339();

    pipe()
        .atomic()
        .hset(status_key, "queuedAt", &now)
        .hset(capabilities_key, "requested", capabilities)
        .hset_multiple(
            downstream_key,
            &[("host", ip), ("userAgent", user_agent), ("lastSeen", &now)],
        )
        .sadd("sessions.active", &session_id)
        .query_async(&mut con)
        .map_err(RequestError::RedisError)
        .await?;

    Ok(session_id)
}

async fn request_slot(
    con: &MultiplexedConnection,
    session_id: &str,
    _capabilities: &str,
) -> Result<(), RequestError> {
    let mut con = con.clone();

    let queue_timeout = Timeout::Queue.get(&con).await;

    let orchestrators: Vec<String> = con.smembers("orchestrators").await.unwrap_or_else(|_| Vec::new());

    // TODO Match orchestrators according to capability
    let matching_orchestrators = orchestrators;
    let queues: Vec<String> = matching_orchestrators
        .iter()
        .map(|orchestrator| format!("orchestrator:{}:slots.available", orchestrator))
        .collect();

    if queues.is_empty() {
        return Err(RequestError::NoOrchestratorAvailable)
    }

    let response: Option<(String, String)> = con
        .blpop(queues, queue_timeout)
        .map_err(RequestError::RedisError)
        .await?;

    match response {
        Some((queue, slot)) => {
            lazy_static! {
                static ref RE: Regex = Regex::new(r"orchestrator:(?P<orchestrator>[^:]+):slots\.available").unwrap();
            }

            match RE.captures(&queue) {
                Some(groups) => {
                    let orchestrator = groups["orchestrator"].to_string();

                    con.set(format!("session:{}:slot", session_id), &slot)
                        .map_err(RequestError::RedisError)
                        .await?;
                    con.lpush(
                        format!("orchestrator:{}:backlog", &orchestrator),
                        session_id,
                    )
                    .map_err(RequestError::RedisError)
                    .await?;

                    Ok(())
                }
                None => Err(RequestError::ParseError),
            }
        }
        None => Err(RequestError::QueueTimeout),
    }
}

async fn await_scheduling(
    con: &MultiplexedConnection,
    session_id: &str,
) -> Result<(), RequestError> {
    let mut con = con.clone();

    let scheduling_timeout = Timeout::Scheduling.get(&con).await;
    let scheduling_key = format!("session:{}:orchestrator", session_id);

    let res: Option<()> = con
        .brpoplpush(&scheduling_key, &scheduling_key, scheduling_timeout)
        .map_err(RequestError::RedisError)
        .await?;

    match res {
        Some(_) => Ok(()),
        None => Err(RequestError::SchedulingTimeout),
    }
}

async fn await_healthcheck(
    con: &MultiplexedConnection,
    session_id: &str,
) -> Result<String, RequestError> {
    let mut con = con.clone();

    let (host, port): (String, String) = con
        .hget(
            format!("session:{}:upstream", session_id),
            &["host", "port"],
        )
        .map_err(RequestError::RedisError)
        .await?;

    let url = format!("http://{}:{}/status", host, port);
    let timeout = Timeout::NodeStartup.get(&con).await as u64;

    wait_for(&url, Duration::from_secs(timeout))
        .map_err(|_| RequestError::HealthCheckTimeout)
        .await
}

async fn run_session_setup(
    ctx: Arc<Context>,
    con: &MultiplexedConnection,
    session_id: &str,
    capabilities: &str,
) -> Result<(), RequestError> {
    let mut con = con.clone();

    ctx.logger
        .log(&session_id, LogCode::QUEUED, None)
        .await
        .ok();
    request_slot(&con, &session_id, capabilities).await?;
    ctx.logger
        .log(&session_id, LogCode::NALLOC, None)
        .await
        .ok();
    await_scheduling(&con, &session_id).await?;
    ctx.logger
        .log(&session_id, LogCode::PENDING, None)
        .await
        .ok();
    await_healthcheck(&con, &session_id).await?;
    ctx.logger
        .log(&session_id, LogCode::NALIVE, None)
        .await
        .ok();

    let now = Utc::now().to_rfc3339();
    let _: RedisResult<()> = con
        .hset(format!("session:{}:status", session_id), "aliveAt", &now)
        .await;

    Ok(())
}

pub async fn handle_create_session_request(
    ctx: Arc<Context>,
    remote_addr: &str,
    user_agent: &str,
    capabilities: &str,
) -> Result<SessionReplyValue, RequestError> {
    let client = Client::open(ctx.config.clone().redis_url).unwrap();
    let mut con = client.get_multiplexed_tokio_connection().await.unwrap();

    let session_id = create_session(&con, capabilities, remote_addr, user_agent).await?;
    let heartbeat_key = format!("session:{}:heartbeat.manager", session_id);

    ctx.heart.add_beat(heartbeat_key.clone(), 15, 30).await;

    let deferred = async {
        // TODO Run session termination workflow to do clean-up
        ctx.heart.stop_beat(heartbeat_key.clone()).await;
    };

    match run_session_setup(ctx.clone(), &con, &session_id, capabilities).await {
        Ok(()) => {
            deferred.await;
        }
        Err(e) => {
            let log_code = match e {
                RequestError::ParseError => LogCode::FAILURE,
                RequestError::RedisError(_) => LogCode::FAILURE,
                RequestError::QueueTimeout => LogCode::QTIMEOUT,
                RequestError::SchedulingTimeout => LogCode::OTIMEOUT,
                RequestError::HealthCheckTimeout => LogCode::NTIMEOUT,
                RequestError::NoOrchestratorAvailable => LogCode::QUNAVAILABLE,
            };

            ctx.logger.log(&session_id, log_code, None).await.ok();
            deferred.await;

            return Err(e);
        }
    };

    let actual_capabilities_str: String = con
        .hget(format!("session:{}:capabilities", session_id), "actual")
        .map_err(RequestError::RedisError)
        .await?;

    Ok(SessionReplyValue {
        session_id,
        capabilities: serde_json::from_str(&actual_capabilities_str).unwrap(),
    })
}
