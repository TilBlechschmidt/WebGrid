use super::MatchingStrategy;
use crate::domain::container::ContainerImageSet;
use crate::domain::webdriver::CapabilitiesRequest;

/// [`ContainerImageSet`] based [`MatchingStrategy`]
///
/// For more details on the actual strategy, have a look at the documentation of the
/// [`match_against_capabilities`](ContainerImageSet::match_against_capabilities) method.
#[derive(Clone)]
pub struct ContainerMatchingStrategy(ContainerImageSet);

impl ContainerMatchingStrategy {
    /// Creates a new instance from a given image set
    pub fn new(image_set: ContainerImageSet) -> Self {
        Self(image_set)
    }
}

impl MatchingStrategy for ContainerMatchingStrategy {
    fn matches(&self, request: CapabilitiesRequest) -> bool {
        self.0.match_against_capabilities(request).is_some()
    }
}
