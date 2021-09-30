use super::super::GqlContext;
use crate::domain::event::SessionIdentifier;
use juniper::graphql_object;

pub struct Session {
    id: SessionIdentifier,
}

impl Session {
    pub fn new(session_id: SessionIdentifier) -> Self {
        Self { id: session_id }
    }
}

#[graphql_object(context = GqlContext)]
impl Session {
    fn id(&self) -> String {
        self.id.to_string()
    }

    fn video_url(&self) -> String {
        format!("/storage/{}/screen.m3u8", &self.id)
    }
}
