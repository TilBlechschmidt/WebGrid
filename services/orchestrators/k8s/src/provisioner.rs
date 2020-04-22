<<<<<<< HEAD
use shared::{load_config, replace_config_variable};
use orchestrator_core::provisioner::{async_trait, NodeInfo, Provisioner};
=======
use orchestrator_core::provisioner::{async_trait, NodeInfo, Provisioner, ProvisionerCapabilities};
use shared::{load_config, replace_config_variable};
>>>>>>> 7049028... :wrench: Replace validate script with git hooks

use k8s_openapi::api::batch::v1::Job;
use k8s_openapi::api::core::v1::Service;
use serde_yaml;

use kube::{
    api::{Api, DeleteParams, Meta, PostParams, PropagationPolicy},
    Client,
};

use k8s_openapi::Resource;
use log::{error, info, trace};
use serde::{de::DeserializeOwned, ser::Serialize};

#[derive(Clone)]
pub struct K8sProvisioner {
    namespace: String,
}

impl K8sProvisioner {
    pub async fn new() -> Self {
        let namespace = std::env::var("NAMESPACE").unwrap_or_else(|_| "webgrid".into());

        info!("Operating in K8s namespace {}", namespace);

        Self { namespace }
    }

    fn generate_name(session_id: &str) -> String {
        format!("session-{}", session_id)
    }

    async fn get_api<T: Resource>(&self) -> Api<T> {
        let client = Client::try_default().await.unwrap();
        Api::namespaced(client, &self.namespace)
    }

    async fn create_resource<T: Resource + Meta + DeserializeOwned + Serialize + Clone>(
        &self,
        value: &T,
    ) {
        let api = self.get_api::<T>().await;

        match api.create(&PostParams::default(), value).await {
            Ok(o) => {
                let name = Meta::name(&o);
                info!("Created {} {}", T::KIND, name);
            }
            Err(e) => {
                error!("Failed to create {} {:?}", T::KIND, e);
            }
        };
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

    async fn create_job(&self, session_id: &str) {
        let name = K8sProvisioner::generate_name(&session_id);

        let mut job_yaml = load_config("job.yaml");
        job_yaml = replace_config_variable(job_yaml, "job_name", &name);
        job_yaml = replace_config_variable(job_yaml, "session_id", session_id);

        trace!("Job YAML {}", job_yaml);

        let job: Job = serde_yaml::from_str(&job_yaml).unwrap();
        self.create_resource(&job).await;
    }

    async fn create_service(&self, session_id: &str) {
        let name = K8sProvisioner::generate_name(&session_id);

        let mut service_yaml = load_config("service.yaml");
        service_yaml = replace_config_variable(service_yaml, "job_name", &name);
        service_yaml = replace_config_variable(service_yaml, "service_name", &name);

        trace!("Service YAML {}", service_yaml);

        let service: Service = serde_yaml::from_str(&service_yaml).unwrap();
        self.create_resource(&service).await;
    }
}

#[async_trait]
impl Provisioner for K8sProvisioner {
<<<<<<< HEAD
=======
    fn capabilities(&self) -> ProvisionerCapabilities {
        ProvisionerCapabilities {
            platform_name: "linux".to_owned(),
            browsers: Vec::new(),
        }
    }

>>>>>>> 7049028... :wrench: Replace validate script with git hooks
    async fn provision_node(&self, session_id: &str) -> NodeInfo {
        let name = K8sProvisioner::generate_name(&session_id);

        self.create_job(&session_id).await;
        self.create_service(&session_id).await;

        NodeInfo {
            host: name,
            port: "3030".to_string(),
        }
    }

    async fn terminate_node(&self, session_id: &str) {
        let name = K8sProvisioner::generate_name(&session_id);

        self.delete_resource::<Service>(&name).await;
        self.delete_resource::<Job>(&name).await;
    }
}
