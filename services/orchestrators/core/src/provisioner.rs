pub use async_trait::async_trait;
use std::fmt;

#[derive(Debug)]
pub struct NodeInfo {
    pub host: String,
    pub port: String,
}

#[async_trait]
pub trait Provisioner {
    async fn provision_node(&self, session_id: &str) -> NodeInfo;
    async fn terminate_node(&self, session_id: &str);
}

#[derive(Debug)]
pub enum Type {
    Local,
    Docker,
    K8s,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}
