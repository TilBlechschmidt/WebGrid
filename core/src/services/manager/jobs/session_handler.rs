use crate::libraries::tracing::global_tracer;

use super::super::{
    context::SessionCreationContext, tasks, Context, RequestError, SessionReply, SessionReplyError,
    SessionRequest,
};
use anyhow::Result;
use async_trait::async_trait;
use hyper::HeaderMap;
use jatsl::{Job, JobScheduler, TaskManager};
use log::{debug, info, warn};
use opentelemetry::{
    global::{self, BoxedSpan},
    trace::Tracer,
    Context as TelemetryContext,
};
use opentelemetry_http::HeaderExtractor;
use serde_json::json;
use std::net::SocketAddr;
use warp::{http::StatusCode, reply, Filter};

pub struct SessionHandlerJob {
    port: u16,
}

#[async_trait]
impl Job for SessionHandlerJob {
    type Context = Context;

    const NAME: &'static str = module_path!();
    const SUPPORTS_GRACEFUL_TERMINATION: bool = true;

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        let routes = self.routes(manager.clone());

        let source_addr: SocketAddr = ([0, 0, 0, 0], self.port).into();
        let (addr, server) = warp::serve(routes)
            .bind_with_graceful_shutdown(source_addr, manager.termination_signal());

        info!("Listening at {:?}", addr);
        manager.ready().await;

        server.await;

        Ok(())
    }
}

impl SessionHandlerJob {
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    fn routes(
        &self,
        manager: TaskManager<Context>,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let with_manager = warp::any().map(move || manager.clone());

        let with_telemetry_context =
            warp::any()
                .and(warp::header::headers_cloned())
                .map(move |headers: HeaderMap| {
                    let parent_cx = global::get_text_map_propagator(|propagator| {
                        propagator.extract(&HeaderExtractor(&headers))
                    });

                    global_tracer().start_with_context("Dispatch session", parent_cx)
                });

        warp::post()
            .and(warp::path("session"))
            .and(with_manager)
            .and(with_telemetry_context)
            .and(warp::body::json())
            .and(warp::header::<String>("user-agent"))
            .and(warp::addr::remote())
            .and_then(SessionHandlerJob::handle_post_with_telemetry)
    }

    async fn handle_post_with_telemetry(
        manager: TaskManager<Context>,
        span: BoxedSpan,
        request: SessionRequest,
        user_agent: String,
        remote_sock_addr: Option<SocketAddr>,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        global_tracer()
            .with_span(span, |telemetry_context| {
                SessionHandlerJob::handle_post(
                    manager,
                    telemetry_context,
                    request,
                    user_agent,
                    remote_sock_addr,
                )
            })
            .await
    }

    async fn handle_post(
        manager: TaskManager<Context>,
        telemetry_context: TelemetryContext,
        request: SessionRequest,
        user_agent: String,
        remote_sock_addr: Option<SocketAddr>,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        // TODO remote_addr is probably the one of the proxy and not of the client
        let remote_addr = remote_sock_addr
            .map(|i| i.to_string())
            .unwrap_or_else(|| "unknown".to_string());
        let capabilities = request.capabilities.to_string();

        info!(
            "Session creation requested from {}\n{}",
            remote_addr, capabilities
        );

        let session_creation_context = SessionCreationContext::new(
            manager.context.clone(),
            remote_addr,
            user_agent,
            capabilities,
            telemetry_context,
        );

        let task = JobScheduler::spawn_task(&tasks::create_session, session_creation_context);

        let reply_value = match task.await {
            Ok(Ok(val)) => val,
            _ => {
                let e = RequestError::ResourceUnavailable;

                let error = SessionReply {
                    value: json!(SessionReplyError {
                        error: "session not created".to_string(),
                        message: format!("{}", e)
                    }),
                };

                return Ok(reply::with_status(
                    reply::json(&error),
                    StatusCode::INTERNAL_SERVER_ERROR,
                ));
            }
        };

        match reply_value {
            Ok(value) => {
                info!("Created session {}", value.session_id);
                debug!("Resulting capabilities {:?}", value.capabilities);

                let reply = SessionReply {
                    value: json!(value),
                };

                Ok(reply::with_status(reply::json(&reply), StatusCode::CREATED))
            }
            Err(e) => {
                warn!("Failed to create session: {}", e);

                let error = SessionReply {
                    value: json!(SessionReplyError {
                        error: "session not created".to_string(),
                        message: format!("{}", e)
                    }),
                };

                Ok(reply::with_status(
                    reply::json(&error),
                    StatusCode::INTERNAL_SERVER_ERROR,
                ))
            }
        }
    }
}
