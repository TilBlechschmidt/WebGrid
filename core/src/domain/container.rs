//! Logic related to container images

use super::webdriver::{Browser, BrowserParseError, CapabilitiesRequest};
use crate::library::helpers::split_into_two;
use std::str::FromStr;
use thiserror::Error;
use tracing::{debug, trace};

/// Error thrown while parsing a container image definition from string
#[derive(Debug, Error)]
pub enum ContainerImageParseError {
    /// Missing = separator between image and browser definition
    #[error("missing = separator between image and browser definition")]
    MissingImageBrowserSeparator,
    /// At least one browser definition failed parsing
    #[error("invalid browser definition")]
    BrowserDefinitionInvalid(#[from] BrowserParseError),
}

/// Definition of a container image containing a web browser
///
/// Parsable from a custom string containing the image and [`Browser`] definition
/// separated by `=`
/// ```
/// # use webgrid::domain::{container::ContainerImage, webdriver::Browser};
/// let image: ContainerImage = "webgrid/node-chrome=chrome::82".parse().unwrap();
/// let browser = Browser { name: "chrome".into(), version: "82".into() };
/// assert_eq!(image.identifier, "webgrid/node-chrome");
/// assert_eq!(image.browser, browser)
/// ```
#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ContainerImage {
    /// Combination of repository, image, and tag
    pub identifier: String,
    /// Browser contained within the image
    pub browser: Browser,
}

impl FromStr for ContainerImage {
    type Err = ContainerImageParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if let Some((identifier, raw_browser)) = split_into_two(s, "=") {
            let browser = raw_browser.parse()?;
            Ok(ContainerImage {
                identifier,
                browser,
            })
        } else {
            Err(ContainerImageParseError::MissingImageBrowserSeparator)
        }
    }
}

/// Set containing [`ContainerImages`](ContainerImage)
///
/// Parsable from a custom string containing [`ContainerImages`](ContainerImage) separated by `,`
#[derive(Debug, Clone)]
pub struct ContainerImageSet(Vec<ContainerImage>);

impl ContainerImageSet {
    /// Returns `true` if this set contains no images.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl FromStr for ContainerImageSet {
    type Err = ContainerImageParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut images = Vec::new();

        for image_string in s.split(',') {
            images.push(image_string.parse()?);
        }

        Ok(Self(images))
    }
}

impl ContainerImageSet {
    /// Retrieves the first [`ContainerImage`] matching the given request
    ///
    /// In order to select a matching image, these steps are followed:
    /// 1. Split the [`CapabilitiesRequest`] into distinct sets using [`into_sets`](CapabilitiesRequest::into_sets)
    /// 2. If the list of sets is empty, return the first image from the underlying [`ContainerImageSet`]
    /// 3. Iterate the set of [capabilities](super::webdriver::Capabilities) to find the first element where either
    ///     - No browser name *and* version is set, returning the first image
    ///     - Browser name and version match any of the images, returning the matching image
    ///
    /// Browser names are compared by a direct string equality check. Meanwhile, versions are instead relying
    /// on a string prefix check. This allows e.g. a requested version of `81` to match an image version string
    /// containing `81.0.4044.122` which is very common.
    pub fn match_against_capabilities(
        &self,
        request: CapabilitiesRequest,
    ) -> Option<&ContainerImage> {
        let first_image = self.0.first();
        let capability_sets = request.into_sets();

        // Short circuit if no capabilities are given
        if capability_sets.is_empty() {
            return first_image;
        }

        capability_sets.into_iter().find_map(|capability_set| {
            // Short circuit if no specific browser is requested
            if capability_set.browser_name.is_none() && capability_set.browser_version.is_none() {
                return first_image;
            }

            for image in self.0.iter() {
                debug!("Matching {:?} against {:?}", capability_set, image);

                let mut version_match = true;
                let mut browser_match = true;

                if let Some(requested_name) = &capability_set.browser_name {
                    browser_match = requested_name == &image.browser.name;
                }

                if let Some(requested_version) = &capability_set.browser_version {
                    version_match = image.browser.version.find(requested_version) == Some(0);
                }

                trace!("Match result {} {}", browser_match, version_match);

                if version_match && browser_match {
                    return Some(image);
                }
            }

            None
        })
    }
}

#[cfg(test)]
mod does {
    use super::*;

    #[test]
    fn parse_image_string() {
        let image: ContainerImage = "webgrid/node-chrome=chrome::82".parse().unwrap();
        let browser = Browser {
            name: "chrome".into(),
            version: "82".into(),
        };
        assert_eq!(image.identifier, "webgrid/node-chrome");
        assert_eq!(image.browser, browser)
    }

    #[test]
    #[should_panic]
    fn fail_on_missing_image_separator() {
        let _: Browser = "webgrid/node-chrome".parse().unwrap();
    }

    #[test]
    fn parse_image_set_string_with_one_image() {
        let set: ContainerImageSet = "webgrid/node-chrome=chrome::82".parse().unwrap();
        assert_eq!(set.0.len(), 1);
    }

    #[test]
    fn parse_image_set_string_with_multiple_images() {
        let set: ContainerImageSet = "image=browser::version,image2=browser2::version2"
            .parse()
            .unwrap();
        assert_eq!(set.0.len(), 2);
    }

    #[test]
    fn match_empty_request() {
        let request: CapabilitiesRequest = serde_json::from_str("{}").unwrap();
        let images: ContainerImageSet =
            "webgrid-node:firefox=firefox::68.7.0esr,webgrid-node:chrome=chrome::81.0.4044.122"
                .parse()
                .unwrap();

        assert_eq!(
            images
                .match_against_capabilities(request)
                .map(|c| &c.identifier),
            Some(&"webgrid-node:firefox".to_owned())
        );
    }

    #[test]
    fn match_browser_name_only() {
        let capabilities = "{\"firstMatch\":[{\"browserName\":\"chrome\"}]}";
        let request: CapabilitiesRequest = serde_json::from_str(capabilities).unwrap();
        let images: ContainerImageSet =
            "webgrid-node:firefox=firefox::68.7.0esr,webgrid-node:chrome=chrome::81.0.4044.122"
                .parse()
                .unwrap();

        assert_eq!(
            images
                .match_against_capabilities(request)
                .map(|c| &c.identifier),
            Some(&"webgrid-node:chrome".to_owned())
        );
    }

    #[test]
    fn match_browser_version_partially() {
        let capabilities =
            "{\"firstMatch\":[{\"browserName\":\"chrome\",\"browserVersion\":\"60.0.4044\"}]}";
        let request: CapabilitiesRequest = serde_json::from_str(capabilities).unwrap();

        let images: ContainerImageSet =
            "webgrid-node:chrome1=chrome::60.0.4044.122,webgrid-node:chrome2=chrome::81.0.4044.122"
                .parse()
                .unwrap();

        assert_eq!(
            images
                .match_against_capabilities(request)
                .map(|c| &c.identifier),
            Some(&"webgrid-node:chrome1".to_owned())
        );
    }

    #[test]
    fn match_browser_version_fully() {
        let capabilities =
            "{\"firstMatch\":[{\"browserName\":\"chrome\",\"browserVersion\":\"81.0.4044.122\"}]}";
        let request: CapabilitiesRequest = serde_json::from_str(capabilities).unwrap();

        let images: ContainerImageSet =
            "webgrid-node:chrome1=chrome::60.0.4044.122,webgrid-node:chrome2=chrome::81.0.4044.122"
                .parse()
                .unwrap();

        assert_eq!(
            images
                .match_against_capabilities(request)
                .map(|c| &c.identifier),
            Some(&"webgrid-node:chrome2".to_owned())
        );
    }
}
