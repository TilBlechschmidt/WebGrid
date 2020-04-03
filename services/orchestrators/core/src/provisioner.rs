pub use async_trait::async_trait;

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
