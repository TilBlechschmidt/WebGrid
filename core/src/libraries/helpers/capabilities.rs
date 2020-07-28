use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityTimeouts {
    pub script: Option<u32>,
    pub page_load: Option<u32>,
    pub implicit: Option<u32>,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum CapabilityPageLoadStrategy {
    None,
    Eager,
    Normal,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub enum CapabilityUnhandledPromptBehavior {
    Dismiss,
    Accept,
    #[serde(rename = "dismiss and notify")]
    DismissAndNotify,
    #[serde(rename = "accept and notify")]
    AcceptAndNotify,
    Ignore,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CapabilitiesProxy {
    pub proxy_type: String,
    pub proxy_autoconfig_url: Option<String>,
    pub ftp_proxy: Option<String>,
    pub http_proxy: Option<String>,
    pub no_proxy: Option<Vec<String>>,
    pub ssl_proxy: Option<String>,
    pub socks_proxy: Option<String>,
    pub socks_version: Option<u8>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub strict_file_interactability: Option<bool>,
    pub accept_insecure_certs: Option<bool>,
    pub browser_name: Option<String>,
    pub browser_version: Option<String>,
    pub platform_name: Option<String>,

    pub page_load_strategy: Option<CapabilityPageLoadStrategy>,
    pub proxy: Option<CapabilitiesProxy>,
    pub timeouts: Option<CapabilityTimeouts>,
    pub unhandled_prompt_behavior: Option<CapabilityUnhandledPromptBehavior>,

    #[serde(flatten)]
    pub extension_capabilities: HashMap<String, Value>,
}

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
    fn empty() -> Self {
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

            extension_capabilities: HashMap::new(),
        }
    }

    fn merge(&self, other: Self) -> Self {
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

            // TODO Merge the extension capabilities
            extension_capabilities: self.extension_capabilities.clone(),
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CapabilitiesRequest {
    pub first_match: Option<Vec<Capabilities>>,
    pub always_match: Option<Capabilities>,
}

impl CapabilitiesRequest {
    pub fn into_sets(self) -> Vec<Capabilities> {
        let always_match = self.always_match.unwrap_or_else(Capabilities::empty);
        let first_match = self.first_match.unwrap_or_default();

        first_match
            .into_iter()
            .map(|b| always_match.merge(b))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_real_world_request() {
        let capabilities = "{\"firstMatch\":[{\"browserName\":\"chrome\",\"goog:chromeOptions\":{\"args\":[\"no-sandbox\",\"disable-gpu\",\"disable-extensions\",\"disable-infobars\",\"dns-prefetch-disable\",\"no-proxy-server\",\"window-size=1920,1080\",\"start-maximized\",\"window-position=0,0\",\"--test-type\",\"disable-dev-shm-usage\"],\"extensions\":[],\"prefs\":{\"profile.default_content_settings.popups\":0}},\"proxy\":{\"proxyType\":\"direct\"}}]}";

        let _parsed: CapabilitiesRequest = serde_json::from_str(&capabilities).unwrap();
    }
}
