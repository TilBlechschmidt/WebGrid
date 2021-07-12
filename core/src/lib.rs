//! This library crate contains all the necessities to run and manage a WebGrid instance.
//!
//! Submodules have been introduced to split responsibilities. Each module has a specific focus
//! and they together form a chain of dependencies from the low-level [`library`], over the WebGrid [`domain`]
//! specific logic, through the executable [`harness`], up to the high-level [`modules`](module) and contained service implementations.

#![deny(missing_docs)]
#![allow(clippy::nonstandard_macro_braces)]

pub mod constants;
pub mod domain;
pub mod harness;
pub mod library;
pub mod module;
