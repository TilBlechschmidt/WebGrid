use std::collections::HashMap;

use super::{super::GqlContext, Orchestrator};
use crate::libraries::helpers::keys;
use juniper::{
    graphql_object, FieldResult, GraphQLEnum, GraphQLInputObject, GraphQLObject, GraphQLScalarValue,
};
use redis::AsyncCommands;

#[derive(GraphQLEnum)]
pub enum SessionState {
    Active,
    Terminated,
}

#[derive(GraphQLScalarValue)]
pub struct Date(String);

#[derive(GraphQLObject)]
pub struct DictionaryEntry {
    pub key: String,
    pub value: String,
}

#[derive(GraphQLInputObject)]
pub struct InputDictionaryEntry {
    pub key: String,
    pub value: String,
}

#[derive(GraphQLObject)]
pub struct SessionStatusTransitions {
    queued_at: Option<Date>,
    pending_at: Option<Date>,
    alive_at: Option<Date>,
    terminated_at: Option<Date>,
}

impl SessionStatusTransitions {
    pub async fn new(session_id: &str, context: &GqlContext) -> FieldResult<Self> {
        let metadata: HashMap<String, String> = context
            .redis
            .lock()
            .await
            .hgetall(keys::session::status(session_id))
            .await?;

        Ok(Self {
            queued_at: metadata.get("queuedAt").map(|s| Date(s.to_owned())),
            pending_at: metadata.get("pendingAt").map(|s| Date(s.to_owned())),
            alive_at: metadata.get("aliveAt").map(|s| Date(s.to_owned())),
            terminated_at: metadata.get("terminatedAt").map(|s| Date(s.to_owned())),
        })
    }
}

#[derive(GraphQLObject)]
pub struct SessionCapabilities {
    requested: Option<String>,
    actual: Option<String>,
}

impl SessionCapabilities {
    pub async fn new(session_id: &str, context: &GqlContext) -> FieldResult<Self> {
        let metadata: HashMap<String, String> = context
            .redis
            .lock()
            .await
            .hgetall(keys::session::capabilities(session_id))
            .await?;

        Ok(Self {
            requested: metadata.get("requested").map(|s| s.to_owned()),
            actual: metadata.get("actual").map(|s| s.to_owned()),
        })
    }
}

pub struct Session {
    id: String,
}

impl Session {
    pub fn new(session_id: String) -> Self {
        Self { id: session_id }
    }

    async fn storage_id(&self, context: &GqlContext) -> FieldResult<Option<String>> {
        Ok(context
            .redis
            .lock()
            .await
            .get(keys::session::storage(&self.id))
            .await?)
    }

    pub async fn metadata(&self, context: &GqlContext) -> FieldResult<Vec<DictionaryEntry>> {
        let dictionary: Vec<(String, String)> = context
            .redis
            .lock()
            .await
            .hgetall(keys::session::metadata(&self.id))
            .await?;

        Ok(dictionary
            .into_iter()
            .map(|(key, value)| DictionaryEntry { key, value })
            .collect())
    }
}

#[graphql_object(context = GqlContext)]
impl Session {
    fn id(&self) -> &str {
        self.id.as_str()
    }

    async fn status(&self, context: &GqlContext) -> FieldResult<SessionStatusTransitions> {
        SessionStatusTransitions::new(&self.id, context).await
    }

    async fn capabilities(&self, context: &GqlContext) -> FieldResult<SessionCapabilities> {
        SessionCapabilities::new(&self.id, context).await
    }

    async fn metadata(&self, context: &GqlContext) -> FieldResult<Vec<DictionaryEntry>> {
        self.metadata(context).await
    }

    async fn alive(&self, context: &GqlContext) -> FieldResult<bool> {
        Ok(context
            .redis
            .lock()
            .await
            .exists(keys::session::heartbeat::node(&self.id))
            .await?)
    }

    async fn slot(&self, context: &GqlContext) -> FieldResult<Option<String>> {
        Ok(context
            .redis
            .lock()
            .await
            .get(keys::session::slot(&self.id))
            .await?)
    }

    async fn orchestrator(&self, context: &GqlContext) -> FieldResult<Option<Orchestrator>> {
        let key = keys::session::orchestrator(&self.id);
        let orchestrator_id: Option<String> =
            context.redis.lock().await.rpoplpush(&key, &key).await?;

        Ok(orchestrator_id.map(Orchestrator::new))
    }

    async fn storage(&self, context: &GqlContext) -> FieldResult<Option<String>> {
        self.storage_id(context).await
    }

    async fn videoURL(&self, context: &GqlContext) -> FieldResult<Option<String>> {
        Ok(self
            .storage_id(context)
            .await?
            .map(|storage_id| format!("/storage/{}/{}.m3u8", storage_id, &self.id)))
    }
}
