use testcontainers::{
    core::{Image, WaitFor},
    ContainerAsync, ImageArgs,
};

use crate::vecs;

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
    pub container: ContainerAsync<GgxNodeImage>,
    pub host_network: bool,
}

impl GgxNodeContainer {
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

#[cfg(test)]
mod tests {
    use testcontainers::runners::AsyncRunner;
    use testcontainers::RunnableImage;

    use super::{GgxNodeContainer, GgxNodeImage};

    #[tokio::test]
    async fn test_ggx_node() {
        env_logger::builder().try_init().expect("init");
        let image: RunnableImage<GgxNodeImage> = GgxNodeImage::brooklyn().into();
        let node = GgxNodeContainer {
            container: image.start().await,
            host_network: false,
        };

        let host = node.get_host();
        let port = node.get_rpc_port().await;
        println!("Node is running at {}:{}", host, port);
        assert_ne!(port, 9944); // port will be random
    }
}
