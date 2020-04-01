#[macro_use]
extern crate lazy_static;

use serde_json::json;
use warp::Filter;

use redis::{AsyncCommands, RedisResult};
use std::net::SocketAddr;
use std::sync::Arc;

mod config;
mod session;
mod structures;
mod context;

use crate::context::Context;
use crate::session::handle_create_session_request;
use crate::structures::{SessionReply, SessionReplyError, SessionRequest};

async fn handle_post(
    ctx: Arc<Context>,
    request: SessionRequest,
    user_agent: String,
    remote_sock_addr: Option<SocketAddr>,
) -> Result<impl warp::Reply, warp::Rejection> {
    // TODO remote_addr is probably the one of the proxy and not of the client
    let remote_addr = remote_sock_addr
        .map(|i| i.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let capabilities = request.capabilities.to_string();

    let reply_value =
        handle_create_session_request(ctx.clone(), &remote_addr, &user_agent, &capabilities);

    match reply_value.await {
        Ok(value) => {
            let reply = SessionReply {
                value: json!(value),
            };

            Ok(warp::reply::with_status(
                warp::reply::json(&reply),
                warp::http::StatusCode::CREATED,
            ))
        }
        Err(e) => {
            let error = SessionReply {
                value: json!(SessionReplyError {
                    error: "session not created".to_string(),
                    message: format!("{}", e)
                }),
            };

            Ok(warp::reply::with_status(
                warp::reply::json(&error),
                warp::http::StatusCode::INTERNAL_SERVER_ERROR,
            ))
        }
    }
}

async fn register(ctx: Arc<Context>) -> RedisResult<()> {
    let mut con = ctx.con.clone();
    let data = [
        ("host", ctx.config.manager_host.clone()),
        ("port", ctx.config.manager_port.to_string()),
    ];
    con.hset_multiple(format!("manager:{}", ctx.config.manager_id), &data)
        .await
}

async fn deregister(ctx: Arc<Context>) -> RedisResult<()> {
    let mut con = ctx.con.clone();
    let _: () = con
        .srem("managers", &ctx.config.manager_id)
        .await?;
    
    con.del(format!("manager:{}", ctx.config.manager_id)).await
}

#[tokio::main]
async fn main() {
    let ctx = Arc::new(Context::new().await);

    register(ctx.clone()).await.unwrap();

    let heartbeat_key = format!("manager:{}:heartbeat", ctx.config.manager_id);
    ctx.heart.add_beat(heartbeat_key.clone(), 60, 120).await;

    let ctx_clone = ctx.clone();
    let with_ctx = warp::any().map(move || ctx_clone.clone());
    let session_route = warp::post()
        .and(warp::path("session"))
        .and(with_ctx)
        .and(warp::body::json())
        .and(warp::header::<String>("user-agent"))
        .and(warp::addr::remote())
        .and_then(handle_post);

    let server = warp::serve(session_route).run(([0, 0, 0, 0], 3033));

    tokio::spawn(server);

    ctx.heart.beat(true).await;
    ctx.heart.stop_beat(heartbeat_key).await;

    deregister(ctx.clone()).await.unwrap();
}