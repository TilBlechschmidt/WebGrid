use crate::harness::HeartStone;
use crate::library::http::Responder;
use async_trait::async_trait;
use futures::Future;
use hyper::body;
use hyper::{
    http::{request::Parts, Method, Response},
    Body,
};
use std::convert::Infallible;
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
    async fn respond<F, Fut>(
        &self,
        parts: Parts,
        body: Body,
        client_ip: IpAddr,
        next: F,
    ) -> Result<Response<Body>, Infallible>
    where
        Fut: Future<Output = Result<Response<Body>, Infallible>> + Send,
        F: FnOnce(Parts, Body, IpAddr) -> Fut + Send,
    {
        let method = &parts.method;
        let path = parts.uri.path();

        let is_session_delete_request = method == Method::DELETE
            && path.eq_ignore_ascii_case(&format!("/session/{}", self.session_id));
        let is_window_delete_request = method == Method::DELETE
            && path.eq_ignore_ascii_case(&format!("/session/{}/window", self.session_id));

        let mut was_last_window = false;

        // Delegate the reply to other responders and spy on their reply
        let reply = match next(parts, body, client_ip).await {
            Ok(reply) => {
                let (reply_parts, reply_body) = reply.into_parts();

                // If processing a window deletion request, disassemble the body and inspect it
                let body = if is_window_delete_request {
                    let body_content = match body::to_bytes(reply_body).await {
                        Ok(bytes) => {
                            String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| "".to_string())
                        }
                        Err(_) => "".into(),
                    };

                    // Check if we deleted the last window
                    was_last_window = body_content.contains("\"value:[]\"")
                        || body_content.contains("\"value: []\"");

                    Body::from(body_content)
                } else {
                    reply_body
                };

                Ok(Response::from_parts(reply_parts, body))
            }
            Err(e) => Err(e),
        };

        // Check if the session is terminating and if so do the same
        let session_closed =
            (is_window_delete_request && was_last_window) || is_session_delete_request;

        if session_closed {
            // TODO This is a MAJOR problem as the termination happens even before the request is forwarded to the browser!
            //      that just ain't gonna work ... on rare occasions (not so rare actually) the whole session tears down before the response is sent.
            self.heart_stone
                .lock()
                .await
                .kill("Session closed by downstream".to_string())
                .await;
        }

        reply
    }
}
