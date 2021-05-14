use super::super::Context;
use anyhow::Result;
use async_trait::async_trait;
use jatsl::{Job, TaskManager};
use log::{debug, info};
use std::{net::SocketAddr, path::PathBuf};
use warp::{reply::Reply, Filter};

pub struct ServerJob {
    port: u16,
    storage_directory: PathBuf,
}

#[async_trait]
impl Job for ServerJob {
    type Context = Context;

    const NAME: &'static str = module_path!();
    const SUPPORTS_GRACEFUL_TERMINATION: bool = true;

    async fn execute(&self, manager: TaskManager<Self::Context>) -> Result<()> {
        // Path matched to path at proxy:
        // /storage/<storage-id>/<...filepath>
        // localhost:40006/storage/5775e524-7626-497d-9adb-5121ed5ab08f/bbc5dce5-30ff-41cd-877b-45459829c187.m3u8

        let cors = warp::cors()
            .allow_any_origin()
            .allow_methods(vec!["OPTIONS", "GET"])
            .allow_headers(vec!["range"]);

        let storage = warp::path(manager.context.storage_id.to_string())
            .and(warp::fs::dir(self.storage_directory.clone()))
            .map(|reply: warp::filters::fs::File| {
                debug!("SERVE {}", reply.path().display());
                if let Some(extension) = reply.path().extension() {
                    if extension == "m3u8" {
                        warp::reply::with_header(
                            reply,
                            "Content-Type",
                            "application/vnd.apple.mpegURL",
                        )
                        .into_response()
                    } else if extension == "m4s" {
                        warp::reply::with_header(reply, "Content-Type", "video/mp4").into_response()
                    } else {
                        reply.into_response()
                    }
                } else {
                    reply.into_response()
                }
            });

        let routes = warp::path("storage").and(storage).with(cors);

        let source_addr: SocketAddr = ([0, 0, 0, 0], self.port).into();
        let (addr, server) = warp::serve(routes)
            .bind_with_graceful_shutdown(source_addr, manager.termination_signal());

        info!(
            "Listening at {}/storage/{}",
            addr, manager.context.storage_id
        );
        manager.ready().await;

        server.await;

        Ok(())
    }
}

impl ServerJob {
    pub fn new(port: u16, storage_directory: PathBuf) -> Self {
        Self {
            port,
            storage_directory,
        }
    }
}
