use super::super::super::core::provisioner::{
    match_image_from_capabilities, Provisioner, ProvisionerCapabilities,
};
use crate::libraries::{
    helpers::{load_config, replace_config_variable, CapabilitiesRequest},
    tracing::{constants::trace, global_tracer},
};
use anyhow::{bail, Result};
use async_trait::async_trait;
use k8s_openapi::{api::batch::v1::Job, Resource};
use kube::{
    api::{Api, DeleteParams, Meta, PostParams, PropagationPolicy},
    error::Error as KubeError,
    Client,
};
use log::{error, info, trace, warn};
use opentelemetry::{
    trace::{FutureExt, TraceContextExt, Tracer},
    Context as TelemetryContext,
};
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

        Ok(Meta::meta(&resource)
            .uid
            .as_ref()
            .map(|uid| uid.to_owned())
            .unwrap_or_default())
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
    ) -> Result<()> {
        let telemetry_context = TelemetryContext::current();
        let wrapped_image = match_image_from_capabilities(capabilities, &self.images);

        if let Some(image) = wrapped_image {
            telemetry_context
                .span()
                .set_attribute(trace::SESSION_CONTAINER_IMAGE.string(image.clone()));

            self.create_job(&session_id, &image)
                .with_context(telemetry_context.clone())
                .await?;

            Ok(())
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
