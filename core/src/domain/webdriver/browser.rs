use crate::library::helpers::split_into_two;
use std::str::FromStr;
use thiserror::Error;

/// Error thrown while parsing a browser from string
#[derive(Debug, Error)]
pub enum BrowserParseError {
    /// Missing `::` separator between browser name and version
    #[error("missing :: separator between browser name and version")]
    MissingImageBrowserSeparator,
}

/// Web browser definition
///
/// Parsable from a custom string containing the name and version separated by `::`
/// ```
/// # use webgrid::domain::webdriver::Browser;
/// let browser: Browser = "chrome::82".parse().unwrap();
/// assert_eq!(browser.name, "chrome");
/// assert_eq!(browser.version, "82");
/// ```
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Browser {
    /// Unique identifier of a given web browser
    pub name: String,
    /// Browser specific version string
    pub version: String,
}

impl FromStr for Browser {
    type Err = BrowserParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((name, version)) = split_into_two(s, "::") {
            Ok(Browser { name, version })
        } else {
            Err(BrowserParseError::MissingImageBrowserSeparator)
        }
    }
}

#[cfg(test)]
mod does {
    use super::*;

    #[test]
    fn parse_browser_string() {
        let browser: Browser = "chrome::82".parse().unwrap();
        assert_eq!(browser.name, "chrome");
        assert_eq!(browser.version, "82");
    }

    #[test]
    #[should_panic]
    fn fail_on_missing_separator() {
        let _: Browser = "chrome".parse().unwrap();
    }
}
