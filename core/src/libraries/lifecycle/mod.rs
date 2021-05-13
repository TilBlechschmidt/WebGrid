//! Service lifecycle functions

mod heart;
mod heart_beat;

pub use heart::{DeathReason, Heart, HeartStone};
pub use heart_beat::{BeatValue, HeartBeat};
