use pretty_env_logger;

mod heart;
mod heart_beat;
pub mod logging;

pub use heart::{Heart, HeartStone};
pub use heart_beat::HeartBeat;

pub fn service_init() {
    pretty_env_logger::init_timed();
}
