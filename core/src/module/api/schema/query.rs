use super::GqlContext;
use juniper::{graphql_object, FieldResult};

pub struct Query;

#[graphql_object(context = GqlContext)]
impl Query {
    fn something(&self) -> FieldResult<String> {
        Ok("hello world".into())
    }
}
