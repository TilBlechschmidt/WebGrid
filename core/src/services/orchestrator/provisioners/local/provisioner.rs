use std::{collections::HashMap, sync::Arc};

use super::super::super::core::provisioner::{NodeInfo, Provisioner, ProvisionerCapabilities};
use crate::libraries::helpers::{constants, CapabilitiesRequest};
use anyhow::Result;
use async_trait::async_trait;
use log::warn;
use opentelemetry::{trace::TraceContextExt, Context as TelemetryContext};
use tokio::{
    process::{Child, Command},
    sync::Mutex,
};

struct ActiveSession {
    session_id: String,
    process: Child,
}

#[derive(Clone)]
pub struct LocalProvisioner {
    session: Arc<Mutex<Option<ActiveSession>>>,
    redis: String,
    log: String,
    trace_endpoint: Option<String>,
}

impl LocalProvisioner {
    pub fn new(redis: String, log: String, trace_endpoint: Option<String>) -> Self {
        Self {
            session: Arc::new(Mutex::new(None)),
            redis,
            log,
            trace_endpoint,
        }
    }
}

#[async_trait]
impl Provisioner for LocalProvisioner {
    fn capabilities(&self) -> ProvisionerCapabilities {
        ProvisionerCapabilities {
            // TODO When using e.g. safaridriver in iOS remote mode it won't be accurate
            platform_name: std::env::consts::OS.to_owned(),
            // TODO Fetch locally available browsers
            //      Best thing would be to provide a list of webdriver binaries
            browsers: vec!["safari".to_string()],
        }
    }

    async fn provision_node(
        &self,
        session_id: &str,
        _capabilities: CapabilitiesRequest,
    ) -> Result<NodeInfo> {
        let telemetry_context = TelemetryContext::current();
        let span = telemetry_context.span();

        // TODO Verify the capabilities

        let mut active_session = self.session.lock().await;

        if active_session.is_some() {
            warn!("There is an active session that will be killed by the new one!");
        }

        let mut additional_env: HashMap<String, String> = HashMap::new();

        if let Some(trace_endpoint) = &self.trace_endpoint {
            additional_env.insert("TRACE_ENDPOINT".to_string(), trace_endpoint.clone());
        }

        span.add_event("Spawning child process".to_string(), vec![]);

        let executable = std::env::current_exe()?;
        let child = Command::new(executable)
            .arg("node")
            // TODO Parametrize the next three
            .env("BROWSER", "safari")
            .env("DRIVER", "/usr/bin/safaridriver")
            .env("DRIVER_PORT", constants::PORT_LOCALDRIVER)
            .env("ID", session_id)
            .env("REDIS", &self.redis)
            .env("RUST_LOG", &self.log)
            .envs(additional_env)
            .spawn()?;

        *active_session = Some(ActiveSession {
            session_id: session_id.to_string(),
            process: child,
        });

        Ok(NodeInfo {
            // TODO Parametrize the host
            host: "localhost".to_string(),
            port: constants::PORT_NODE.to_owned(),
        })
    }

    async fn terminate_node(&self, session_id: &str) {
        let mut lock = self.session.lock().await;
        if let Some(mut active_session) = lock.take() {
            if session_id == active_session.session_id {
                active_session.process.kill().await.ok();
            } else {
                warn!(
                    "Attempted to terminate non-matching node {} (active: {})",
                    session_id, active_session.session_id
                );
                return;
            }
        } else {
            warn!("Attempted to terminate non-running node {}", session_id);
        }
    }
}
