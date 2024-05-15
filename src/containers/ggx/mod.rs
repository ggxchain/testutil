pub mod assets_pallet;
pub mod dex_pallet;

use async_trait::async_trait;
use std::time::Duration;
use subxt::{OnlineClient, PolkadotConfig};
use subxt_signer::sr25519::Keypair;
use testcontainers::runners::AsyncRunner;
use testcontainers::{
    core::{Image, WaitFor},
    ContainerAsync, ImageArgs, RunnableImage,
};
use tokio::time::timeout;

use crate::{handle_tx_error, metadata, vecs};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GgxNodeImage {
    // these image:tag will be used
    image: String,
    tag: String,
}

// NOTE(Bohdan): update these if necessary, but do not rename variables, as fetch.sh depends on them.
const DEFAULT_GGX_IMAGE: &str = "ggxdocker/ggxnode";

pub enum GgxNodeNetwork {
    Brooklyn,
    Sydney,
}

impl GgxNodeNetwork {
    pub fn as_str(&self) -> &'static str {
        match *self {
            GgxNodeNetwork::Brooklyn => "brooklyn-9db132a",
            GgxNodeNetwork::Sydney => "sydney-9db132a",
        }
    }
}

impl GgxNodeImage {
    pub fn brooklyn() -> Self {
        Self {
            image: DEFAULT_GGX_IMAGE.to_string(),
            tag: GgxNodeNetwork::Brooklyn.as_str().to_string(),
        }
    }

    pub fn sydney() -> Self {
        Self {
            image: DEFAULT_GGX_IMAGE.to_string(),
            tag: GgxNodeNetwork::Sydney.as_str().to_string(),
        }
    }
}

impl GgxNodeImage {
    pub fn with_image(mut self, image: String) -> Self {
        self.image = image;
        self
    }

    pub fn with_tag(mut self, tag: String) -> Self {
        self.tag = tag;
        self
    }
}

impl Image for GgxNodeImage {
    type Args = GgxNodeArgs;

    fn name(&self) -> String {
        self.image.clone()
    }

    fn tag(&self) -> String {
        self.tag.clone()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr("Running JSON-RPC server: addr=")]
    }

    fn expose_ports(&self) -> Vec<u16> {
        vec![
            9944, // rpc
        ]
    }
}

#[derive(Debug, Clone)]
pub struct GgxNodeArgs {
    pub args: Vec<String>,
}

impl Default for GgxNodeArgs {
    fn default() -> Self {
        Self {
            args: vecs![
                "--rpc-external",
                "--rpc-methods=unsafe",
                "--unsafe-rpc-external",
                "--dev",
                "--rpc-port=9944",
                // disable unused features
                "--no-prometheus",
                "--no-telemetry"
            ],
        }
    }
}

impl ImageArgs for GgxNodeArgs {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>> {
        Box::new(self.args.into_iter())
    }
}

pub struct GgxNodeContainer {
    container: ContainerAsync<GgxNodeImage>,
    host_network: bool,
    api: Option<OnlineClient<PolkadotConfig>>,
}

#[async_trait]
pub trait SubstrateApi {
    fn api(&self) -> &OnlineClient<PolkadotConfig>;

    /// block current thread until an event of type T occurs
    async fn wait_for_event<T>(&self, timeout_duration: Duration) -> T
    where
        T: std::fmt::Debug + subxt::events::StaticEvent + Send,
    {
        // wait for T event
        timeout(timeout_duration, async {
            loop {
                let events = self
                    .api()
                    .events()
                    .at_latest()
                    .await
                    .expect("cannot get events");
                match events.find_first::<T>() {
                    Ok(Some(e)) => {
                        log::debug!("Event found: {:?}", e);
                        return e;
                    }
                    _ => {
                        log::debug!("Waiting for an event...");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
        })
        .await
        .expect("timeout waiting for event")
    }

    async fn send_tx_and_wait_until_finalized<T>(&self, owner: Keypair, payload: T)
    where
        T: subxt::tx::TxPayload + Sync + Send,
    {
        let wait = self
            .api()
            .tx()
            .sign_and_submit_then_watch_default(&payload, &owner)
            .await
            .expect("cannot submit tx");

        // this panics if tx was not accepted
        if let Err(e) = wait.wait_for_finalized_success().await {
            handle_tx_error(e);
        }
    }
}

#[async_trait]
impl SubstrateApi for GgxNodeContainer {
    fn api(&self) -> &OnlineClient<PolkadotConfig> {
        self.api.as_ref().unwrap()
    }
}

impl GgxNodeContainer {
    pub async fn from(container: ContainerAsync<GgxNodeImage>) -> Self {
        Self::from_inner(container, false).await
    }

    pub async fn from_with_host_network(container: ContainerAsync<GgxNodeImage>) -> Self {
        Self::from_inner(container, true).await
    }

    async fn from_inner(container: ContainerAsync<GgxNodeImage>, host_network: bool) -> Self {
        let mut result = Self {
            container,
            host_network,
            api: None,
        };

        let api = OnlineClient::<PolkadotConfig>::from_url(result.get_host_ws_url().await)
            .await
            .expect("failed to connect to the parachain");

        result.api = Some(api);

        result
    }

    pub async fn get_denom_trace(&self) -> String {
        let query = metadata::ggx::storage().ics20_transfer().denom_trace_root();

        let mut it = self
            .api()
            .storage()
            .at_latest()
            .await
            .expect("cannot get storage at latest")
            .iter(query, 100)
            .await
            .expect("cannot iter");

        fn try_find_ibc_hash(input: Vec<u8>) -> Option<Vec<u8>> {
            let needle = b"ibc/";
            input
                .windows(needle.len())
                .position(|window| window == needle)
                .map(|pos| input[pos..].to_vec())
        }

        // NOTE(Warchant): this is an uber hack. I do not know how to properly extract `ibc/{hash}`,
        // and I was not able to calculate it correctly from PrefixedDenom.
        // Here v.0.0 will be something like `{bytes garbage}ibc/{str hex hash}`. We need `ibc/{hash}`.
        while let Ok(Some(v)) = it.next().await {
            if let Some(hash) = try_find_ibc_hash(v.0 .0) {
                return String::from_utf8(hash).expect("invalid utf-8");
            }
        }

        panic!(
            "that storage key should have contained `ibc/` with a hash inside... but it doesn't"
        );
    }

    /// use this only if network is not `host`
    pub async fn get_rpc_port(&self) -> u16 {
        if self.host_network {
            9944
        } else {
            self.container.get_host_port_ipv4(9944).await
        }
    }

    /// use this only if network is not `host`
    pub fn get_host(&self) -> String {
        "127.0.0.1".to_string()
    }

    /// use this only if network is not `host`
    pub async fn get_ws_url(&self) -> String {
        format!("ws://{}:{}", self.get_host(), self.get_rpc_port().await)
    }

    pub async fn get_host_ws_url(&self) -> String {
        let wsport = self.get_rpc_port().await;
        format!("ws://{}:{}", self.get_host(), wsport)
    }
}

pub async fn start_ggx(extraargs: Vec<String>) -> GgxNodeContainer {
    log::info!("Starting GGX");
    let mut args = GgxNodeArgs::default();

    args.args.extend(extraargs);

    let image = GgxNodeImage::brooklyn();
    let image = RunnableImage::from((image, args)).with_network("host");

    GgxNodeContainer::from_with_host_network(image.start().await).await
}

#[cfg(test)]
mod tests {
    use testcontainers::runners::AsyncRunner;
    use testcontainers::RunnableImage;

    use super::{GgxNodeContainer, GgxNodeImage};

    #[tokio::test]
    async fn test_ggx_node() {
        let _ = env_logger::builder().try_init();
        let image: RunnableImage<GgxNodeImage> = GgxNodeImage::brooklyn().into();
        let node = GgxNodeContainer::from(image.start().await).await;
        let host = node.get_host();
        let port = node.get_rpc_port().await;
        println!("Node is running at {}:{}", host, port);
        assert_ne!(port, 9944); // port will be random
    }
}
