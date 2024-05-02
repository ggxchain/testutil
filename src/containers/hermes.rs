use std::time::Duration;

use testcontainers::core::{CmdWaitFor, ExecCommand, Image, WaitFor};
use testcontainers::{ContainerAsync, ImageArgs};

use crate::vecs;

#[derive(Debug, Eq, Clone, PartialEq)]
pub struct HermesArgs {
    pub args: Vec<String>,
}

impl Default for HermesArgs {
    fn default() -> Self {
        Self {
            args: vecs!["bash", "-c", "while sleep 60; do echo ALIVE; done"],
        }
    }
}

impl ImageArgs for HermesArgs {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>> {
        // keep container busy
        Box::new(self.args.into_iter())
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct HermesImage {
    pub wait_for: Vec<WaitFor>,
}

//
impl Image for HermesImage {
    type Args = HermesArgs;

    fn name(&self) -> String {
        "ggxdocker/hermes".to_string()
    }

    fn tag(&self) -> String {
        "v1".to_string()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        // ready immediately
        self.wait_for.clone()
    }
}

pub struct HermesContainer(pub ContainerAsync<HermesImage>);

impl HermesContainer {
    pub async fn exec(&self, cmd: Vec<String>, wait_for: CmdWaitFor, timeout: Duration) {
        tokio::time::timeout(timeout, async {
            let c = ExecCommand::new(cmd).with_cmd_ready_condition(wait_for);
            self.0.exec(c).await;
        })
        .await
        .expect("cmd timed out");
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use testcontainers::runners::AsyncRunner;

    use crate::vecs;

    use super::*;

    #[tokio::test]
    async fn test_hermes() {
        let container = HermesImage::default().start().await;
        let container = HermesContainer(container);
        let cmd = vecs![
            "hermes",
            "--config",
            "config/cos_sub.toml",
            "keys",
            "add",
            "--chain",
            "earth-0",
            "--key-file",
            "config/alice_cosmos_key.json",
            "--key-name",
            "alice"
        ];

        container
            .exec(
                cmd,
                WaitFor::message_on_stdout("SUCCESS Added key 'alice'").into(),
                Duration::from_secs(60),
            )
            .await;
    }
}
