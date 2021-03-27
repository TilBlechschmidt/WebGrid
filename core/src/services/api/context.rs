use std::path::PathBuf;

use crate::libraries::{
    helpers::keys,
    lifecycle::{BeatValue, HeartBeat},
    resources::{DefaultResourceManager, ResourceManagerProvider},
};

use super::Options;

#[derive(Clone)]
pub struct Context {
    pub heart_beat: HeartBeat<Self, DefaultResourceManager>,
    pub api_id: String,
    pub web_root: PathBuf,
    resource_manager: DefaultResourceManager,
}

impl Context {
    pub async fn new(options: &Options, redis_url: String, api_id: &str) -> Self {
        let addr = format!("{}:{}", options.host, options.port);
        let heart_beat = HeartBeat::with_value(BeatValue::Constant(addr));

        heart_beat.add_beat(&keys::api::host(api_id), 60, 120).await;

        Self {
            heart_beat,
            api_id: api_id.to_owned(),
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
