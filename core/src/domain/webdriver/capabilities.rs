//! Capability parsing structures
//!
//! This module contains data structures to deserialize the capability object described in the [W3C WebDriver Specification](https://www.w3.org/TR/webdriver1/#capabilities).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Timeout values for requests to the browser
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityTimeouts {
    /// Determines when to interrupt a script that is being evaluated.
    pub script: Option<u32>,
    /// Provides the timeout limit used to interrupt navigation of the browsing context.
    pub page_load: Option<u32>,
    /// Gives the timeout of when to abort locating an element.
    pub implicit: Option<u32>,
}

/// Describes which DOM event is used to determine whether or not a page has finished loading
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CapabilityPageLoadStrategy {
    /// Only page download, no parsing or asset loading
    None,
    /// Wait until all HTML content has been parsed, discarding assets
    ///
    /// This strategy waits until the [DOMContentLoaded](https://developer.mozilla.org/en-US/docs/Web/API/Document/DOMContentLoaded_event) event is fired
    Eager,
    /// Wait until all assets have been parsed and executed
    ///
    /// This strategy waits until the [load](https://developer.mozilla.org/en-US/docs/Web/API/Window/load_event) event is fired.
    /// Note that this may not indicate that async JavaScript has finished executing!
    Normal,
}

/// How popups like alerts or prompts should be handled
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum CapabilityUnhandledPromptBehavior {
    /// All simple dialogs encountered should be dismissed.
    Dismiss,
    /// All simple dialogs encountered should be accepted.
    Accept,
    /// All simple dialogs encountered should be dismissed, and an error returned that the dialog was handled.
    #[serde(rename = "dismiss and notify")]
    DismissAndNotify,
    /// All simple dialogs encountered should be accepted, and an error returned that the dialog was handled.
    #[serde(rename = "accept and notify")]
    AcceptAndNotify,
    /// All simple dialogs encountered should be left to the user to handle.
    Ignore,
}

/// HTTP proxy settings
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CapabilitiesProxy {
    /// Indicates the type of proxy configuration.
    pub proxy_type: String,
    /// Defines the URL for a proxy auto-config file if proxy_type is equal to "pac".
    pub proxy_autoconfig_url: Option<String>,
    /// Defines the proxy host for FTP traffic when the proxy_type is "manual".
    pub ftp_proxy: Option<String>,
    /// Defines the proxy host for HTTP traffic when the proxy_type is "manual".
    pub http_proxy: Option<String>,
    /// Lists the address for which the proxy should be bypassed when the proxy_type is "manual".
    pub no_proxy: Option<Vec<String>>,
    /// Defines the proxy host for encrypted TLS traffic when the proxy_type is "manual".
    pub ssl_proxy: Option<String>,
    /// Defines the proxy host for a SOCKS proxy when the proxy_type is "manual".
    pub socks_proxy: Option<String>,
    /// Defines the SOCKS proxy version when the proxy_type is "manual".
    pub socks_version: Option<u8>,
}

/// Extension capabilities specific to WebGrid
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WebGridOptions {
    /// Arbitrary metadata which can be set by the client and later fetched through the API.
    pub metadata: Option<HashMap<String, String>>,

    /// Force disables screen recording, defaults to `false`
    ///
    /// While this value defaults to false, the session may still decide that recording should be disabled
    /// either due to a forced setting by the administrator or due to unavailable storage.
    #[serde(default)]
    pub disable_recording: bool,
}

impl Default for WebGridOptions {
    fn default() -> Self {
        Self {
            metadata: None,
            disable_recording: false,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
/// Struct containing information about the browser requested or provided
pub struct Capabilities {
    /// Indicates if strict interactability checks should be applied to input type=file elements.
    pub strict_file_interactability: Option<bool>,
    /// Indicates whether untrusted and self-signed TLS certificates are implicitly trusted on navigation for the duration of the session.
    pub accept_insecure_certs: Option<bool>,
    /// Identifies the user agent.
    pub browser_name: Option<String>,
    /// Identifies the version of the user agent.
    pub browser_version: Option<String>,
    /// Identifies the operating system of the endpoint node.
    pub platform_name: Option<String>,

    /// Defines the current session’s page load strategy.
    pub page_load_strategy: Option<CapabilityPageLoadStrategy>,
    /// Defines the current session’s proxy configuration.
    pub proxy: Option<CapabilitiesProxy>,
    /// Describes the timeouts imposed on certain session operations.
    pub timeouts: Option<CapabilityTimeouts>,
    /// Describes the current session’s user prompt handler.
    pub unhandled_prompt_behavior: Option<CapabilityUnhandledPromptBehavior>,

    /// Extension capabilities specific to WebGrid
    #[serde(rename = "webgrid:options")]
    pub webgrid_options: Option<WebGridOptions>,

    /// Additional capabilities that are not part of the W3C standard or added by WebGrid.
    #[serde(flatten)]
    pub extension_capabilities: HashMap<String, serde_json::Value>,
}

/// Error thrown when interacting with Capabilities
enum CapabilityError {
    ValueDefinedInBothSets,
}

trait OptionXorExt
where
    Self: std::marker::Sized,
{
    fn xor(self, other: Self) -> Result<Self, CapabilityError>;
}

impl<T> OptionXorExt for Option<T> {
    fn xor(self, other: Self) -> Result<Self, CapabilityError> {
        if self.is_some() && other.is_some() {
            return Err(CapabilityError::ValueDefinedInBothSets);
        }

        Ok(self.or(other))
    }
}

impl Capabilities {
    /// Empty traits object, contains not even default values
    pub fn empty() -> Self {
        Capabilities {
            strict_file_interactability: None,
            accept_insecure_certs: None,
            browser_name: None,
            browser_version: None,
            platform_name: None,

            page_load_strategy: None,
            proxy: None,
            timeouts: None,
            unhandled_prompt_behavior: None,

            webgrid_options: None,
            extension_capabilities: HashMap::new(),
        }
    }

    /// Combines two capabilities objects
    pub fn merge(&self, other: Self) -> Self {
        Capabilities {
            strict_file_interactability: self
                .strict_file_interactability
                .xor(other.strict_file_interactability),
            accept_insecure_certs: self.accept_insecure_certs.xor(other.accept_insecure_certs),
            browser_name: self.browser_name.clone().xor(other.browser_name),
            browser_version: self.browser_version.clone().xor(other.browser_version),
            platform_name: self.platform_name.clone().xor(other.platform_name),

            page_load_strategy: self
                .page_load_strategy
                .clone()
                .xor(other.page_load_strategy),
            proxy: self.proxy.clone().xor(other.proxy),
            timeouts: self.timeouts.clone().xor(other.timeouts),
            unhandled_prompt_behavior: self
                .unhandled_prompt_behavior
                .clone()
                .xor(other.unhandled_prompt_behavior),

            webgrid_options: self.webgrid_options.clone().xor(other.webgrid_options),
            // TODO Merge the extension capabilities
            extension_capabilities: self.extension_capabilities.clone(),
        }
    }
}

/// List of requested capabilities by a client
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[cfg_attr(test, derive(Clone))]
#[serde(rename_all = "camelCase")]
pub struct CapabilitiesRequest {
    /// List of capabilites where the first matching one will be used
    pub first_match: Option<Vec<Capabilities>>,
    /// Capabilities that always have to be satisfied and will be merged with each of the `first_match` capabilities
    pub always_match: Option<Capabilities>,
}

impl CapabilitiesRequest {
    /// Converts the request into a set of possible combinations
    pub fn into_sets(self) -> Vec<Capabilities> {
        let always_match = self.always_match.unwrap_or_else(Capabilities::empty);
        let first_match = self
            .first_match
            .unwrap_or_else(|| vec![Capabilities::empty()]);

        first_match
            .into_iter()
            .map(|b| always_match.merge(b))
            .collect()
    }
}

/// Raw [`CapabilitiesRequest`](crate::domain::webdriver::CapabilitiesRequest) json string containing all fields
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RawCapabilitiesRequest(String);

impl RawCapabilitiesRequest {
    /// Creates a new instance from a raw string
    pub fn new(raw: String) -> Self {
        Self(raw)
    }

    /// Parses the contained string into a strongly typed object
    pub fn parse(&self) -> Result<CapabilitiesRequest, serde_json::Error> {
        serde_json::from_str(&self.0)
    }

    /// Provides a reference to the underlying request string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod do_deserialize {
    use super::*;

    #[test]
    fn missing_first_match() {
        let capabilities = r#"{"alwaysMatch":{"browserName":"chrome"}}"#;
        let parsed: CapabilitiesRequest = serde_json::from_str(capabilities).unwrap();
        let sets = parsed.into_sets();

        let mut expected_capabilities = Capabilities::empty();
        expected_capabilities.browser_name = Some("chrome".to_string());

        assert_eq!(sets, vec![expected_capabilities]);
    }

    #[test]
    fn missing_always_match() {
        let capabilities = r#"{"firstMatch":[{"browserName":"chrome"}]}"#;
        let parsed: CapabilitiesRequest = serde_json::from_str(capabilities).unwrap();
        let sets = parsed.into_sets();

        let mut expected_capabilities = Capabilities::empty();
        expected_capabilities.browser_name = Some("chrome".to_string());

        assert_eq!(sets, vec![expected_capabilities]);
    }

    #[test]
    fn real_world_request() {
        let capabilities = "{\"firstMatch\":[{\"browserName\":\"chrome\",\"goog:chromeOptions\":{\"args\":[\"no-sandbox\",\"disable-gpu\",\"disable-extensions\",\"disable-infobars\",\"dns-prefetch-disable\",\"no-proxy-server\",\"window-size=1920,1080\",\"start-maximized\",\"window-position=0,0\",\"--test-type\",\"disable-dev-shm-usage\"],\"extensions\":[],\"prefs\":{\"profile.default_content_settings.popups\":0}},\"proxy\":{\"proxyType\":\"direct\"}}]}";

        let _parsed: CapabilitiesRequest = serde_json::from_str(capabilities).unwrap();
    }
}
