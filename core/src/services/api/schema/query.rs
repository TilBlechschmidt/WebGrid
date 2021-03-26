use super::{
    types::{InputDictionaryEntry, Orchestrator, Session, SessionState, Timeouts},
    GQLContext,
};
use crate::libraries::helpers::keys;
use juniper::{graphql_object, FieldResult};
use redis::AsyncCommands;

pub struct Query;

impl Query {
    async fn sessions_with_state(
        state: SessionState,
        context: &GQLContext,
    ) -> FieldResult<Vec<Session>> {
        let key = match state {
            SessionState::Active => &*keys::session::LIST_ACTIVE,
            SessionState::Terminated => &*keys::session::LIST_TERMINATED,
        };

        let session_ids: Vec<String> = context.redis.lock().await.smembers(key).await?;

        Ok(session_ids.into_iter().map(Session::new).collect())
    }
}

#[graphql_object(context = GQLContext)]
impl Query {
    fn timeouts() -> Timeouts {
        Timeouts::new()
    }

    async fn sessions(
        &self,
        state: Option<SessionState>,
        metadata: Option<Vec<InputDictionaryEntry>>,
        context: &GQLContext,
    ) -> FieldResult<Vec<Session>> {
        let sessions = if let Some(state) = state {
            Query::sessions_with_state(state, context).await?
        } else {
            let mut active = Query::sessions_with_state(SessionState::Active, context).await?;
            let mut terminated =
                Query::sessions_with_state(SessionState::Terminated, context).await?;

            active.append(&mut terminated);

            active
        };

        if let Some(expected_metadata) = metadata {
            let mut filtered_sessions = Vec::with_capacity(sessions.len());

            for session in sessions.into_iter() {
                let session_metadata = session.metadata(context).await.unwrap_or_default();

                // Check if every element in expected_metadata can be found in session_metadata
                // by converting expected_metadata to an array of booleans describing the match.
                let unmatched_metadata_count = expected_metadata
                    .iter()
                    .map(|expected| {
                        session_metadata.iter().rfold(false, |acc, entry| {
                            acc || (expected.key == entry.key && expected.value == entry.value)
                        })
                    })
                    .filter(|m| !*m)
                    .count();

                if unmatched_metadata_count == 0 {
                    filtered_sessions.push(session);
                }
            }

            Ok(filtered_sessions)
        } else {
            Ok(sessions)
        }
    }

    async fn session(id: String, context: &GQLContext) -> FieldResult<Option<Session>> {
        let is_active: bool = context
            .redis
            .lock()
            .await
            .sismember(&*keys::session::LIST_ACTIVE, &id)
            .await?;

        let is_terminated: bool = context
            .redis
            .lock()
            .await
            .sismember(&*keys::session::LIST_TERMINATED, &id)
            .await?;

        if !(is_active || is_terminated) {
            Ok(None)
        } else {
            Ok(Some(Session::new(id)))
        }
    }

    async fn orchestrators(context: &GQLContext) -> FieldResult<Vec<Orchestrator>> {
        let orchestrator_ids: Vec<String> = context
            .redis
            .lock()
            .await
            .smembers(&*keys::orchestrator::LIST)
            .await?;

        Ok(orchestrator_ids
            .into_iter()
            .map(Orchestrator::new)
            .collect())
    }

    async fn orchestrator(id: String, context: &GQLContext) -> FieldResult<Option<Orchestrator>> {
        let exists: bool = context
            .redis
            .lock()
            .await
            .sismember(&*keys::orchestrator::LIST, &id)
            .await?;

        if !exists {
            Ok(None)
        } else {
            Ok(Some(Orchestrator::new(id)))
        }
    }
}
