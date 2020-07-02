use helpers::{load_config, replace_config_variable, CapabilitiesRequest, ServicePort};
use orchestrator_core::provisioner::{
    async_trait, match_image_from_capabilities, NodeInfo, Provisioner, ProvisionerCapabilities,
};

use k8s_openapi::api::batch::v1::Job;
use k8s_openapi::api::core::v1::Service;
use serde_yaml;

use kube::{
    api::{Api, DeleteParams, Meta, PostParams, PropagationPolicy},
    error::Error as KubeError,
    Client,
};

use k8s_openapi::Resource;
use log::{error, info, trace, warn};
use serde::{de::DeserializeOwned, ser::Serialize};

#[derive(Clone)]
pub struct K8sProvisioner {
    namespace: String,
    images: Vec<(String, String)>,
}

impl K8sProvisioner {
    pub async fn new(images: Vec<(String, String)>) -> Self {
        if images.is_empty() {
            warn!("No images provided! Orchestrator won't be able to schedule nodes.");
        }

        let namespace = std::env::var("NAMESPACE").unwrap_or_else(|_| "webgrid".into());

        info!("Operating in K8s namespace {}", namespace);

        Self { namespace, images }
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
            grace_period_seconds: None,
            propagation_policy: Some(PropagationPolicy::Foreground),
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

    async fn create_job(&self, session_id: &str, image: &str) -> Result<String, KubeError> {
        let name = K8sProvisioner::generate_name(&session_id);

        let mut job_yaml = load_config("job.yaml");
        job_yaml = replace_config_variable(job_yaml, "job_name", &name);
        job_yaml = replace_config_variable(job_yaml, "session_id", session_id);
        job_yaml = replace_config_variable(job_yaml, "image_name", image);

        trace!("Job YAML {}", job_yaml);

        let job: Job = serde_yaml::from_str(&job_yaml).unwrap();
        let resource = self.create_resource(&job).await?;

        Ok(self.unwrap_uid(resource).unwrap_or_default())
    }

    async fn create_service(&self, session_id: &str, job_uid: &str) -> Result<String, KubeError> {
        let name = K8sProvisioner::generate_name(&session_id);

        let mut service_yaml = load_config("service.yaml");
        service_yaml = replace_config_variable(service_yaml, "job_name", &name);
        service_yaml = replace_config_variable(service_yaml, "service_name", &name);
        service_yaml = replace_config_variable(service_yaml, "job_uid", job_uid);
        service_yaml = replace_config_variable(service_yaml, "session_id", session_id);

        trace!("Service YAML {}", service_yaml);

        let service: Service = serde_yaml::from_str(&service_yaml).unwrap();
        let resource = self.create_resource(&service).await?;

        Ok(self.unwrap_uid(resource).unwrap_or_default())
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
            browsers: Vec::new(),
        }
    }

    async fn provision_node(
        &self,
        session_id: &str,
        capabilities: CapabilitiesRequest,
    ) -> NodeInfo {
        let wrapped_image = match_image_from_capabilities(capabilities, &self.images);

        // TODO Remove this very crude "error handling" with some proper Result<NodeInfo>!
        if let Some(image) = wrapped_image {
            if let Ok(job_uid) = self.create_job(&session_id, &image).await {
                self.create_service(&session_id, &job_uid).await.ok();
            }

            NodeInfo {
                host: K8sProvisioner::generate_name(&session_id),
                port: ServicePort::Node.port().to_string(),
            }
        } else {
            NodeInfo {
                host: "NO_IMAGE_FOUND".to_string(),
                port: "1337".to_string(),
            }
        }
    }

    async fn terminate_node(&self, session_id: &str) {
        let name = K8sProvisioner::generate_name(&session_id);

        // Service will be auto-deleted by K8s garbage collector
        // This requires the ownerReference to be set in the service yaml!
        self.delete_resource::<Job>(&name).await;
    }
}
