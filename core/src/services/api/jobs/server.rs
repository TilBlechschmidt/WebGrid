use super::super::Context;
use crate::libraries::scheduling::{Job, TaskManager};
use anyhow::Result;
use async_trait::async_trait;
use log::info;
use std::net::SocketAddr;
use warp::Filter;

#[derive(Clone)]
pub struct ServerJob {
    port: u16,
}

#[async_trait]
impl Job for ServerJob {
    type Context = Context;

    const NAME: &'static str = module_path!();
    const SUPPORTS_GRACEFUL_TERMINATION: bool = true;

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        let cors = warp::cors()
            .allow_any_origin()
            .allow_methods(vec!["OPTIONS", "GET"])
            .allow_headers(vec!["range"]);

        let hello = warp::any().map(|| "Hello world!");

        let routes = warp::path("api").and(hello).with(cors);

        let source_addr: SocketAddr = ([0, 0, 0, 0], self.port).into();
        let (addr, server) = warp::serve(routes)
            .bind_with_graceful_shutdown(source_addr, manager.termination_signal());

        info!("Listening at {}/api", addr);
        manager.ready().await;

        server.await;

        Ok(())
    }
}

impl ServerJob {
    pub fn new(port: u16) -> Self {
        Self { port }
    }
}
