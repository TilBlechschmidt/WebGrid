//! Provisioner for new session nodes

use structopt::StructOpt;

mod core;
pub mod provisioners;

#[cfg(feature = "docker")]
use provisioners::docker::Options as DockerOptions;

#[cfg(feature = "kubernetes")]
use provisioners::kubernetes::Options as K8sOptions;

#[cfg(feature = "local")]
use provisioners::local::Options as LocalOptions;

#[derive(Debug, StructOpt)]
// TODO Give it some reasonable description (the one from orchestrator_core won't work :C)
pub struct Options {
    // TODO Flatten out orchestrator_core options
    #[structopt(flatten)]
    pub core: core::Options,

    #[structopt(subcommand)]
    pub provisioner: Provisioner,
}

#[derive(Debug, StructOpt)]
pub enum Provisioner {
    #[cfg(feature = "docker")]
    Docker(DockerOptions),
    #[cfg(feature = "kubernetes")]
    Kubernetes(K8sOptions),
    #[cfg(feature = "local")]
    Local(LocalOptions),
}
