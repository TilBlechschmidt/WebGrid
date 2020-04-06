use crate::config::Config;
use log::{error, info};
use rand::seq::SliceRandom;
use redis::{cmd, Client, Commands, Connection, PubSub, RedisResult};
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

static PATTERN_MANAGER: &str = "__keyspace@0__:manager:*:heartbeat";
static PATTERN_SESSION: &str = "__keyspace@0__:session:*:heartbeat.node";

lazy_static! {
    static ref REGEX_MANAGER: Regex =
        Regex::new(r"__keyspace@0__:manager:(?P<mid>[^:]+):heartbeat").unwrap();
    static ref REGEX_SESSION: Regex =
        Regex::new(r"__keyspace@0__:session:(?P<sid>[^:]+):heartbeat\.node").unwrap();
}

#[derive(Clone)]
pub struct RoutingInfo {
    pub managers: Arc<Mutex<HashMap<String, String>>>,
    pub sessions: Arc<Mutex<HashMap<String, String>>>,
}

impl RoutingInfo {
    pub fn new() -> Self {
        RoutingInfo {
            managers: Arc::new(Mutex::new(HashMap::new())),
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn get_manager_upstreams(&self) -> Vec<String> {
        let managers = self.managers.lock().unwrap();
        managers.iter().map(|(_, v)| v.clone()).collect()
    }

    pub fn get_manager_upstream(&self) -> Option<String> {
        let upstreams = self.get_manager_upstreams();
        if upstreams.is_empty() {
            return None;
        }
        upstreams.choose(&mut rand::thread_rng()).cloned()
    }

    pub fn get_session_upstream(&self, session_id: &str) -> Option<String> {
        let sessions = self.sessions.lock().unwrap();
        sessions.get(session_id).cloned()
    }

    // TODO Code duplication in the four methods below
    fn add_manager_upstream(&self, manager_id: String, host: &str, port: &str) -> Option<String> {
        let addr = format!("{}:{}", host, port);
        let mut managers = self.managers.lock().unwrap();
        managers.insert(manager_id, addr)
    }

    fn add_session_upstream(&self, session_id: String, host: &str, port: &str) -> Option<String> {
        let addr = format!("{}:{}", host, port);
        let mut sessions = self.sessions.lock().unwrap();
        sessions.insert(session_id, addr)
    }

    fn remove_manager_upstream(&self, manager_id: &str) {
        self.managers.lock().unwrap().remove(manager_id);
    }

    fn remove_session_upstream(&self, session_id: &str) {
        self.sessions.lock().unwrap().remove(session_id);
    }
}

fn verify_keyspace_events_config(con: &mut Connection) -> RedisResult<()> {
    let keyspace_events: RedisResult<(String, String)> = cmd("CONFIG")
        .arg("GET")
        .arg("notify-keyspace-events")
        .query(con);

    match keyspace_events {
        Ok((_key, events_value)) => {
            if !(events_value.contains('K')
                && events_value.contains('g')
                && events_value.contains('x'))
            {
                error!("Redis server config does not contain the values 'Kgx' at the 'notify-keyspace-events' key");
            }

            Ok(())
        }
        Err(e) => Err(e),
    }
}

fn handle_manager_message(
    channel: &str,
    operation: &str,
    info: &RoutingInfo,
    con: &mut Connection,
) {
    if let Some(caps) = REGEX_MANAGER.captures(channel) {
        let manager_id = &caps["mid"];

        match operation {
            // Manager has been added
            "expire" => {
                let data_key = format!("manager:{}", manager_id);
                let res: RedisResult<(String, String)> = con.hget(data_key, &["host", "port"]);

                if let Ok((host, port)) = res {
                    if info
                        .add_manager_upstream(manager_id.to_string(), &host, &port)
                        .is_none()
                    {
                        info!("+ Manager {} @ {}:{}", manager_id, host, port);
                    }
                }
            }
            // Manager has died
            "expired" => {
                info!("- Manager {}", manager_id);
                info.remove_manager_upstream(manager_id);
            }
            &_ => {}
        }
    }
}

fn handle_session_message(
    channel: &str,
    operation: &str,
    info: &RoutingInfo,
    con: &mut Connection,
) {
    if let Some(caps) = REGEX_SESSION.captures(channel) {
        let session_id = &caps["sid"];

        match operation {
            // Node has become alive
            "expire" => {
                let data_key = format!("session:{}:upstream", session_id);
                let res: RedisResult<(String, String)> = con.hget(data_key, &["host", "port"]);

                if let Ok((host, port)) = res {
                    if info
                        .add_session_upstream(session_id.to_string(), &host, &port)
                        .is_none()
                    {
                        info!("+ Session {} @ {}:{}", session_id, host, port);
                    }
                }
            }
            // Node has died
            "expired" => {
                info!("- Session {}", session_id);
                info.remove_session_upstream(session_id);
            }
            &_ => {}
        }
    }
}

fn loop_iteration(pubsub: &mut PubSub, info: RoutingInfo, con: &mut Connection) -> RedisResult<()> {
    let msg = pubsub.get_message()?;
    let pattern = msg.get_pattern().ok();
    let channel: &str = msg.get_channel_name();
    let operation: String = msg.get_payload()?;

    if pattern == Some(PATTERN_MANAGER.to_string()) {
        handle_manager_message(channel, &operation, &info, con);
    } else if pattern == Some(PATTERN_SESSION.to_string()) {
        handle_session_message(channel, &operation, &info, con);
    }

    Ok(())
}

pub fn main_loop(info: RoutingInfo, config: Config) -> RedisResult<()> {
    let client = Client::open(config.redis_url)?;
    let mut con = client.get_connection()?;
    let mut pubsub_con = client.get_connection()?;

    verify_keyspace_events_config(&mut con)?;

    let mut pubsub = pubsub_con.as_pubsub();

    pubsub.psubscribe(PATTERN_MANAGER)?;
    pubsub.psubscribe(PATTERN_SESSION)?;

    loop {
        loop_iteration(&mut pubsub, info.clone(), &mut con).ok();
    }
}
