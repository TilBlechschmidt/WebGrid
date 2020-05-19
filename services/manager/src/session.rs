use shared::capabilities::CapabilitiesRequest;
use shared::lifecycle::wait_for;
use shared::logging::LogCode;
use shared::metrics::MetricsEntry;
use shared::{parse_browser_string, Timeout};

use chrono::prelude::*;
use futures::future::*;
use log::{debug, warn};
use redis::{aio::ConnectionManager, pipe, AsyncCommands, RedisResult};
use regex::Regex;
use serde_json;
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;
use uuid::Uuid;

use crate::context::Context;
use crate::structures::*;

async fn create_session(
    con: &ConnectionManager,
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

async fn match_orchestrators(
    con: &ConnectionManager,
    capabilities: &str,
) -> Result<Vec<String>, RequestError> {
    let mut con = con.clone();

    let requested_capabilities: CapabilitiesRequest =
        serde_json::from_str(capabilities).map_err(RequestError::InvalidCapabilities)?;

    let orchestrator_ids: Vec<String> = con
        .smembers("orchestrators")
        .await
        .unwrap_or_else(|_| Vec::new());

    let capability_sets = requested_capabilities.into_sets();
    let mut matching_orchestrators = Vec::with_capacity(orchestrator_ids.len());

    if capability_sets.is_empty() {
        return Ok(orchestrator_ids);
    }

    for id in orchestrator_ids.into_iter() {
        let platform_name: String = con
            .get(format!("orchestrator:{}:capabilities:platformName", id))
            .await
            .unwrap_or_default();
        let raw_browsers: Vec<String> = con
            .smembers(format!("orchestrator:{}:capabilities:browsers", id))
            .await
            .unwrap_or_else(|_| Vec::new());
        let browsers: Vec<(String, String)> = raw_browsers
            .into_iter()
            .filter_map(|raw_browser| parse_browser_string(&raw_browser))
            .collect();

        for capability in &capability_sets {
            let mut platform_match = true;
            let mut browser_match = true;

            if let Some(requested_platform_name) = &capability.platform_name {
                platform_match = requested_platform_name == &platform_name;
            }

            if !browsers.is_empty() {
                browser_match = false;

                for browser in &browsers {
                    let mut version_match = true;
                    let mut name_match = true;

                    if let Some(requested_browser_name) = &capability.browser_name {
                        name_match = &browser.0 == requested_browser_name;
                    }

                    if let Some(requested_browser_version) = &capability.browser_version {
                        version_match = browser.1.find(requested_browser_version) == Some(0);
                    }

                    browser_match = browser_match || (version_match && name_match);
                }
            }

            if platform_match && browser_match {
                matching_orchestrators.push(id);
                break;
            }
        }
    }

    Ok(matching_orchestrators)
}

async fn request_slot(
    con: &ConnectionManager,
    session_id: &str,
    capabilities: &str,
) -> Result<(), RequestError> {
    let mut con = con.clone();

    let queue_timeout = Timeout::Queue.get(&con).await;

    let queues: Vec<String> = match_orchestrators(&con, capabilities)
        .await?
        .iter()
        .map(|orchestrator| format!("orchestrator:{}:slots.available", orchestrator))
        .collect();

    if queues.is_empty() {
        return Err(RequestError::NoOrchestratorAvailable);
    }

    let response: Option<(String, String)> = con
        .blpop(queues, queue_timeout)
        .map_err(RequestError::RedisError)
        .await?;

    match response {
        Some((queue, slot)) => {
            lazy_static! {
                static ref RE: Regex =
                    Regex::new(r"orchestrator:(?P<orchestrator>[^:]+):slots\.available").unwrap();
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

async fn await_scheduling(con: &ConnectionManager, session_id: &str) -> Result<(), RequestError> {
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
    con: &ConnectionManager,
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
    con: &ConnectionManager,
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
    let mut con = ctx.create_client().await;
    let session_creation_start = Instant::now();

    let session_id = create_session(&con, capabilities, remote_addr, user_agent).await?;
    let heartbeat_key = format!("session:{}:heartbeat.manager", session_id);

    debug!("Created session object {}", session_id);

    ctx.heart.add_beat(heartbeat_key.clone(), 15, 30).await;

    let deferred = async {
        // TODO Run session termination workflow on error to do clean-up
        ctx.heart.stop_beat(heartbeat_key.clone()).await;

        let elapsed_seconds = session_creation_start.elapsed().as_secs_f64();
        ctx.metrics_tx
            .send(MetricsEntry::SessionStarted(elapsed_seconds))
            .ok();
    };

    match run_session_setup(ctx.clone(), &con, &session_id, capabilities).await {
        Ok(()) => {
            debug!("Session {} setup completed", session_id);
            deferred.await;
        }
        Err(e) => {
            warn!("Failed to setup session {} {:?}", session_id, e);

            let log_code = match e {
                RequestError::ParseError => LogCode::FAILURE,
                RequestError::RedisError(_) => LogCode::FAILURE,
                RequestError::QueueTimeout => LogCode::QTIMEOUT,
                RequestError::SchedulingTimeout => LogCode::OTIMEOUT,
                RequestError::HealthCheckTimeout => LogCode::NTIMEOUT,
                RequestError::NoOrchestratorAvailable => LogCode::QUNAVAILABLE,
                RequestError::InvalidCapabilities(_) => LogCode::INVALIDCAP,
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
