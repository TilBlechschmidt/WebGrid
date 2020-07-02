use futures::{
    channel::mpsc::{channel, Receiver, Sender},
    lock::Mutex,
    pin_mut,
    prelude::*,
    select,
};
use log::{debug, error, info};
use std::{
    fmt,
    fmt::{Error as FmtError, Formatter},
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::signal::{
    ctrl_c,
    unix::{signal, SignalKind},
};
use tokio::time::delay_for;

#[derive(Debug, Clone)]
pub enum DeathReason {
    Killed(String),
    LifetimeExceeded,
    Terminated,
}

impl fmt::Display for DeathReason {
    fn fmt(&self, w: &mut Formatter<'_>) -> Result<(), FmtError> {
        match self {
            DeathReason::Killed(reason) => write!(w, "Killed ({})", reason),
            DeathReason::LifetimeExceeded => write!(w, "Lifetime was exceeded"),
            DeathReason::Terminated => write!(w, "Terminated due to external signal"),
        }
    }
}

#[derive(Debug)]
pub enum HeartInteraction {
    Kill(String),
    Rejuvenate,
}

pub struct Heart {
    rx: Receiver<HeartInteraction>,
    lifetime_start: Arc<Mutex<Instant>>,
    lifetime: Option<Duration>,
}

impl Heart {
    pub fn new() -> (Self, HeartStone) {
        Heart::internal_new(None)
    }

    pub fn with_lifetime(lifetime: Duration) -> (Self, HeartStone) {
        Heart::internal_new(Some(lifetime))
    }

    fn internal_new(lifetime: Option<Duration>) -> (Self, HeartStone) {
        if let Some(lifetime) = lifetime {
            info!("Lifetime set to {} seconds", lifetime.as_secs());
        }

        let (tx, rx) = channel(2);
        let heart = Self {
            rx,
            lifetime_start: Arc::new(Mutex::new(Instant::now())),
            lifetime,
        };
        let stone = HeartStone::new(tx);

        (heart, stone)
    }

    pub async fn death(&mut self) -> DeathReason {
        let mut age_future = match self.lifetime {
            Some(lifetime) => Heart::lifetime_watch(lifetime, self.lifetime_start.clone()).boxed(),
            None => futures::future::pending().boxed(),
        }
        .fuse();

        debug!("Heart starts beating");

        loop {
            select! {
                interaction = self.rx.next() => {
                    if let Some(interaction) = interaction {
                        match interaction {
                            HeartInteraction::Kill(reason) => return DeathReason::Killed(reason),
                            HeartInteraction::Rejuvenate => {
                                *self.lifetime_start.lock().await = Instant::now();
                            }
                        }
                    }
                },
                () = age_future => return DeathReason::LifetimeExceeded,
                () = Heart::termination_signal().fuse() => return DeathReason::Terminated,
            };
        }
    }

    async fn termination_signal() {
        let mut sigterm_stream = signal(SignalKind::terminate()).unwrap();
        let sigterm = sigterm_stream.recv().fuse();
        let ctrl_c = ctrl_c().fuse();

        pin_mut!(sigterm, ctrl_c);

        select! {
            (_) = sigterm => (),
            (_) = ctrl_c => (),
        };
    }

    async fn lifetime_watch(lifetime: Duration, lifetime_start: Arc<Mutex<Instant>>) {
        loop {
            let elapsed_time = lifetime_start.lock().await.elapsed();

            if elapsed_time > lifetime {
                break;
            }

            delay_for(lifetime - elapsed_time).await;
        }
    }
}

#[derive(Clone)]
pub struct HeartStone {
    remote: Sender<HeartInteraction>,
}

impl HeartStone {
    fn new(remote: Sender<HeartInteraction>) -> Self {
        Self { remote }
    }

    pub async fn kill(&mut self, reason: String) {
        self.send(HeartInteraction::Kill(reason)).await;
    }

    pub async fn reset_lifetime(&mut self) {
        self.send(HeartInteraction::Rejuvenate).await;
    }

    async fn send(&mut self, interaction: HeartInteraction) {
        if let Err(e) = self.remote.send(interaction).await {
            error!("Failed to interact with Heart: {}", e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::poll;
    use tokio::task::{spawn, yield_now};
    use tokio::time::delay_for as sleep;

    #[tokio::test]
    async fn lives_without_lifetime() {
        let (mut heart, _stone) = Heart::new();

        let handle = spawn(async move { heart.death().await });
        sleep(Duration::from_millis(100)).await;
        yield_now().await;

        assert!(!poll!(handle).is_ready());
    }

    #[tokio::test]
    async fn dies_when_killed() {
        let (mut heart, mut stone) = Heart::new();

        let handle = spawn(async move { heart.death().await });
        stone.kill("Testing".to_owned()).await;
        yield_now().await;

        assert!(poll!(handle).is_ready());
    }

    #[tokio::test]
    async fn dies_after_lifetime() {
        let lifetime = Duration::from_millis(10);
        let (mut heart, _stone) = Heart::with_lifetime(lifetime);

        let handle = spawn(async move { heart.death().await });
        sleep(lifetime).await;
        yield_now().await;

        assert!(poll!(handle).is_ready());
    }

    // TODO Re-implement this test, tokio JoinHandles can not be .shared() thus it is inactive for now :(
    // #[tokio::test]
    // async fn lives_longer_after_rejuvenation() {
    //     let lifetime = Duration::from_millis(10);
    //     let (mut heart, mut stone) = Heart::with_lifetime(lifetime);

    //     let handle = spawn(async move { heart.death().await });

    //     // Wait half the lifetime and reset it
    //     sleep(lifetime / 2).await;
    //     stone.reset_lifetime().await;

    //     // Check status after the original lifetime elapsed
    //     sleep(lifetime / 2).await;
    //     yield_now().await;
    //     assert!(!poll!(handle).is_ready());

    //     // Wait for the reset lifetime to expire and check status
    //     sleep(lifetime / 2).await;
    //     yield_now().await;
    //     assert!(poll!(handle).is_ready());
    // }
}
