//! Services to provision new browsers

mod matching;
mod provisioning;
mod state;
mod sync;
mod termination;

pub use matching::{ContainerMatchingStrategy, MatchingStrategy, ProvisionerMatchingService};
pub use provisioning::ProvisioningService;
pub use state::ProvisioningState;
pub use sync::HardwareSynchronisationService;
pub use termination::SessionTerminationWatcherService;
