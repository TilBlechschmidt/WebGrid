#[macro_use]
extern crate lazy_static;

use std::thread;

mod config;
mod proxy;
mod watcher;

use crate::config::Config;
use crate::proxy::ProxyServer;
use crate::watcher::RoutingInfo;

#[tokio::main]
async fn main() {
    let config = Config::new().unwrap();
    let info = RoutingInfo::new();
    let proxy = ProxyServer::new(info.clone());

    thread::spawn(move || {
        watcher::main_loop(info, config).unwrap();
    });

    proxy.serve().await;
}
