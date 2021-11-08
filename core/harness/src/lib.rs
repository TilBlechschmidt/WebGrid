//! Runtime harness to execute services in the context of components

#![deny(missing_docs)]
// Disable the lint for now as it has a high false-positive rate
#![allow(unknown_lints, clippy::nonstandard_macro_braces)]

mod heart;
mod module;
mod redis;
mod service;

pub use self::redis::*;
pub use heart::*;
pub use module::*;
pub use service::*;
