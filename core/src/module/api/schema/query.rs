use crate::domain::event::SessionIdentifier;

use super::types::Session;
use super::GqlContext;
use juniper::{graphql_object, FieldResult};
use mongodb::bson::doc;
use mongodb::options::CountOptions;

pub struct Query;

#[graphql_object(context = GqlContext)]
impl Query {
    async fn session(
        &self,
        id: SessionIdentifier,
        context: &GqlContext,
    ) -> FieldResult<Option<Session>> {
        let filter = doc! { "_id": id };
        let options = CountOptions::builder().limit(1).build();

        let is_stored = context
            .storage_collection
            .count_documents(filter.clone(), options.clone())
            .await?
            > 0;

        let is_staged = context
            .storage_collection
            .count_documents(filter, options)
            .await?
            > 0;

        if !(is_staged || is_stored) {
            Ok(None)
        } else {
            Ok(Some(Session::new(id)))
        }
    }
}
