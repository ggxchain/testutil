#[cfg(test)]
mod ibc {
    use std::time::Duration;
    use testcontainers::core::WaitFor;
    use testcontainers::runners::AsyncRunner;
    use testcontainers::{Image, RunnableImage};
    use testutil::containers::cosmos::{CosmosContainer, CosmosImage};
    use testutil::containers::ggx::{GgxNodeArgs, GgxNodeContainer, GgxNodeImage};
    use testutil::containers::hermes::{HermesArgs, HermesContainer, HermesImage};
    use testutil::vecs;

    fn init() {
        let _ = env_logger::builder().try_init();
    }

    async fn start_ggx() -> GgxNodeContainer {
        log::info!("Starting GGX");
        let mut args = GgxNodeArgs::default();
        args.args.extend(vecs![
            "--alice",
            "--enable-offchain-indexing=true",
            "-lpallet_ibc=trace",
            "-lpallet-ics20-transfer=trace",
            "--detailed-log-output"
        ]);

        let image = GgxNodeImage::brooklyn();
        let image = RunnableImage::from((image, args))
            .with_network("host")
            .with_container_name("alice");
        GgxNodeContainer(image.start().await)
    }

    async fn start_cosmos() -> CosmosContainer {
        log::info!("Starting Cosmos");
        let image = CosmosImage::default();
        let image = RunnableImage::from(image)
            .with_network("host")
            .with_container_name("cosmos");
        CosmosContainer(image.start().await)
    }

    async fn start_hermes() -> HermesContainer {
        log::info!("Starting HERMES");
        tokio::time::timeout(Duration::from_secs(60 * 5), async {
            let image = HermesImage {
                wait_for: vec![
                    WaitFor::message_on_stderr("Hermes has started"),
                ]
            };
            let args = vecs!["bash", "-xce", r#"
echo ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
echo ADDING KEYS
hermes --config config/cos_sub.toml keys add --chain earth-0 --key-file config/alice_cosmos_key.json --key-name alice
hermes --config config/cos_sub.toml keys add --chain rococo-0 --key-file config/bob_substrate_key.json --key-name Bob

echo ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
echo CREATING CHANNEL
hermes --config config/cos_sub.toml create channel --a-chain earth-0 --b-chain rococo-0 --a-port transfer --b-port transfer --new-client-connection --yes

sleep 5

echo ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
echo STARTING HERMES
hermes --config config/cos_sub.toml start
"#];
            let image: RunnableImage<HermesImage> = image.into();
            let image = image.with_args(HermesArgs {
                args
            })
                .with_network("host")
                .with_container_name("hermes");

            HermesContainer(image.start().await)
        }).await.expect("hermes timed out")
    }

    #[tokio::test]
    async fn test_cosmos_deposit() {
        init();
        let alice = start_ggx().await;
        let cosmos = start_cosmos().await;
        let hermes = start_hermes().await;
        log::info!("Running the test...")
        // TODO
    }
}
