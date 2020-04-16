use redis::{aio::MultiplexedConnection, Client};
use std::net::SocketAddr;
use warp::Filter;

use log::info;
use shared::service_init;

mod config;
mod data_collector;
mod structures;

use crate::config::Config;
use data_collector::*;
use structures::*;

async fn handle_post(con: MultiplexedConnection) -> Result<impl warp::Reply, warp::Rejection> {
    let metrics: Vec<String> = vec![
        proxy_requests(&con).await,
        proxy_traffic(&con).await,
        session_log(&con).await,
        session_startup_duration(&con).await,
        // TODO Replace later with session_total{stage="queued|pending|alive|terminated"} counter
        sessions_active(&con).await,
    ]
    .iter()
    .map(|metric| format!("{}", metric))
    .collect();

    Ok(warp::reply::with_status(
        metrics.join("\n"),
        warp::http::StatusCode::OK,
    ))
}

#[tokio::main]
async fn main() {
    service_init();

    let config = Config::new().unwrap();

    let client = Client::open(config.clone().redis_url).unwrap();
    let con = client.get_multiplexed_tokio_connection().await.unwrap();

    let heart = shared::lifecycle::Heart::new(&con, None);

    let con_clone = con.clone();
    let with_con = warp::any().map(move || con_clone.clone());
    let session_route = warp::get()
        .and(warp::path("metrics"))
        .and(with_con)
        .and_then(handle_post);

    let listening_socket: SocketAddr = ([0, 0, 0, 0], 3036).into();
    info!("Listening at {:?}", listening_socket);
    let server = warp::serve(session_route).run(listening_socket);

    tokio::spawn(server);

    heart.beat(true).await;
}
