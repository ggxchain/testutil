use std::time::Duration;

use testcontainers::{Container, Image};

#[derive(Clone, Debug, Default)]
pub struct InterbtcClientsImage {
    pub image: String,
    pub tag: String,
    pub wait_for: Vec<testcontainers::core::WaitFor>,
}

const DEFAULT_INTERBTC_CLIENTS_IMAGE: &str = "ggxdocker/interbtc-clients";

pub enum InterbtcClientsNetwork {
    Brooklyn,
    Sydney,
}
impl InterbtcClientsNetwork {
    pub fn as_str(&self) -> &'static str {
        match *self {
            InterbtcClientsNetwork::Brooklyn => "brooklyn-022a15afe51ae2e9c0ef18bc7f587bc6166865b7",
            InterbtcClientsNetwork::Sydney => "sydney-022a15afe51ae2e9c0ef18bc7f587bc6166865b7",
        }
    }
}

impl InterbtcClientsImage {
    pub fn brooklyn() -> Self {
        Self {
            image: DEFAULT_INTERBTC_CLIENTS_IMAGE.to_string(),
            tag: InterbtcClientsNetwork::Brooklyn.as_str().to_string(),
            wait_for: vec![],
        }
    }

    pub fn sydney() -> Self {
        Self {
            image: DEFAULT_INTERBTC_CLIENTS_IMAGE.to_string(),
            tag: InterbtcClientsNetwork::Sydney.as_str().to_string(),
            wait_for: vec![],
        }
    }
}

impl Image for InterbtcClientsImage {
    type Args = Vec<String>;

    fn name(&self) -> String {
        self.image.to_string()
    }

    fn tag(&self) -> String {
        self.tag.clone()
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
