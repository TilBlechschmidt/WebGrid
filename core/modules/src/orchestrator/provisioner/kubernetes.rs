use super::SessionProvisioner;
use super::{CONTAINER_SESSION_ID_LABEL, PROVISIONER_INSTANCE_LABEL};
use async_trait::async_trait;
use domain::container::ContainerImageSet;
use domain::event::{ProvisionedSessionMetadata, SessionIdentifier};
use domain::webdriver::RawCapabilitiesRequest;
use k8s_openapi::{api::batch::v1::Job, Resource};
use kube::api::{DeleteParams, ListParams, PropagationPolicy};
use kube::{
    api::{Api, PostParams, Resource as KubeResource, ResourceExt},
    error::Error as KubeError,
    Client,
};
use library::helpers::{load_config, replace_config_variable};
use library::{BoxedError, EmptyResult};
use serde::{de::DeserializeOwned, ser::Serialize};
use std::fmt::Debug;
use std::str::FromStr;
use thiserror::Error;
use tracing::{debug, error, trace, warn};
use uuid::Uuid;

#[derive(Error, Debug)]
enum KubernetesProvisionerError {
    #[error("no matching image found")]
    NoImageFound,

    #[error("create kubernetes job failed")]
    KubernetesError(#[from] KubeError),

    #[error("job template unreadable")]
    JobTemplateUnreadable(#[from] std::io::Error),

    #[error("invalid job template yml")]
    JobTemplateInvalid(#[from] serde_yaml::Error),

    #[error("invalid capabilities")]
    InvalidCapabilities(#[from] serde_json::Error),
}

/// Implementation based on [Kubernetes Jobs](https://kubernetes.io/docs/concepts/workloads/controllers/job/)
pub struct KubernetesProvisioner {
    namespace: String,
    images: ContainerImageSet,
    instance: String,
}

impl KubernetesProvisioner {
    /// Creates a new instance with the provided images, connecting to the default API endpoint drawn from the environment.
    /// By default, it uses the `webgrid` namespace unless the `NAMESPACE` variable is set (which it is by default in K8s pods).
    pub fn new(images: ContainerImageSet, instance: String) -> Self {
        if images.is_empty() {
            warn!("No images provided to provisioner. It won't be able to launch any sessions!");
        }

        let namespace = std::env::var("NAMESPACE").unwrap_or_else(|_| "webgrid".into());

        Self {
            namespace,
            images,
            instance,
        }
    }

    fn generate_name(session_id: &SessionIdentifier) -> String {
        let prefix = std::env::var("WEBGRID_RESOURCE_PREFIX").unwrap_or_default();
        let short_id = &session_id.to_string()[..8];
        format!("{}session-{}", prefix, short_id)
    }

    async fn get_api<T: Resource + Debug + KubeResource>(&self) -> Result<Api<T>, KubeError>
    where
        <T as KubeResource>::DynamicType: Default,
    {
        let client = Client::try_default().await?;
        Ok(Api::namespaced(client, &self.namespace))
    }

    async fn list_jobs(&self, filter: fn(&Job) -> bool) -> Result<Vec<Job>, BoxedError> {
        let api = self.get_api::<Job>().await?;
        let label_filter = format!("{}={}", PROVISIONER_INSTANCE_LABEL, self.instance);

        let params = ListParams::default().labels(&label_filter);
        let jobs = api.list(&params).await?;

        Ok(jobs.into_iter().filter(filter).collect())
    }

    async fn create_resource<
        T: Resource + KubeResource + DeserializeOwned + Serialize + Clone + Debug,
    >(
        &self,
        value: &T,
    ) -> Result<T, KubeError>
    where
        <T as KubeResource>::DynamicType: Default,
    {
        let api = self.get_api::<T>().await?;

        match api.create(&PostParams::default(), value).await {
            Ok(o) => {
                let name = ResourceExt::name(&o);
                trace!(resource = ?T::KIND, ?name, "Created resource");
                Ok(o)
            }
            Err(e) => {
                error!(resource = ?T::KIND, ?e, "Failed to create resource");
                Err(e)
            }
        }
    }

    async fn delete_resource<
        T: Resource + KubeResource + DeserializeOwned + Serialize + Clone + Debug,
    >(
        &self,
        name: &str,
    ) -> Result<(), KubeError>
    where
        <T as KubeResource>::DynamicType: Default,
    {
        let api = self.get_api::<T>().await?;

        let params = DeleteParams {
            dry_run: false,
            grace_period_seconds: Some(0),
            propagation_policy: Some(PropagationPolicy::Foreground),
            preconditions: None,
        };

        match api.delete(name, &params).await {
            Ok(o) => {
                if o.is_left() {
                    trace!(resource = ?T::KIND, ?name, "Deletion of resource scheduled");
                } else {
                    trace!(resource = ?T::KIND, ?name, "Deleted resource");
                }
            }
            Err(e) => {
                error!(resource = ?T::KIND, ?e, "Failed to delete resource");
            }
        };

        Ok(())
    }

    async fn create_job(
        &self,
        session_id: &SessionIdentifier,
        raw_capabilities: &RawCapabilitiesRequest,
    ) -> Result<ProvisionedSessionMetadata, KubernetesProvisionerError> {
        let request = raw_capabilities.parse()?;
        let image = self
            .images
            .match_against_capabilities(request)
            .ok_or(KubernetesProvisionerError::NoImageFound)?;

        debug!(?session_id, image = ?image.identifier, browser = ?image.browser, "Creating job");

        let name = Self::generate_name(session_id);
        let mut job_yaml = load_config("job.yaml")?;
        job_yaml = replace_config_variable(job_yaml, "job_name", &name);
        job_yaml = replace_config_variable(job_yaml, "session_id", &session_id.to_string());
        job_yaml = replace_config_variable(job_yaml, "image_name", &image.identifier);
        job_yaml =
            replace_config_variable(job_yaml, "provisioner_instance", &self.instance.to_string());
        job_yaml = replace_config_variable(job_yaml, "capabilities", raw_capabilities.as_str());

        trace!("Job YAML {}", job_yaml);

        let job: Job = serde_yaml::from_str(&job_yaml)?;
        let _resource = self.create_resource(&job).await?;

        // TODO Append more meaningful information
        Ok(ProvisionedSessionMetadata::new())
    }
}

#[async_trait]
impl SessionProvisioner for KubernetesProvisioner {
    async fn provision(
        &self,
        session_id: &SessionIdentifier,
        raw_capabilities: &RawCapabilitiesRequest,
    ) -> Result<ProvisionedSessionMetadata, BoxedError> {
        Ok(self.create_job(session_id, raw_capabilities).await?)
    }

    /// Returns the session identifier of all jobs which have not reached either a Failed or Success state
    ///
    /// The rationale behind this is that some randomly failed Job would poison the list of managed sessions
    /// and block a permit. To prevent this, failed sessions are counted as "not alive", too. However,
    /// in a full-on system failure scenario where every Job will fail, it would create more and more resources
    /// which are all in a failed state. While it is reasonable to keep failed items for debugging by the K8s admin,
    /// DDoS-ing the cluster with dead resources isn't nice either. For more details see the `purge_terminated` method.
    async fn alive_sessions(&self) -> Result<Vec<SessionIdentifier>, BoxedError> {
        let running_jobs = self.list_jobs(JobExt::is_running).await?;

        let alive_session_ids = running_jobs
            .iter()
            .filter_map(|job| {
                job.meta()
                    .labels
                    .get(CONTAINER_SESSION_ID_LABEL)
                    .and_then(|id| {
                        Uuid::from_str(id)
                            .map_err(|e| {
                                warn!(?id, ?e, "Failed to parse session id from container label",)
                            })
                            .ok()
                    })
            })
            .collect();

        Ok(alive_session_ids)
    }

    /// Purges all successful jobs. When more than 10 failed jobs have accumulated, a warning is presented.
    /// As this number surpasses 100, failed jobs will be deleted until their count is below 50. This ensures
    /// that we do not accumulate an infinite amount of K8s resource objects.
    async fn purge_terminated(&self) -> EmptyResult {
        // Delete successful jobs
        let successful_jobs = self.list_jobs(JobExt::is_successful).await?;

        for job in successful_jobs.into_iter() {
            self.delete_resource::<Job>(&job.name()).await?;
        }

        // Handle failed jobs
        let failed_jobs = self.list_jobs(JobExt::has_failed).await?;

        if failed_jobs.len() > 100 {
            error!("Detected an unreasonably high number of failed Jobs! Purging some of them to keep K8s smooth — note that this hints at a critical error either in the WebGrid node or your infrastructure. This warrants triaging as it will cause problems for downstream consumers!");

            let amount_to_purge = failed_jobs.len() - 50;
            for job in failed_jobs.into_iter().take(amount_to_purge) {
                self.delete_resource::<Job>(&job.name()).await?;
            }
        } else if failed_jobs.len() > 10 {
            warn!("Detected an increasing number of failed Jobs. The resources will not be cleaned up yet, so please check for the root cause");
        }

        Ok(())
    }
}

// TODO Write tests for the K8s provisioner (using a dummy image and checking with the API)

/// Helper methods for the Job type
/// Note that these methods *expect* the job to only ever have one Pod in its lifetime!
trait JobExt {
    fn is_running(&self) -> bool;
    fn is_successful(&self) -> bool;
    fn has_failed(&self) -> bool;
}

impl JobExt for Job {
    fn is_successful(&self) -> bool {
        if let Some(status) = &self.status {
            match status.succeeded {
                Some(count) => count > 0,
                None => false,
            }
        } else {
            false
        }
    }

    fn has_failed(&self) -> bool {
        if let Some(status) = &self.status {
            match status.failed {
                Some(count) => count > 0,
                None => false,
            }
        } else {
            false
        }
    }

    fn is_running(&self) -> bool {
        !(self.is_successful() || self.has_failed())
    }
}
