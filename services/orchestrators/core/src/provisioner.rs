use shared::capabilities::CapabilitiesRequest;
use shared::{parse_browser_string, split_into_two};

pub use async_trait::async_trait;
use std::fmt;

#[derive(Debug)]
pub struct NodeInfo {
    pub host: String,
    pub port: String,
}

pub struct ProvisionerCapabilities {
    pub platform_name: String,
    pub browsers: Vec<String>,
}

#[async_trait]
pub trait Provisioner {
    fn capabilities(&self) -> ProvisionerCapabilities;
    async fn provision_node(&self, session_id: &str, capabilities: CapabilitiesRequest)
        -> NodeInfo;
    async fn terminate_node(&self, session_id: &str);
}

#[derive(Debug)]
pub enum Type {
    Local,
    Docker,
    K8s,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}

/// Parses an environment string that describes images
/// "imageA=browser1::version1,imageB=browser2::version2"
/// => [("imageA", "browser1::version1"), ("imageB", "browser2::version2")]
pub fn parse_images_string(input: String) -> Vec<(String, String)> {
    input
        .split(',')
        .map(|image_string| split_into_two(image_string, "="))
        .filter(|p| p.is_some())
        .map(|p| p.unwrap())
        .collect()
}

pub fn match_image_from_capabilities(
    capabilities: CapabilitiesRequest,
    images: &[(String, String)],
) -> Option<String> {
    let first_image = images.first().map(|(image, _)| image.clone());
    let capability_sets = capabilities.into_sets();

    // Short circuit if no capabilities are given
    if capability_sets.is_empty() {
        return first_image;
    }

    capability_sets.into_iter().find_map(|capability_set| {
        // Short circuit if no specific browser is requested
        if capability_set.browser_name.is_none() && capability_set.browser_version.is_none() {
            return first_image.clone();
        }

        for (image, browser_string) in images {
            println!(
                "Matching {:?} against {} {}",
                capability_set, image, browser_string
            );

            if let Some((browser, version)) = parse_browser_string(&browser_string) {
                let mut version_match = true;
                let mut browser_match = true;

                if let Some(requested_browser) = capability_set.browser_name.clone() {
                    browser_match = requested_browser == browser;
                }

                if let Some(requested_version) = capability_set.browser_version.clone() {
                    version_match = version.find(&requested_version) == Some(0);
                }

                println!("{} {}", browser_match, version_match);

                if version_match && browser_match {
                    return Some(image.clone());
                }
            }
        }

        None
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_browser_specified_matched() {
        let capabilities = "{}";
        let request: CapabilitiesRequest = serde_json::from_str(&capabilities).unwrap();
        let images = parse_images_string(
            "webgrid-node:firefox=firefox::68.7.0esr,webgrid-node:chrome=chrome::81.0.4044.122"
                .to_owned(),
        );

        assert_eq!(
            match_image_from_capabilities(request, &images),
            Some("webgrid-node:firefox".to_owned())
        );
    }

    #[test]
    fn browser_name_matched() {
        let capabilities = "{\"firstMatch\":[{\"browserName\":\"chrome\"}]}";
        let request: CapabilitiesRequest = serde_json::from_str(&capabilities).unwrap();
        let images = parse_images_string(
            "webgrid-node:firefox=firefox::68.7.0esr,webgrid-node:chrome=chrome::81.0.4044.122"
                .to_owned(),
        );

        assert_eq!(
            match_image_from_capabilities(request, &images),
            Some("webgrid-node:chrome".to_owned())
        );
    }

    #[test]
    fn browser_version_partially_matched() {
        let capabilities =
            "{\"firstMatch\":[{\"browserName\":\"chrome\",\"browserVersion\":\"60.0.4044\"}]}";
        let request: CapabilitiesRequest = serde_json::from_str(&capabilities).unwrap();
        let images = parse_images_string(
            "webgrid-node:chrome1=chrome::60.0.4044.122,webgrid-node:chrome2=chrome::81.0.4044.122"
                .to_owned(),
        );

        assert_eq!(
            match_image_from_capabilities(request, &images),
            Some("webgrid-node:chrome1".to_owned())
        );
    }

    #[test]
    fn browser_version_fully_matched() {
        let capabilities =
            "{\"firstMatch\":[{\"browserName\":\"chrome\",\"browserVersion\":\"81.0.4044.122\"}]}";
        let request: CapabilitiesRequest = serde_json::from_str(&capabilities).unwrap();
        let images = parse_images_string(
            "webgrid-node:chrome1=chrome::60.0.4044.122,webgrid-node:chrome2=chrome::81.0.4044.122"
                .to_owned(),
        );

        assert_eq!(
            match_image_from_capabilities(request, &images),
            Some("webgrid-node:chrome2".to_owned())
        );
    }
}
