use testcontainers::{
    core::{Image, WaitFor},
    ImageArgs,
};

use testcontainers::Container;

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

impl Image for CosmosImage {
    type Args = CosmosArgs;

    fn name(&self) -> String {
        "ggxdocker/cosmos".to_string()
    }

    fn tag(&self) -> String {
        "v1".to_string()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![]
    }
}

pub struct CosmosContainer<'a>(pub Container<'a, CosmosImage>);

#[cfg(test)]
mod tests {
    use std::thread;
    use std::time::Duration;
    use super::*;
    use testcontainers::clients::Cli;

    #[test]
    fn test_cosmos() {
        let docker = Cli::default();
        let image = CosmosImage::default();
        let node = docker.run(image);
        let node = CosmosContainer(node);

        // sleep 10s
        thread::sleep(Duration::from_secs(100));
    }
}
