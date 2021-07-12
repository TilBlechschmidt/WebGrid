use crate::harness::HeartStone;
use crate::library::http::{Responder, ResponderResult};
use async_trait::async_trait;
use hyper::{
    http::{request::Parts, Method},
    Body,
};
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct TerminationInterceptor {
    heart_stone: Arc<Mutex<HeartStone>>,
    session_id: String,
}

impl TerminationInterceptor {
    pub fn new(heart_stone: HeartStone, session_id: String) -> Self {
        Self {
            heart_stone: Arc::new(Mutex::new(heart_stone)),
            session_id,
        }
    }
}

#[async_trait]
impl Responder for TerminationInterceptor {
    #[inline]
    async fn respond(&self, parts: Parts, body: Body, client_ip: IpAddr) -> ResponderResult {
        let method = &parts.method;
        let path = parts.uri.path();

        let is_session_delete_request = method == Method::DELETE
            && path.eq_ignore_ascii_case(&format!("/session/{}", self.session_id));
        let is_window_delete_request = method == Method::DELETE
            && path.eq_ignore_ascii_case(&format!("/session/{}/window", self.session_id));

        // TODO When deleting a window, it is possible that there are still others left over in which case we should not terminate!
        let session_closed = is_window_delete_request || is_session_delete_request;

        if session_closed {
            self.heart_stone
                .lock()
                .await
                .kill("Session closed by downstream".to_string())
                .await;
        }

        ResponderResult::Continue(parts, body, client_ip)
    }
}
