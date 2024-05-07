mod metadata;

#[cfg(test)]
mod ibc {
    use crate::metadata;

    use crate::metadata::ggx::runtime_types::pallet_assets::types::AssetAccount;
    use rust_decimal::Decimal;
    use std::time::Duration;

    use futures::join;
    use subxt::utils::{AccountId32, MultiAddress};
    use subxt::{OnlineClient, PolkadotConfig};
    use subxt_signer::sr25519::dev;
    use testcontainers::core::{CmdWaitFor, WaitFor};
    use testcontainers::runners::AsyncRunner;
    use testcontainers::RunnableImage;
    use testutil::containers::cosmos::start_cosmos;
    use testutil::containers::ggx::start_ggx;
    use testutil::containers::hermes::{HermesArgs, HermesContainer, HermesImage};
    use testutil::{handle_tx_error, vecs};

    fn init() {
        let _ = env_logger::builder().try_init();
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
            id: GGX_CROSS_ASSET_ID,
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

    async fn get_ggx_asset(
        api: &OnlineClient<PolkadotConfig>,
        account_id: AccountId32,
        _asset_id: u32,
    ) -> Option<AssetAccount<u128, u128, (), AccountId32>> {
        let query = metadata::ggx::storage()
            .assets()
            .account(GGX_CROSS_ASSET_ID, account_id);

        api.storage()
            .at_latest()
            .await
            .expect("cannot get storage at latest")
            .fetch(&query)
            .await
            .expect("cannot get asset balance")
    }

    fn try_find_ibc_hash(input: Vec<u8>) -> Option<Vec<u8>> {
        let needle = b"ibc/";
        input
            .windows(needle.len())
            .position(|window| window == needle)
            .map(|pos| input[pos..].to_vec())
    }

    async fn get_denom_trace(api: &OnlineClient<PolkadotConfig>) -> String {
        let query = metadata::ggx::storage().ics20_transfer().denom_trace_root();

        let mut it = api
            .storage()
            .at_latest()
            .await
            .expect("cannot get storage at latest")
            .iter(query, 100)
            .await
            .expect("cannot iter");

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

    const ALICE_COSMOS_ADDRESS: &str = "cosmos1xh2jvz9ecty8qdctlgscmys2dr5gz729k0l7x4";
    const GGX_CROSS_ASSET_ID: u32 = 666;

    #[tokio::test]
    async fn test_cosmos_ggx_deposit_withdraw() {
        init();

        let (alice, cosmos) = join!(
            start_ggx(vecs!["--alice", "--enable-offchain-indexing=true"]),
            start_cosmos()
        );
        // hermes connects to alice and cosmos, must be started after them...
        let hermes = start_hermes().await;

        log::info!("Starting the test...");

        let init_alice_cosmos_balances = cosmos
            .get_bank_balances_by_address(ALICE_COSMOS_ADDRESS)
            .await
            .expect("cannot get alice balance");

        let api = OnlineClient::<PolkadotConfig>::from_url(alice.get_host_ws_url().await)
            .await
            .expect("failed to connect to the parachain");

        log::info!("Creating cross asset");
        create_cross_asset(&api).await;

        // DEPOSIT COSMOS --> GGX

        // transfer from earth to ggx rococo
        // hermes --config config/cos_sub.toml tx ft-transfer --timeout-height-offset 1000 --number-msgs 1 --dst-chain rococo-0 --src-chain earth-0 --src-port transfer --src-channel channel-0 --amount 999000 --denom ERT
        log::info!("Transfer from earth (cosmos) to rococo (ggx)");
        const BOB_DEPOSIT_AMOUNT: u128 = 999000;
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
                    BOB_DEPOSIT_AMOUNT.to_string(),
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

        // alice balance on cosmos changed!
        assert_ne!(init_alice_cosmos_balances, current_alice_cosmos_balances);

        let bob = dev::bob();
        let bob_asset = get_ggx_asset(&api, bob.public_key().into(), GGX_CROSS_ASSET_ID)
            .await
            .expect("unable to get Bob's GGX_CROSS_ASSET_ID asset");
        assert_eq!(bob_asset.balance, BOB_DEPOSIT_AMOUNT);
        log::info!(
            "Deposit is successful! Bob has {} of asset {} on GGX",
            BOB_DEPOSIT_AMOUNT,
            GGX_CROSS_ASSET_ID
        );

        // WITHDRAW GGX --> COSMOS
        log::info!("Get denom hash to withdraw");
        let denom = get_denom_trace(&api).await;
        log::info!("Got this denom: {}", denom);

        log::info!("Transfer from rococo (GGX) to earth (Cosmos)");
        // hermes --config config/cos_sub.toml tx ft-transfer --timeout-height-offset 1000 --denom ibc/972368C2A53AAD83A3718FD4A43522394D4B5A905D79296BF04EE80565B595DF  --dst-chain earth-0 --src-chain rococo-0 --src-port transfer --src-channel channel-0 --amount 999000
        const BOB_WITHDRAW_AMOUNT: u128 = 500000;
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
                    "--denom",
                    denom,
                    "--dst-chain",
                    "earth-0",
                    "--src-chain",
                    "rococo-0",
                    "--src-port",
                    "transfer",
                    "--src-channel",
                    "channel-0",
                    "--amount",
                    BOB_WITHDRAW_AMOUNT.to_string()
                ],
                CmdWaitFor::message_on_stdout_or_stderr("SUCCESS"),
                Duration::from_secs(60), // timeout
            )
            .await;

        // wait 30 sec
        log::info!("Waiting 30 sec...");
        tokio::time::sleep(Duration::from_secs(30)).await;

        // check that Bob has correct amount after we have withdrawn a bit
        let bob_asset = get_ggx_asset(&api, bob.public_key().into(), GGX_CROSS_ASSET_ID)
            .await
            .expect("unable to get Bob's GGX_CROSS_ASSET_ID asset");
        assert_eq!(bob_asset.balance, BOB_DEPOSIT_AMOUNT - BOB_WITHDRAW_AMOUNT);

        // check balance on Cosmos
        let alice_balances = cosmos
            .get_bank_balances_by_address(ALICE_COSMOS_ADDRESS)
            .await
            .expect("unable to get Alice balance");

        let alice_ert_balance = alice_balances
            .balances
            .iter()
            .find(|s| s.denom == "ERT")
            .expect("invalid balance");
        assert_eq!(
            alice_ert_balance.amount,
            Decimal::from_str_exact("199501000").unwrap()
        );
    }
}
