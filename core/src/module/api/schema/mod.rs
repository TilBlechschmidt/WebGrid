use juniper::{EmptyMutation, EmptySubscription, RootNode};
use mongodb::Collection;
pub use query::Query;

use crate::domain::SessionMetadata;

mod query;
// mod types;

pub struct GqlContext {
    pub storage_collection: Collection<SessionMetadata>,
    pub staging_collection: Collection<SessionMetadata>,
}

impl juniper::Context for GqlContext {}

pub type Schema =
    RootNode<'static, Query, EmptyMutation<GqlContext>, EmptySubscription<GqlContext>>;

pub fn schema() -> Schema {
    Schema::new(
        Query,
        EmptyMutation::<GqlContext>::new(),
        EmptySubscription::<GqlContext>::new(),
    )
}
