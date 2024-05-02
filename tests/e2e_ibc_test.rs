mod metadata;

#[cfg(test)]
mod ibc {
    use crate::metadata;
    use std::time::Duration;
    use subxt::utils::{AccountId32, MultiAddress};
    use subxt::{OnlineClient, PolkadotConfig};
    use subxt_signer::sr25519::dev;
    use testcontainers::core::{CmdWaitFor, WaitFor};
    use testcontainers::runners::AsyncRunner;
    use testcontainers::RunnableImage;
    use testutil::containers::cosmos::{CosmosContainer, CosmosImage};
    use testutil::containers::ggx::{GgxNodeArgs, GgxNodeContainer, GgxNodeImage};
    use testutil::containers::hermes::{HermesArgs, HermesContainer, HermesImage};
    use testutil::{handle_tx_error, vecs};

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
        CosmosContainer {
            container: image.start().await,
            host_network: true,
        }
    }

    async fn start_hermes() -> HermesContainer {
        log::info!("Starting HERMES");
        tokio::time::timeout(Duration::from_secs(60 * 5), async {
            let image = HermesImage {
                wait_for: vec![
                    // first, wait for channel to open
                    WaitFor::message_on_stdout("STARTING HERMES"),
                    // then another 10 seconds
                    WaitFor::seconds(10),
                ]
            };
            let args = vecs!["bash", "-ce", r#"
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

    async fn create_cross_asset(api: &OnlineClient<PolkadotConfig>) {
        // we send tx as alice, but we pass bob's MultiAddress to assets::force_create call.
        let alice = dev::alice(); // sudoer
        let bob = dev::bob();
        let owner: MultiAddress<AccountId32, u32> = MultiAddress::Id(bob.public_key().into());

        type Call = metadata::ggx::runtime_types::ggxchain_runtime_brooklyn::RuntimeCall;
        type AssetsCall = metadata::ggx::runtime_types::pallet_assets::pallet::Call;
        let call = Call::Assets(AssetsCall::force_create {
            id: 666,
            owner,
            is_sufficient: true,
            min_balance: 10,
        });

        let sudo_tx = metadata::ggx::tx().sudo().sudo(call);

        let wait = api
            .tx()
            .sign_and_submit_then_watch_default(&sudo_tx, &alice)
            .await
            .expect("cannot submit tx");

        if let Err(e) = wait.wait_for_finalized_success().await {
            handle_tx_error(e);
        }
    }

    const ALICE_COSMOS_ADDRESS: &str = "cosmos1xh2jvz9ecty8qdctlgscmys2dr5gz729k0l7x4";

    #[tokio::test]
    async fn test_ibc() {
        init();
        let alice = start_ggx().await;
        let cosmos = start_cosmos().await;
        let hermes = start_hermes().await;
        log::info!("Starting the test...");

        let init_alice_cosmos_balances = cosmos
            .get_bank_balances_by_address(ALICE_COSMOS_ADDRESS)
            .await
            .expect("cannot get alice balance");

        let api = OnlineClient::<PolkadotConfig>::from_url(alice.get_host_ws_url())
            .await
            .expect("failed to connect to the parachain");

        log::info!("Creating cross asset");
        create_cross_asset(&api).await;

        // transfer from earth to ggx rococo
        // hermes --config config/cos_sub.toml tx ft-transfer --timeout-height-offset 1000 --number-msgs 1 --dst-chain rococo-0 --src-chain earth-0 --src-port transfer --src-channel channel-0 --amount 999000 --denom ERT
        log::info!("Transfer from earth (cosmos) to rococo (ggx)");
        hermes
            .exec(
                vecs![
                    "hermes",
                    "--config",
                    "config/cos_sub.toml",
                    "tx",
                    "ft-transfer",
                    "--timeout-height-offset",
                    "1000",
                    "--number-msgs",
                    "1",
                    "--dst-chain",
                    "rococo-0",
                    "--src-chain",
                    "earth-0",
                    "--src-port",
                    "transfer",
                    "--src-channel",
                    "channel-0",
                    "--amount",
                    "999000",
                    "--denom",
                    "ERT"
                ],
                CmdWaitFor::message_on_stdout_or_stderr("SUCCESS"),
                Duration::from_secs(60), // timeout
            )
            .await;

        // wait for auto relay by hermes, about 30s
        log::info!("Waiting 30 sec...");
        tokio::time::sleep(Duration::from_secs(30)).await;

        let current_alice_cosmos_balances = cosmos
            .get_bank_balances_by_address(ALICE_COSMOS_ADDRESS)
            .await
            .expect("cannot get alice balances");

        assert_ne!(init_alice_cosmos_balances, current_alice_cosmos_balances);
    }
}
