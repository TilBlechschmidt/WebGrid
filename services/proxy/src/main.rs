#[macro_use]
extern crate lazy_static;

use redis::Client;
use std::thread;

use shared::{metrics::MetricsProcessor, service_init};

mod config;
mod proxy;
mod watcher;

use crate::config::Config;
use crate::proxy::ProxyServer;
use crate::watcher::RoutingInfo;

#[tokio::main]
async fn main() {
    service_init();

    let config = Config::new().unwrap();

    let client = Client::open(config.clone().redis_url).unwrap();
    let con = client.get_multiplexed_tokio_connection().await.unwrap();

    let mut metrics = MetricsProcessor::new(&con);
    let info = RoutingInfo::new();
    let proxy = ProxyServer::new(info.clone(), metrics.get_tx());

    tokio::spawn(async move {
        metrics.process().await;
    });

    thread::spawn(move || {
        watcher::main_loop(info, config).unwrap();
    });

    proxy.serve().await;
}
