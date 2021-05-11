use super::super::super::core::provisioner::{
    match_image_from_capabilities, NodeInfo, Provisioner, ProvisionerCapabilities,
};
use crate::libraries::{
    helpers::{load_config, replace_config_variable, CapabilitiesRequest},
    tracing::{constants::trace, global_tracer},
};
use anyhow::{bail, Result};
use async_trait::async_trait;
use futures::{StreamExt, TryStreamExt};
use k8s_openapi::{
    api::{
        batch::v1::Job,
        core::v1::{Pod, PodStatus},
    },
    Resource,
};
use kube::{
    api::{Api, DeleteParams, ListParams, Meta, PostParams, PropagationPolicy, WatchEvent},
    error::Error as KubeError,
    Client,
};
use log::{error, info, trace, warn};
use opentelemetry::{
    trace::{FutureExt, Span, TraceContextExt, Tracer},
    Context as TelemetryContext,
};
use opentelemetry_semantic_conventions as semcov;
use serde::{de::DeserializeOwned, ser::Serialize};

#[derive(Clone)]
pub struct K8sProvisioner {
    namespace: String,
    images: Vec<(String, String)>,
    node_port: u16,
}

impl K8sProvisioner {
    pub async fn new(node_port: u16, images: Vec<(String, String)>) -> Self {
        if images.is_empty() {
            warn!("No images provided! Orchestrator won't be able to schedule nodes.");
        }

        let namespace = std::env::var("NAMESPACE").unwrap_or_else(|_| "webgrid".into());

        info!("Operating in K8s namespace {}", namespace);

        Self {
            namespace,
            images,
            node_port,
        }
    }

    fn generate_name(session_id: &str) -> String {
        let prefix = std::env::var("WEBGRID_RESOURCE_PREFIX").unwrap_or_default();
        let short_id = &session_id[..8];
        format!("{}session-{}", prefix, short_id)
    }

    async fn get_api<T: Resource>(&self) -> Api<T> {
        let client = Client::try_default().await.unwrap();
        Api::namespaced(client, &self.namespace)
    }

    async fn create_resource<T: Resource + Meta + DeserializeOwned + Serialize + Clone>(
        &self,
        value: &T,
    ) -> Result<T, KubeError> {
        let api = self.get_api::<T>().await;

        match api.create(&PostParams::default(), value).await {
            Ok(o) => {
                let name = Meta::name(&o);
                info!("Created {} {}", T::KIND, name);
                Ok(o)
            }
            Err(e) => {
                error!("Failed to create {} {:?}", T::KIND, e);
                Err(e)
            }
        }
    }

    async fn delete_resource<T: Resource + Meta + DeserializeOwned + Serialize + Clone>(
        &self,
        name: &str,
    ) {
        let api = self.get_api::<T>().await;

        let params = DeleteParams {
            dry_run: false,
            grace_period_seconds: Some(0),
            propagation_policy: Some(PropagationPolicy::Foreground),
            preconditions: None,
        };

        match api.delete(&name, &params).await {
            Ok(o) => {
                if o.is_left() {
                    info!("Deletion of {} {} scheduled", T::KIND, name);
                } else {
                    info!("Deleted {} {}", T::KIND, name);
                }
            }
            Err(e) => {
                error!("Failed to delete {} {:?}", T::KIND, e);
            }
        };
    }

    async fn create_job(&self, session_id: &str, image: &str) -> Result<String> {
        let name = K8sProvisioner::generate_name(&session_id);
        let span = global_tracer().start("Create job");

        let mut job_yaml = load_config("job.yaml");
        job_yaml = replace_config_variable(job_yaml, "job_name", &name);
        job_yaml = replace_config_variable(job_yaml, "session_id", session_id);
        job_yaml = replace_config_variable(job_yaml, "image_name", image);

        trace!("Job YAML {}", job_yaml);

        let job: Job = serde_yaml::from_str(&job_yaml).unwrap();
        let context = TelemetryContext::current_with_span(span);
        let resource = self.create_resource(&job).with_context(context).await?;

        Ok(self.unwrap_uid(resource).unwrap_or_default())
    }

    async fn wait_for_pod(&self, session_id: &str) -> Result<String> {
        let api = self.get_api::<Pod>().await;
        let span = global_tracer().start("Kubernetes internal");

        let labels = format!("web-grid/component=node,web-grid/sessionID={}", session_id);
        let params = ListParams::default().labels(&labels);

        let mut current_state = None;

        let mut stream = api.watch(&params, "0").await?.boxed();
        while let Some(status) = stream.try_next().await? {
            match status {
                WatchEvent::Modified(s) => {
                    let pod_name = Pod::name(&s);

                    if let Some(status) = s.status {
                        let new_state = container_state(&status);

                        if new_state != current_state {
                            if let Some(state) = &new_state {
                                let description = match state {
                                    ContainerState::Running => "Running".to_string(),
                                    ContainerState::Unknown => "Unknown".to_string(),
                                    ContainerState::Terminated => "Terminated".to_string(),
                                    ContainerState::Waiting(reason) => reason.clone(),
                                };

                                span.add_event(
                                    description,
                                    vec![semcov::resource::K8S_POD_NAME.string(pod_name)],
                                );
                            }

                            current_state = new_state;
                        }

                        if !is_pod_ready(&status) {
                            continue;
                        } else if let Some(pod_ip) = status.pod_ip {
                            return Ok(pod_ip);
                        }
                    }
                }
                WatchEvent::Error(s) => error!("Error: {}", s),
                _ => {}
            }
        }

        bail!("Timed out waiting for pod to become ready");
    }

    fn unwrap_uid<T: Resource + Meta + DeserializeOwned + Serialize + Clone>(
        &self,
        resource: T,
    ) -> Option<String> {
        match &Meta::meta(&resource).uid {
            Some(uid) => Some(uid.to_owned()),
            None => None,
        }
    }
}

#[async_trait]
impl Provisioner for K8sProvisioner {
    fn capabilities(&self) -> ProvisionerCapabilities {
        ProvisionerCapabilities {
            platform_name: "linux".to_owned(),
            browsers: self
                .images
                .iter()
                .map(|(_, browser)| browser.to_owned())
                .collect(),
        }
    }

    async fn provision_node(
        &self,
        session_id: &str,
        capabilities: CapabilitiesRequest,
    ) -> Result<NodeInfo> {
        let telemetry_context = TelemetryContext::current();
        let wrapped_image = match_image_from_capabilities(capabilities, &self.images);

        if let Some(image) = wrapped_image {
            telemetry_context
                .span()
                .set_attribute(trace::SESSION_CONTAINER_IMAGE.string(image.clone()));

            self.create_job(&session_id, &image)
                .with_context(telemetry_context.clone())
                .await?;

            let ip = self.wait_for_pod(&session_id).await?;

            Ok(NodeInfo {
                host: ip,
                port: self.node_port.to_string(),
            })
        } else {
            bail!("No matching image found!")
        }
    }

    async fn terminate_node(&self, session_id: &str) {
        let name = K8sProvisioner::generate_name(&session_id);

        // Service will be auto-deleted by K8s garbage collector
        // This requires the ownerReference to be set in the service yaml!
        self.delete_resource::<Job>(&name).await;
    }
}

fn is_pod_ready(status: &PodStatus) -> bool {
    let mut ready = false;

    if let Some(container_statuses) = &status.container_statuses {
        ready = container_statuses
            .iter()
            .fold(ready, |acc, s| acc || s.ready);
    }

    ready
}

#[derive(PartialEq, Eq)]
enum ContainerState {
    Waiting(String),
    Running,
    Terminated,
    Unknown,
}

fn container_state(status: &PodStatus) -> Option<ContainerState> {
    match &status.container_statuses {
        Some(statuses) => statuses
            .first()
            .map(|s| {
                s.state.as_ref().map(|state| {
                    if let Some(_running) = &state.running {
                        ContainerState::Running
                    } else if let Some(waiting) = &state.waiting {
                        ContainerState::Waiting(waiting.reason.clone().unwrap_or_default())
                    } else if let Some(_terminated) = &state.terminated {
                        ContainerState::Terminated
                    } else {
                        ContainerState::Unknown
                    }
                })
            })
            .flatten(),
        None => None,
    }
}
