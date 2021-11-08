use super::types::Session;
use super::GqlContext;
use domain::event::SessionIdentifier;
use futures::TryStreamExt;
use juniper::{graphql_object, FieldResult, GraphQLInputObject};
use mongodb::bson::doc;
use mongodb::options::FindOptions;
use tracing::debug;

pub struct Query;
struct SessionQuery {}

#[graphql_object(context = GqlContext)]
impl Query {
    /// Set of queries that return `Session` objects
    fn session(&self) -> SessionQuery {
        SessionQuery {}
    }
}

#[graphql_object(context = GqlContext)]
impl SessionQuery {
    /// Fetch the latest `count` sessions that have terminated. Sorted in descending order based on the creation date.
    async fn latest(count: Option<i32>, context: &GqlContext) -> FieldResult<Vec<Session>> {
        let limit = count.unwrap_or(10) as i64;
        debug!(limit, "QUERY latest");

        let options = FindOptions::builder()
            .limit(limit)
            .sort(doc! { "createdAt": -1 })
            .build();

        Ok(context
            .storage_collection
            .find(None, options)
            .await?
            .map_ok(Session::new)
            .try_collect::<Vec<_>>()
            .await?)
    }

    /// Fetch a single session with the given `id`
    async fn fetch(
        &self,
        id: SessionIdentifier,
        context: &GqlContext,
    ) -> FieldResult<Option<Session>> {
        debug!(?id, "QUERY fetch");
        let filter = doc! { "_id": id };

        let stored_session = context
            .storage_collection
            .find_one(filter.clone(), None)
            .await?;

        let staged_session = context.staging_collection.find_one(filter, None).await?;

        if let Some(session) = stored_session.or(staged_session) {
            Ok(Some(Session::new(session)))
        } else {
            Ok(None)
        }
    }

    /// Query for sessions based on metadata properties set by the client.
    /// Fields are combined with the `AND` operator.
    async fn query(
        &self,
        fields: Vec<MetadataQuery>,
        limit: Option<i32>,
        context: &GqlContext,
    ) -> FieldResult<Vec<Session>> {
        debug!(?fields, ?limit, "QUERY fieldQuery");
        let limit = limit.unwrap_or(10) as i64;
        let options = FindOptions::builder().limit(limit).build();

        let mut filter = doc! {};
        fields.into_iter().for_each(|field| {
            filter.insert(
                format!("clientMetadata.{}", field.key),
                doc! { "$regex": field.regex },
            );
        });

        Ok(context
            .storage_collection
            .find(filter, options)
            .await?
            .map_ok(Session::new)
            .try_collect::<Vec<_>>()
            .await?)
    }
}

/// Field entry used when querying. The value is parsed as a regular expression.
#[derive(GraphQLInputObject, Debug)]
struct MetadataQuery {
    key: String,
    regex: String,
}
