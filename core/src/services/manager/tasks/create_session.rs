use super::super::{context::SessionCreationContext, RequestError, SessionReplyValue};
use crate::libraries::metrics::MetricsEntry;
use crate::libraries::resources::{ResourceManager, ResourceManagerProvider};
use crate::libraries::tracing::StringPropagator;
use crate::libraries::{
    helpers::{keys, parse_browser_string, CapabilitiesRequest, Timeout},
    tracing::global_tracer,
};
use crate::with_redis_resource;
use chrono::offset::Utc;
use futures::TryFutureExt;
use jatsl::TaskManager;
use lazy_static::lazy_static;
use log::{debug, warn};
use opentelemetry::trace::{FutureExt, Span, StatusCode as TelemetryStatusCode, Tracer};
use rand::seq::SliceRandom;
use rand::thread_rng;
use redis::{aio::ConnectionLike, pipe, AsyncCommands};
use regex::Regex;
use std::time::Instant;
use uuid::Uuid;

pub async fn create_session(
    manager: TaskManager<SessionCreationContext>,
) -> Result<SessionReplyValue, RequestError> {
    let mut con = with_redis_resource!(manager);
    let telemetry_context = &manager.context.telemetry_context;

    // Allocate a session ID
    let session_creation_start = Instant::now();
    let serialized_telemetry_context = StringPropagator::serialize(&telemetry_context).ok();
    let session_id =
        subtasks::create_new_session(&mut con, &manager.context, serialized_telemetry_context)
            .with_context(telemetry_context.clone())
            .await?;

    debug!("Created session object {}", session_id);

    // Start the heartbeat
    manager
        .context
        .heart_beat
        .add_beat(&keys::session::heartbeat::manager(&session_id), 15, 30)
        .await;

    // Create cleanup routine
    // TODO Call this on all exit paths
    let deferred = async {
        manager
            .context
            .heart_beat
            .stop_beat(&keys::session::heartbeat::manager(&session_id))
            .await;

        let elapsed_seconds = session_creation_start.elapsed().as_secs_f64();
        manager
            .context
            .metrics
            .submit(MetricsEntry::SessionStarted(elapsed_seconds))
            .ok();
    };

    // Create startup routine
    let startup = async {
        // Request a slot
        subtasks::request_slot(&mut con, &session_id, &manager.context.capabilities)
            .with_context(telemetry_context.clone())
            .await?;

        // Await scheduling & startup
        subtasks::await_scheduling(&mut con, &session_id)
            .with_context(telemetry_context.clone())
            .await?;

        // Hand off responsibility
        debug!("Session {} setup completed", &session_id);

        let now = Utc::now().to_rfc3339();
        con.hset::<_, _, _, ()>(keys::session::status(&session_id), "aliveAt", &now)
            .map_err(RequestError::RedisError)
            .await?;

        // Fetch response information
        let actual_capabilities_str: String = con
            .hget(keys::session::capabilities(&session_id), "actual")
            .map_err(RequestError::RedisError)
            .await?;

        // Send reply
        Ok(SessionReplyValue {
            session_id: session_id.clone(),
            capabilities: serde_json::from_str(&actual_capabilities_str).unwrap(),
        })
    };

    // Run startup routine, catching any errors along the way (to call the cleanup routine)
    match startup.await {
        Ok(response) => {
            deferred.await;
            Ok(response)
        }
        Err(e) => {
            warn!("Failed to setup session {} {:?}", session_id, e);
            deferred.await;
            Err(e)
        }
    }
}

mod subtasks {
    use super::*;
    use crate::libraries::tracing::constants::trace;
    use std::collections::HashMap;

    pub async fn create_new_session<C: ConnectionLike + AsyncCommands>(
        con: &mut C,
        context: &SessionCreationContext,
        serialized_telemetry_context: Option<String>,
    ) -> Result<String, RequestError> {
        let tracer = global_tracer();
        let span = tracer.start("Create session object");

        let session_id = Uuid::new_v4().to_hyphenated().to_string();
        let now = Utc::now().to_rfc3339();

        // Parse the capabilities to get the name/build metadata
        let requested_capabilities: CapabilitiesRequest =
            serde_json::from_str(&context.capabilities)
                .map_err(RequestError::InvalidCapabilities)?;

        let capability_sets = requested_capabilities.into_sets();
        let first_capability_set = capability_sets.first();

        let mut metadata = HashMap::new();

        if let Some(Some(Some(client_metadata))) =
            first_capability_set.map(|c| c.webgrid_options.clone().map(|o| o.metadata))
        {
            metadata = client_metadata;
        }

        pipe()
            .atomic()
            .hset(keys::session::status(&session_id), "queuedAt", &now)
            .hset_multiple(
                keys::session::telemetry::creation(&session_id),
                &[
                    ("traceID", span.span_context().trace_id().to_hex()),
                    ("context", serialized_telemetry_context.unwrap_or_default()),
                ],
            )
            .hset(
                keys::session::capabilities(&session_id),
                "requested",
                &context.capabilities,
            )
            .sadd(&(*keys::session::LIST_ACTIVE), &session_id)
            .query_async(con)
            .map_err(RequestError::RedisError)
            .await?;

        if !metadata.is_empty() {
            con.hset_multiple::<_, _, _, ()>(
                keys::session::metadata(&session_id),
                &metadata.into_iter().collect::<Vec<(String, String)>>(),
            )
            .map_err(RequestError::RedisError)
            .await?;
        }

        span.set_attribute(trace::SESSION_ID.string(session_id.clone()));

        Ok(session_id)
    }

    pub async fn request_slot<C: ConnectionLike + AsyncCommands>(
        con: &mut C,
        session_id: &str,
        capabilities: &str,
    ) -> Result<(), RequestError> {
        let tracer = global_tracer();
        let span = tracer.start("Request slot");
        let queue_timeout = Timeout::Queue.get(con).await;

        let mut queues: Vec<String> = helpers::match_orchestrators(con, capabilities)
            .await?
            .iter()
            .map(|orchestrator_id| keys::orchestrator::slots::available(orchestrator_id))
            .collect();

        if queues.is_empty() {
            span.set_status(
                TelemetryStatusCode::Error,
                "No matching orchestrator available".to_string(),
            );
            return Err(RequestError::NoOrchestratorAvailable);
        }

        // Ensure some degree of load balancing for orchestrators
        queues.shuffle(&mut thread_rng());

        span.add_event("Entering queue".to_string(), vec![]);

        let response: Option<(String, String)> = con
            .blpop(queues, queue_timeout)
            .map_err(RequestError::RedisError)
            .await?;

        span.add_event("Received response".to_string(), vec![]);

        match response {
            Some((queue, slot)) => {
                lazy_static! {
                    static ref RE: Regex =
                        Regex::new(r"orchestrator:(?P<orchestrator>[^:]+):slots\.available")
                            .unwrap();
                }

                match RE.captures(&queue) {
                    Some(groups) => {
                        let orchestrator = groups["orchestrator"].to_string();

                        span.set_attribute(
                            trace::SESSION_ORCHESTRATOR.string(orchestrator.clone()),
                        );

                        con.set(keys::session::slot(session_id), &slot)
                            .map_err(RequestError::RedisError)
                            .await?;
                        con.lpush(keys::orchestrator::backlog(&orchestrator), session_id)
                            .map_err(RequestError::RedisError)
                            .await?;

                        Ok(())
                    }
                    None => {
                        span.set_status(
                            TelemetryStatusCode::Error,
                            "Unable to parse redis response".to_string(),
                        );
                        Err(RequestError::ParseError)
                    }
                }
            }
            None => {
                span.set_status(
                    TelemetryStatusCode::Error,
                    "Timed out waiting for slot".to_string(),
                );
                Err(RequestError::QueueTimeout)
            }
        }
    }

    pub async fn await_scheduling<C: ConnectionLike + AsyncCommands>(
        con: &mut C,
        session_id: &str,
    ) -> Result<(), RequestError> {
        let tracer = global_tracer();
        let span = tracer.start("Await scheduling");

        let scheduling_timeout = Timeout::Scheduling.get(con).await;
        let scheduling_key = keys::session::orchestrator(session_id);

        let res: Option<()> = con
            .brpoplpush(&scheduling_key, &scheduling_key, scheduling_timeout)
            .map_err(RequestError::RedisError)
            .await?;

        match res {
            Some(_) => Ok(()),
            None => {
                span.set_status(
                    TelemetryStatusCode::Error,
                    "Timed out waiting for orchestrator to respond".to_string(),
                );
                Err(RequestError::SchedulingTimeout)
            }
        }
    }

    mod helpers {
        use super::*;

        pub async fn match_orchestrators<C: ConnectionLike + AsyncCommands>(
            con: &mut C,
            capabilities: &str,
        ) -> Result<Vec<String>, RequestError> {
            let requested_capabilities: CapabilitiesRequest =
                serde_json::from_str(capabilities).map_err(RequestError::InvalidCapabilities)?;

            let orchestrator_ids: Vec<String> = con
                .smembers(&(*keys::orchestrator::LIST))
                .await
                .unwrap_or_else(|_| Vec::new());

            // TODO Filter orchestrators by heartbeat

            let capability_sets = requested_capabilities.into_sets();
            let mut matching_orchestrators = Vec::with_capacity(orchestrator_ids.len());

            if capability_sets.is_empty() {
                return Ok(orchestrator_ids);
            }

            for id in orchestrator_ids.into_iter() {
                let platform_name: String = con
                    .get(keys::orchestrator::capabilities::platform_name(&id))
                    .await
                    .unwrap_or_default();
                let raw_browsers: Vec<String> = con
                    .smembers(keys::orchestrator::capabilities::browsers(&id))
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
                        platform_match = requested_platform_name == &platform_name
                            || requested_platform_name == "any";
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
                                version_match =
                                    browser.1.find(requested_browser_version) == Some(0);
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
    }
}
