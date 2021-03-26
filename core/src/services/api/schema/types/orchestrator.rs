use super::super::GQLContext;
use crate::libraries::helpers::keys;
use juniper::{graphql_object, FieldResult, GraphQLObject};
use redis::AsyncCommands;

pub struct Orchestrator {
    id: String,
}

impl Orchestrator {
    pub fn new(id: String) -> Self {
        Self { id }
    }
}

#[graphql_object(context = GQLContext)]
impl Orchestrator {
    pub fn id(&self) -> String {
        self.id.to_owned()
    }

    pub async fn alive(&self, context: &GQLContext) -> FieldResult<bool> {
        Ok(context
            .redis
            .lock()
            .await
            .exists(keys::orchestrator::heartbeat(&self.id))
            .await?)
    }

    pub async fn backlog(&self, context: &GQLContext) -> FieldResult<Vec<String>> {
        Ok(context
            .redis
            .lock()
            .await
            .lrange(keys::orchestrator::backlog(&self.id), 0, -1)
            .await?)
    }

    pub async fn pending(&self, context: &GQLContext) -> FieldResult<Vec<String>> {
        Ok(context
            .redis
            .lock()
            .await
            .lrange(keys::orchestrator::pending(&self.id), 0, -1)
            .await?)
    }

    pub fn slots(&self) -> OrchestratorSlots {
        OrchestratorSlots {
            orchestrator_id: self.id.to_owned(),
        }
    }

    pub fn capabilities(&self) -> OrchestratorCapabilities {
        OrchestratorCapabilities {
            orchestrator_id: self.id.to_owned(),
        }
    }
}

pub struct OrchestratorSlots {
    orchestrator_id: String,
}

#[graphql_object(context = GQLContext)]
impl OrchestratorSlots {
    pub async fn allocated(&self, context: &GQLContext) -> FieldResult<Vec<String>> {
        Ok(context
            .redis
            .lock()
            .await
            .smembers(keys::orchestrator::slots::allocated(&self.orchestrator_id))
            .await?)
    }

    pub async fn available(&self, context: &GQLContext) -> FieldResult<Vec<String>> {
        Ok(context
            .redis
            .lock()
            .await
            .lrange(
                keys::orchestrator::slots::available(&self.orchestrator_id),
                0,
                -1,
            )
            .await?)
    }

    pub async fn reclaimed(&self, context: &GQLContext) -> FieldResult<Vec<String>> {
        Ok(context
            .redis
            .lock()
            .await
            .lrange(
                keys::orchestrator::slots::reclaimed(&self.orchestrator_id),
                0,
                -1,
            )
            .await?)
    }
}

pub struct OrchestratorCapabilities {
    orchestrator_id: String,
}

#[graphql_object(context = GQLContext)]
impl OrchestratorCapabilities {
    pub async fn platformName(&self, context: &GQLContext) -> FieldResult<String> {
        Ok(context
            .redis
            .lock()
            .await
            .get(keys::orchestrator::capabilities::platform_name(
                &self.orchestrator_id,
            ))
            .await?)
    }

    pub async fn browsers(&self, context: &GQLContext) -> FieldResult<Vec<Browser>> {
        let raw_browsers: Vec<String> = context
            .redis
            .lock()
            .await
            .smembers(keys::orchestrator::capabilities::browsers(
                &self.orchestrator_id,
            ))
            .await?;

        Ok(raw_browsers.into_iter().map(Browser::from).collect())
    }
}

#[derive(GraphQLObject)]
struct Browser {
    name: String,
    version: String,
}

impl From<String> for Browser {
    fn from(source: String) -> Self {
        let parts: Vec<String> = source.split("::").map(|s| s.to_owned()).collect();

        Browser {
            name: parts.first().map(|s| s.to_owned()).unwrap_or_default(),
            version: parts.last().map(|s| s.to_owned()).unwrap_or_default(),
        }
    }
}
