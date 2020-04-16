use orchestrator_core::provisioner::{async_trait, NodeInfo, Provisioner};

use k8s_openapi::api::batch::v1::Job;
use k8s_openapi::api::core::v1::Service;
use serde_json::json;

use kube::{
    api::{Api, DeleteParams, Meta, PostParams, PropagationPolicy},
    Client,
};

use k8s_openapi::Resource;
use log::{error, info};
use serde::{de::DeserializeOwned, ser::Serialize};

#[derive(Clone)]
pub struct K8sProvisioner {
    client: Client,
    namespace: String,
}

impl K8sProvisioner {
    pub async fn new() -> Self {
        let client = Client::infer().await.unwrap();
        let namespace = std::env::var("NAMESPACE").unwrap_or_else(|_| "webgrid".into());

        info!("Operating in K8s namespace {}", namespace);

        Self { client, namespace }
    }

    fn generate_name(session_id: &str) -> String {
        format!("session-{}", session_id)
    }

    fn get_api<T: Resource>(&self) -> Api<T> {
        Api::namespaced(self.client.clone(), &self.namespace)
    }

    async fn create_resource<T: Resource + Meta + DeserializeOwned + Serialize + Clone>(
        &self,
        value: &T,
    ) {
        let api = self.get_api::<T>();

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
        let api = self.get_api::<T>();

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

        let job: Job = serde_json::from_value(json!({
            "apiVersion": "batch/v1",
            "kind": "Job",
            "metadata": {
                "name": name,
                "labels": {
                    "app": "webgrid-node",
                    "sessionID": &session_id
                }
            },
            "spec": {
                "backoffLimit": 1,
                "template": {
                    "metadata": { "labels": { "app": name } },
                    "spec": {
                        "restartPolicy": "Never",
                        "volumes": [
                            { "name": "dshm", "emptyDir": { "medium": "Memory" } }
                        ],
                        "containers": [{
                            "name": name,
                            "image": "registry.blechschmidt.de/webgrid-node:firefox",
                            "imagePullPolicy": "Always",
                            "ports": [
                                { "containerPort": 3030 }
                            ],
                            "env": [
                                { "name": "WEBGRID_SESSION_ID", "value": &session_id },
                                { "name": "WEBGRID_REDIS_URL", "value": "redis://redis/" }
                            ],
                            "volumeMounts": [
                                { "mountPath": "/dev/shm", "name": "dshm" }
                            ]
                        }],
                    }
                }
            }

        }))
        .unwrap();

        self.create_resource(&job).await;
    }

    async fn create_service(&self, session_id: &str) {
        let name = K8sProvisioner::generate_name(&session_id);

        let service: Service = serde_json::from_value(json!({
            "apiVersion": "v1",
            "kind": "Service",
            "metadata": {
                "name": name
            },
            "spec": {
                "ports": [{ "port": 3030, "targetPort": 3030, "protocol": "TCP" }],
                "selector": { "app": name }
            }
        }))
        .unwrap();

        self.create_resource(&service).await;
    }
}

#[async_trait]
impl Provisioner for K8sProvisioner {
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
