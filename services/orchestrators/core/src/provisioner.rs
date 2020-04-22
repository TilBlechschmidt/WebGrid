pub use async_trait::async_trait;
use std::fmt;

#[derive(Debug)]
pub struct NodeInfo {
    pub host: String,
    pub port: String,
}

pub struct ProvisionerCapabilities {
    pub platform_name: String,
    pub browsers: Vec<String>,
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

pub fn split_into_two(input: &str, separator: &'static str) -> Option<(String, String)> {
    let parts: Vec<&str> = input.splitn(2, separator).collect();

    if parts.len() != 2 {
        return None;
    }

    Some((parts[0].to_string(), parts[1].to_string()))
}

pub fn parse_browser_string(input: &str) -> Option<(String, String)> {
    split_into_two(input, "::")
}

/// Parses an environment string that describes images
/// "imageA=browser1::version1,imageB=browser2::version2"
/// => [("imageA", "browser1::version1"), ("imageB", "browser2::version2")]
pub fn parse_images_string(input: String) -> Vec<(String, String)> {
    input
        .split(',')
        .map(|image_string| split_into_two(image_string, "="))
        .filter(|p| p.is_some())
        .map(|p| p.unwrap())
        .collect()
}
