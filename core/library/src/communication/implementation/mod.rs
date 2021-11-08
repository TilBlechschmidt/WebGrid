//! Implementations of traits from this module using third-party crates

// pub mod bincode;
pub mod json;
pub mod redis;

#[cfg(feature = "test")]
pub mod mock;
