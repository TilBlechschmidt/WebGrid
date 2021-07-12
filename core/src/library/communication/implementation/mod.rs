//! Implementations of traits from this module using third-party crates

// pub mod bincode;
pub mod json;
pub mod redis;

#[cfg(test)]
pub mod mock;
