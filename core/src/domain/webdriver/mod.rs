//! Types in accordance with the [WebDriver](https://github.com/w3c/webdriver) specification

mod browser;
mod capabilities;
mod creation;
mod instance;

pub use browser::*;
pub use capabilities::*;
pub use creation::*;
pub use instance::*;
