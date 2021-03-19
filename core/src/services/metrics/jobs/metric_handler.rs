use super::super::data_collector::*;
use super::super::Context;
use crate::libraries::resources::{ResourceManager, ResourceManagerProvider};
use crate::libraries::scheduling::{Job, TaskManager};
use crate::with_redis_resource;
use anyhow::Result;
use async_trait::async_trait;
use log::info;
use std::net::SocketAddr;
use warp::Filter;

#[derive(Clone)]
pub struct MetricHandlerJob {
    port: u16,
}

#[async_trait]
impl Job for MetricHandlerJob {
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

impl MetricHandlerJob {
    pub fn new(port: u16) -> Self {
        Self { port }
    }

    fn routes(
        &self,
        manager: TaskManager<Context>,
    ) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
        let with_manager = warp::any().map(move || manager.clone());

        warp::get()
            .and(warp::path("metrics"))
            .and(with_manager)
            .and_then(MetricHandlerJob::handle_get)
    }

    async fn handle_get(
        manager: TaskManager<Context>,
    ) -> Result<impl warp::Reply, warp::Rejection> {
        let mut con = with_redis_resource!(manager);

        #[allow(clippy::eval_order_dependence)]
        let metrics: Vec<String> = vec![
            proxy_requests(&mut con).await,
            proxy_traffic(&mut con).await,
            session_log(&mut con).await,
            session_startup_duration(&mut con).await,
            // TODO Replace later with session_total{stage="queued|pending|alive|terminated"} counter
            sessions_active(&mut con).await,
            sessions_terminated(&mut con).await,
            slots_available(&mut con).await,
            slots_total(&mut con).await,
            storage_capacity(&mut con).await,
            storage_usage(&mut con).await,
        ]
        .iter()
        .map(|metric| format!("{}", metric))
        .collect();

        Ok(warp::reply::with_status(
            metrics.join("\n"),
            warp::http::StatusCode::OK,
        ))
    }
}
