//! Structures to keep the process alive until some event occurs

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
use tokio::time::sleep;

/// Reason why the heart stopped beating
#[derive(Debug, Clone)]
pub enum DeathReason {
    /// Internal kill signal has been sent
    Killed(String),
    /// Predetermined lifetime has been exceeded
    LifetimeExceeded,
    /// SIGINT or other process-external cause
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

/// Action to a hearth
#[derive(Debug)]
pub enum HeartInteraction {
    /// Kill it for the given reason
    Kill(String),
    /// Reset its lifetime to the original value
    Rejuvenate,
}

/// Lifecycle management struct that can be used to keep the application alive
pub struct Heart {
    /// Receiver for interactions sent by heart stone
    rx: Receiver<HeartInteraction>,
    /// Point in time when the lifetime was last reset
    lifetime_start: Arc<Mutex<Instant>>,
    /// Maximum lifetime duration
    lifetime: Option<Duration>,
}

impl Heart {
    /// Creates a new heart and linked stone with no lifetime limit
    pub fn new() -> (Self, HeartStone) {
        Heart::internal_new(None)
    }

    /// Creates a new heart with no lifetime and discards the linked stone
    pub fn without_heart_stone() -> Self {
        Heart::internal_new(None).0
    }

    /// Creates a new heart and linked stone with a lifetime
    pub fn with_lifetime(lifetime: Duration) -> (Self, HeartStone) {
        Heart::internal_new(Some(lifetime))
    }

    /// Reduces the next lifetime timeout by artificially shifting the beginning of the current period.
    /// This allows e.g. shorter initial lifetimes.
    pub async fn reduce_next_lifetime(&mut self, new_lifetime: Duration) {
        if let Some(lifetime) = self.lifetime {
            *self.lifetime_start.lock().await = Instant::now() - lifetime + new_lifetime;
        } else {
            log::error!("Attempted to reduce non-existent lifetime");
        }
    }

    /// Future that waits until the heart dies for the returned reason
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

    async fn termination_signal() {
        let mut sigterm_stream = signal(SignalKind::terminate()).unwrap();
        let sigterm = sigterm_stream.recv().fuse();
        let ctrl_c = ctrl_c().fuse();

        pin_mut!(sigterm, ctrl_c);

        select! {
            _ = sigterm => {},
            _ = ctrl_c => {},
        };
    }

    async fn lifetime_watch(lifetime: Duration, lifetime_start: Arc<Mutex<Instant>>) {
        loop {
            let elapsed_time = lifetime_start.lock().await.elapsed();

            if elapsed_time > lifetime {
                break;
            }

            sleep(lifetime - elapsed_time).await;
        }
    }
}

/// Remote controller for the heart
#[derive(Clone)]
pub struct HeartStone {
    remote: Sender<HeartInteraction>,
}

impl HeartStone {
    fn new(remote: Sender<HeartInteraction>) -> Self {
        Self { remote }
    }

    /// Kill the associated heart
    pub async fn kill(&mut self, reason: String) {
        self.send(HeartInteraction::Kill(reason)).await;
    }

    /// Reset the lifetime of the associated heart
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
mod does {
    use super::*;
    use futures::poll;
    use tokio::task::{spawn, yield_now};
    use tokio::time::sleep;

    #[tokio::test]
    async fn reduce_lifetime() {
        let lifetime = Duration::from_millis(500);
        let reduced_lifetime = Duration::from_millis(100);

        let (mut heart, _stone) = Heart::with_lifetime(lifetime);
        let (mut reduced_heart, _reduced_stone) = Heart::with_lifetime(lifetime);

        reduced_heart.reduce_next_lifetime(reduced_lifetime).await;

        let handle = spawn(async move { heart.death().await });
        let reduced_handle = spawn(async move { reduced_heart.death().await });

        sleep(reduced_lifetime).await;
        yield_now().await;

        assert!(!poll!(handle).is_ready());
        assert!(poll!(reduced_handle).is_ready());
    }

    #[tokio::test]
    async fn live_without_lifetime() {
        let (mut heart, _stone) = Heart::new();

        let handle = spawn(async move { heart.death().await });
        sleep(Duration::from_millis(100)).await;
        yield_now().await;

        assert!(!poll!(handle).is_ready());
    }

    #[tokio::test]
    async fn die_when_killed() {
        let (mut heart, mut stone) = Heart::new();

        let handle = spawn(async move { heart.death().await });
        stone.kill("Testing".to_owned()).await;
        yield_now().await;

        assert!(poll!(handle).is_ready());
    }

    #[tokio::test]
    async fn die_after_lifetime() {
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
