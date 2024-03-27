use std::time::Duration;

use testcontainers::{Container, Image};

#[derive(Clone, Debug, Default)]
pub struct InterbtcClientsImage {
    pub wait_for: Vec<testcontainers::core::WaitFor>,
}

const DEFAULT_INTERBTC_CLIENTS_IMAGE: &str = "ggxdocker/interbtc-clients";

#[cfg(feature = "brooklyn")]
const DEFAULT_INTERBTC_BROOKLYN_TAG: &str = "brooklyn-e17086b48b3e5553b20bbafa08a028dd31467c6d";

#[cfg(feature = "sydney")]
const DEFAULT_INTERBTC_SYDNEY_TAG: &str = "sydney-e17086b48b3e5553b20bbafa08a028dd31467c6d";

impl Image for InterbtcClientsImage {
    type Args = Vec<String>;

    fn name(&self) -> String {
        DEFAULT_INTERBTC_CLIENTS_IMAGE.to_string()
    }

    fn tag(&self) -> String {
        #[cfg(feature = "brooklyn")]
        let t = DEFAULT_INTERBTC_BROOKLYN_TAG;

        #[cfg(feature = "sydney")]
        let t = DEFAULT_INTERBTC_SYDNEY_TAG;

        t.to_string()
    }

    fn ready_conditions(&self) -> Vec<testcontainers::core::WaitFor> {
        vec![testcontainers::core::WaitFor::Duration {
            // wait 2 seconds for the container to be ready
            length: Duration::from_secs(2),
            // NOTE: this single Image is used for oracle,faucet,vault so do not put WaitFor tool-specific logs here
        }]
    }
}

pub struct InterbtcClientsContainer<'d>(pub Container<'d, InterbtcClientsImage>);
