use orchestrator_core::Options as CoreOptions;
use structopt::StructOpt;

#[cfg(feature = "docker")]
use orchestrator_docker::Options as DockerOptions;
#[cfg(feature = "kubernetes")]
use orchestrator_k8s::Options as K8sOptions;

#[derive(Debug, StructOpt)]
#[cfg(feature = "service_orchestrator")]
// TODO Give it some reasonable description (the one from orchestrator_core won't work :C)
pub struct OrchestratorOptions {
    // TODO Flatten out orchestrator_core options
    #[structopt(flatten)]
    pub core: CoreOptions,

    #[structopt(subcommand)]
    pub provisioner: Provisioner,
}

#[derive(Debug, StructOpt)]
#[cfg(feature = "service_orchestrator")]
pub enum Provisioner {
    #[cfg(feature = "docker")]
    Docker(DockerOptions),
    #[cfg(feature = "kubernetes")]
    Kubernetes(K8sOptions),
}
