use chrono::Utc;
use futures::{future::FutureExt, pin_mut, select};
use hyper::{body, Client, Uri};
use log::{debug, info, trace};
use redis::{aio::MultiplexedConnection, AsyncCommands, RedisResult, Script};
use std::collections::HashMap;
use std::sync::{atomic::AtomicBool, atomic::Ordering, Arc, Mutex};
use std::time::Duration;
use tokio::signal::{
    ctrl_c,
    unix::{signal, SignalKind},
};
use tokio::time;
use tokio::time::{delay_for, timeout};

#[derive(Debug)]
pub enum DeathReason {
    Killed,
    LifetimeExceeded,
}

#[derive(Clone)]
pub struct Heart {
    original_lifetime: Option<usize>,
    lifetime: Arc<Mutex<Option<usize>>>,
    beating: Arc<AtomicBool>,
    con: MultiplexedConnection,
    beats: Arc<Mutex<HashMap<String, (usize, usize)>>>,
}

impl Heart {
    pub fn new(con: &MultiplexedConnection, lifetime: Option<usize>) -> Heart {
        if let Some(lifetime) = lifetime {
            info!("Lifetime set to {} seconds", lifetime);
        }

        Heart {
            original_lifetime: lifetime,
            lifetime: Arc::new(Mutex::new(lifetime)),
            beating: Arc::new(AtomicBool::new(false)),
            con: con.clone(),
            beats: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn add_beat(&self, key: String, interval_secs: usize, expiration_secs: usize) {
        let insert_key = key.clone();

        debug!("Added heartbeat {}", key);

        {
            let mut beats = self.beats.lock().unwrap();
            beats.insert(insert_key, (interval_secs, expiration_secs));
        }

        let _: RedisResult<()> = self.con.clone().set_ex(key, "42", expiration_secs).await;
    }

    pub async fn stop_beat(&self, key: String) {
        debug!("Removed heartbeat {}", key);

        {
            let mut beats = self.beats.lock().unwrap();
            beats.remove(&key);
        }
        let _: RedisResult<()> = self.con.clone().expire(key, 1).await;
    }

    pub fn kill(&self) {
        self.beating.store(false, Ordering::Relaxed);
    }

    pub fn reset_lifetime(&self) {
        if let Some(original_lifetime) = self.original_lifetime {
            let mut lifetime = self.lifetime.lock().unwrap();
            *lifetime = Some(original_lifetime);
        }
    }

    pub async fn beat(&self, handle_signals: bool) -> DeathReason {
        let mut con = self.con.clone();
        let mut interval = time::interval(Duration::from_secs(1));
        let mut passed_time: usize = 0;

        debug!("Heart starts beating (handle_signals: {})", handle_signals);

        if handle_signals {
            let cloned_heart = self.clone();
            tokio::spawn(async move {
                let mut sigterm_stream = signal(SignalKind::terminate()).unwrap();
                let sigterm = sigterm_stream.recv().fuse();
                let ctrl_c = ctrl_c().fuse();

                pin_mut!(sigterm, ctrl_c);

                select! {
                    (_) = sigterm => cloned_heart.kill(),
                    (_) = ctrl_c => cloned_heart.kill(),
                };

                info!("Received shutdown signal!");
            });
        }

        self.beating.store(true, Ordering::Relaxed);
        loop {
            interval.tick().await;
            passed_time += 1;

            for (key, (refresh_time, expiration_time)) in self.beats.lock().unwrap().iter() {
                if passed_time % refresh_time == 0 {
                    let _: RedisResult<()> = con
                        .set_ex(key, Utc::now().to_rfc3339(), *expiration_time)
                        .await;
                }
            }

            let beating = self.beating.load(Ordering::Relaxed);
            if !beating {
                return DeathReason::Killed;
            }

            let mut lifetime = self.lifetime.lock().unwrap();
            if lifetime.is_some() {
                let new_lifetime = lifetime.unwrap() - 1;
                *lifetime = Some(new_lifetime);

                if new_lifetime == 0 {
                    return DeathReason::LifetimeExceeded;
                }
            }
        }
    }
}

pub async fn wait_for(url: &str, timeout_duration: Duration) -> Result<String, ()> {
    let client = Client::new();

    let url = url.parse::<Uri>().unwrap();

    let check_interval = Duration::from_millis(250);
    let request_timeout = Duration::from_millis(1000);
    let mut remaining_duration = timeout_duration;

    debug!("Awaiting 200 OK response from {}", url);

    loop {
        let request = client.get(url.clone());

        let response = timeout(request_timeout, request).await;

        // Rust does not yet support boolean and let in the same IF statement. TODO Replace this once language support lands
        if let Ok(Ok(res)) = response {
            if res.status() == 200 {
                return match body::to_bytes(res.into_body()).await {
                    Ok(bytes) => {
                        Ok(String::from_utf8(bytes.to_vec()).unwrap_or_else(|_| "".to_string()))
                    }
                    Err(_) => Ok("".to_string()),
                };
            }

            trace!("Received response with status != 200");
        }

        if remaining_duration.as_secs() == 0 {
            debug!("Timeout while waiting for {}", url);
            return Err(());
        }

        delay_for(check_interval).await;
        remaining_duration -= check_interval;
    }
}

pub fn generate_session_termination_script(use_orchestrator_argument: bool) -> Script {
    let body = r"
    
    local slot = redis.call('get', 'session:' .. ARGV[1] .. ':slot')
    redis.call('del', 'session:' .. ARGV[1] .. ':slot')
    redis.call('rpush', 'orchestrator:' .. orchestrator .. ':slots.reclaimed', slot)
    redis.call('smove', 'sessions.active', 'sessions.terminated', ARGV[1])
    redis.call('hset', 'session:'  .. ARGV[1] .. ':status', 'terminatedAt', ARGV[2])
    redis.call('expire', 'session:' .. ARGV[1] .. ':heartbeat.node', 1)
    return {ARGV[1], slot, orchestrator}
    ";

    let orchestrator_fetch_call = "local orchestrator = redis.call('rpoplpush', 'session:' .. ARGV[1] .. ':orchestrator', 'session:' .. ARGV[1] .. ':orchestrator')";
    let orchestrator_argument = "local orchestrator = ARGV[3]";

    if use_orchestrator_argument {
        Script::new(&format!("{}\n{}", orchestrator_argument, body))
    } else {
        Script::new(&format!("{}\n{}", orchestrator_fetch_call, body))
    }
}
