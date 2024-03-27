use testcontainers::{
    core::{Image, WaitFor},
    Container, ImageArgs,
};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct GgxNodeImage {
    // these image:tag will be used
    image: String,
    tag: String,
}

// NOTE(Bohdan): update these if necessary, but do not rename variables, as fetch.sh depends on them.
const DEFAULT_GGX_IMAGE: &str = "public.ecr.aws/k7w7q6c4/ggxchain-node";

#[allow(dead_code)]
const DEFAULT_GGX_SYDNEY_TAG: &str = "sydney-0b88ed23";
#[allow(dead_code)]
const DEFAULT_GGX_BROOKLYN_TAG: &str = "brooklyn-0b88ed23";

#[cfg(feature = "brooklyn")]
const DEFAULT_GGX_TAG: &str = DEFAULT_GGX_BROOKLYN_TAG;

#[cfg(feature = "sydney")]
const DEFAULT_GGX_TAG: &str = DEFAULT_GGX_SYDNEY_TAG;

impl Default for GgxNodeImage {
    fn default() -> Self {
        Self {
            // default image+tag
            image: DEFAULT_GGX_IMAGE.to_string(),
            tag: DEFAULT_GGX_TAG.to_string(),
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

    fn expose_ports(&self) -> Vec<u16> {
        vec![
            9944, // rpc
        ]
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![WaitFor::message_on_stderr("Running JSON-RPC server: addr=")]
    }
}

#[derive(Debug, Clone)]
pub struct GgxNodeArgs {
    pub args: Vec<String>,
}
impl Default for GgxNodeArgs {
    fn default() -> Self {
        Self {
            args: [
                "--rpc-external",
                "--rpc-methods=unsafe",
                "--unsafe-rpc-external",
                "--dev",
                "--rpc-port=9944",
                // disable unused features
                "--no-prometheus",
                "--no-telemetry",
            ]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        }
    }
}

impl ImageArgs for GgxNodeArgs {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>> {
        Box::new(self.args.into_iter())
    }
}

pub struct GgxNodeContainer<'d>(pub Container<'d, GgxNodeImage>);
impl<'d> GgxNodeContainer<'d> {
    /// use this only if network is not `host`
    pub fn get_rpc_port(&self) -> u16 {
        self.0.get_host_port_ipv4(9944)
    }

    /// use this only if network is not `host`
    pub fn get_host(&self) -> String {
        "127.0.0.1".to_string()
    }

    /// use this only if network is not `host`
    pub fn get_ws_url(&self) -> String {
        format!("ws://{}:{}", self.get_host(), self.get_rpc_port())
    }

    pub fn get_host_ws_url(&self) -> String {
        format!("ws://{}:9944", self.get_host())
    }
}

#[cfg(test)]
mod tests {
    use super::{GgxNodeContainer, GgxNodeImage};
    use testcontainers::{clients::Cli, RunnableImage};

    #[tokio::test]
    async fn test_ggx_node() {
        env_logger::init();
        let docker = Cli::default();
        let image: RunnableImage<GgxNodeImage> = GgxNodeImage::default().into();
        let node = GgxNodeContainer(docker.run(image));

        let host = node.get_host();
        let port = node.get_rpc_port();
        println!("Node is running at {}:{}", host, port);
        assert_ne!(port, 9944); // port will be random
    }
}
