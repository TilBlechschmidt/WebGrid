use std::path::PathBuf;

use crate::libraries::resources::{DefaultResourceManager, ResourceManagerProvider};

use super::Options;

#[derive(Clone)]
pub struct Context {
    pub web_root: PathBuf,
    resource_manager: DefaultResourceManager,
}

impl Context {
    pub async fn new(options: &Options, redis_url: String) -> Self {
        Self {
            resource_manager: DefaultResourceManager::new(redis_url),
            web_root: options.web_root.clone(),
        }
    }
}

impl ResourceManagerProvider<DefaultResourceManager> for Context {
    fn resource_manager(&self) -> DefaultResourceManager {
        self.resource_manager.clone()
    }
}
