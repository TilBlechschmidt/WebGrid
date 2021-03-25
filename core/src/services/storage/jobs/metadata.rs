use super::super::Context;
use crate::libraries::helpers::keys;
use crate::libraries::scheduling::{Job, TaskManager};
use crate::libraries::storage::FileMetadata;
use crate::{
    libraries::resources::{ResourceManager, ResourceManagerProvider},
    with_redis_resource,
};
use anyhow::Result;
use async_trait::async_trait;
use log::debug;
use redis::AsyncCommands;

#[derive(Clone)]
pub struct MetadataJob {}

#[async_trait]
impl Job for MetadataJob {
    type Context = Context;

    const NAME: &'static str = module_path!();

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        let mut redis = with_redis_resource!(manager);
        let storage_id = &manager.context.storage_id;
        let storage = &manager.context.storage;

        manager.ready().await;

        loop {
            let (_, raw_metadata): (String, String) = redis
                .blpop(keys::storage::metadata::pending(storage_id), 0)
                .await?;

            let metadata: FileMetadata = serde_json::from_str(&raw_metadata)?;

            debug!("Adding file from metadata queue: {}", raw_metadata);

            storage.add_file(metadata).await?;
        }
    }
}

impl MetadataJob {
    pub fn new() -> Self {
        Self {}
    }
}
