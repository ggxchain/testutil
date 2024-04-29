use testcontainers::{
    core::{Image, WaitFor},
    ImageArgs,
};

use testcontainers::ContainerAsync;

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct CosmosImage {}

#[derive(Debug, Clone)]
pub struct CosmosArgs {
    args: Vec<String>,
}
impl Default for CosmosArgs {
    fn default() -> Self {
        Self {
            args: vec!["ignite", "chain", "serve", "-f", "-v", "-c", "earth.yml"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}

impl ImageArgs for CosmosArgs {
    fn into_iterator(self) -> Box<dyn Iterator<Item=String>> {
        Box::new(self.args.into_iter())
    }
}

impl Image for CosmosImage {
    type Args = CosmosArgs;

    fn name(&self) -> String {
        "ggxdocker/cosmos".to_string()
    }

    fn tag(&self) -> String {
        "v1".to_string()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stderr("starting node with ABCI Tendermint in-process")
        ]
    }

    fn expose_ports(&self) -> Vec<u16> {
        vec![
            26657, // tendermint node
            1316,  // blockchain API
            4500,  // token faucet
        ]
    }
}

pub struct CosmosContainer(pub ContainerAsync<CosmosImage>);

#[cfg(test)]
mod tests {
    use super::*;
    use testcontainers::runners::AsyncRunner;

    #[tokio::test]
    async fn test_cosmos() {
        let image = CosmosImage::default();
        let node = image.start().await;
        let _node = CosmosContainer(node);
    }
}
