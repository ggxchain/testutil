#[cfg(test)]
mod ibc {
    use rust_decimal::Decimal;
    use std::time::Duration;

    use futures::join;
    use subxt::tx::Signer;
    use subxt::utils::{AccountId32, MultiAddress};
    use subxt::{OnlineClient, PolkadotConfig};
    use subxt_signer::sr25519::dev;
    use subxt_signer::sr25519::{Keypair, PublicKey};
    use testcontainers::core::{CmdWaitFor, WaitFor};
    use testcontainers::runners::AsyncRunner;
    use testcontainers::RunnableImage;
    use testutil::containers::cosmos::start_cosmos;
    use testutil::containers::ggx::dex_pallet::DexPallet;
    use testutil::containers::ggx::{start_ggx, GgxNodeContainer};
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

    async fn deposit_cosmos_to_ggx(
        hermes: &HermesContainer,
        deposit_amount: u128,
        denom: String,
    ) {
        let cmd = vecs![
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
            deposit_amount.to_string(),
            "--denom",
            denom
        ];

        hermes
            .exec(
                cmd,
                CmdWaitFor::message_on_stdout_or_stderr("SUCCESS"),
                Duration::from_secs(60), // timeout
            )
            .await;

        // wait for auto relay by hermes, about 30s
        log::info!("Waiting 30 sec...");
        tokio::time::sleep(Duration::from_secs(30)).await;
    }

    async fn withdraw_ggx_to_cosmos(
        alice: &GgxNodeContainer,
        hermes: &HermesContainer,
        withdraw_amount: u128,
    ) {
        log::info!("Get denom hash to withdraw");
        let denom = alice.get_denom_trace().await;
        log::info!("Got this denom: {}", denom);

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
                    withdraw_amount.to_string()
                ],
                CmdWaitFor::message_on_stdout_or_stderr("SUCCESS"),
                Duration::from_secs(60), // timeout
            )
            .await;

        // wait 30 sec
        log::info!("Waiting 30 sec...");
        tokio::time::sleep(Duration::from_secs(30)).await;
    }

    const BOB_GGX_ADDRESS: &str = "5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty";
    const ALICE_COSMOS_ADDRESS: &str = "cosmos1xh2jvz9ecty8qdctlgscmys2dr5gz729k0l7x4";
    const GGX_ASSET_A: u32 = 666;
    const GGX_ASSET_A_NAME: &str = "ERT";
    const GGX_ASSET_B: u32 = 777;
    const GGX_ASSET_B_NAME: &str = "USDT";

    #[tokio::test]
    async fn test_cosmos_ggx_deposit_withdraw_sunny_day() {
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

        log::info!("Creating cross asset");
        alice
            .create_cross_asset(dev::bob(), GGX_ASSET_A, 10_u128)
            .await;

        // DEPOSIT COSMOS --> GGX

        // transfer from earth to ggx rococo
        // hermes --config config/cos_sub.toml tx ft-transfer --timeout-height-offset 1000 --number-msgs 1 --dst-chain rococo-0 --src-chain earth-0 --src-port transfer --src-channel channel-0 --amount 999000 --denom ERT
        log::info!(
            "Transfer from earth (cosmos) to rococo (ggx) of {} {} to addr {}",
            BOB_DEPOSIT_AMOUNT,
            GGX_ASSET_A_NAME,
            BOB_GGX_ADDRESS
        );
        const BOB_DEPOSIT_AMOUNT: u128 = 999000;
        deposit_cosmos_to_ggx(
            &hermes,
            BOB_DEPOSIT_AMOUNT,
            GGX_ASSET_A_NAME.to_string(),
        )
        .await;

        let current_alice_cosmos_balances = cosmos
            .get_bank_balances_by_address(ALICE_COSMOS_ADDRESS)
            .await
            .expect("cannot get alice balances");

        // alice balance on cosmos changed!
        assert_ne!(init_alice_cosmos_balances, current_alice_cosmos_balances);

        let bob_asset = alice
            .get_ggx_asset(dev::bob().public_key().into(), GGX_ASSET_A)
            .await
            .expect("unable to get Bob's GGX_CROSS_ASSET_ID asset");
        assert_eq!(bob_asset.balance, BOB_DEPOSIT_AMOUNT);
        log::info!(
            "Deposit is successful! Bob has {} of asset {} on GGX",
            BOB_DEPOSIT_AMOUNT,
            GGX_ASSET_A
        );

        // WITHDRAW GGX --> COSMOS

        log::info!("Transfer from rococo (GGX) to earth (Cosmos)");
        // hermes --config config/cos_sub.toml tx ft-transfer --timeout-height-offset 1000 --denom ibc/972368C2A53AAD83A3718FD4A43522394D4B5A905D79296BF04EE80565B595DF  --dst-chain earth-0 --src-chain rococo-0 --src-port transfer --src-channel channel-0 --amount 999000
        const BOB_WITHDRAW_AMOUNT: u128 = 500000;
        withdraw_ggx_to_cosmos(&alice, &hermes, BOB_WITHDRAW_AMOUNT).await;

        // check that Bob has correct amount after we have withdrawn a bit
        let bob_asset = alice
            .get_ggx_asset(dev::bob().public_key().into(), GGX_ASSET_A)
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
            .find(|s| s.denom == GGX_ASSET_A_NAME)
            .expect("invalid balance");
        assert_eq!(
            alice_ert_balance.amount,
            Decimal::from_str_exact("199501000").unwrap()
        );
    }

    #[tokio::test]
    async fn test_limit_order_between_two_cosmos_wrapped_tokens_sunny_day() {
        init();

        let (alice, cosmos) = join!(
            start_ggx(vecs!["--alice", "--enable-offchain-indexing=true"]),
            start_cosmos()
        );
        // hermes connects to alice and cosmos, must be started after them...
        let hermes = start_hermes().await;

        log::info!("Creating cross assets A and B");
        const ALICE_A_BALANCE: u128 = 100;
        const BOB_B_BALANCE: u128 = 1000;
        alice
            .create_cross_asset(dev::alice(), GGX_ASSET_A, ALICE_A_BALANCE)
            .await;
        alice
            .create_cross_asset(dev::bob(), GGX_ASSET_B, BOB_B_BALANCE)
            .await;

        log::info!(
            "Alice has asset A={}, Bob has asset B={}, but they are not deposited to DEX yet",
            ALICE_A_BALANCE,
            BOB_B_BALANCE
        );
        assert!(alice
            .dex_balance_of(dev::alice(), GGX_ASSET_A)
            .await
            .is_none());
        assert!(alice
            .dex_balance_of(dev::bob(), GGX_ASSET_B)
            .await
            .is_none());

        log::info!("Now, Alice and Bob deposit all of their amount of A and B to DEX");
        alice
            .dex_deposit(dev::alice(), GGX_ASSET_A, ALICE_A_BALANCE)
            .await;
        alice
            .dex_deposit(dev::bob(), GGX_ASSET_B, BOB_B_BALANCE)
            .await;

        log::info!("Done, Alice and Bob deposited full amount of A and B");
        let alice_dex_balance = alice
            .dex_balance_of(dev::alice(), GGX_ASSET_A)
            .await
            .expect("can't get Alice balance");
        assert_eq!(alice_dex_balance.amount, ALICE_A_BALANCE);
        assert_eq!(alice_dex_balance.reserved, 0);
        let bob_dex_balance = alice
            .dex_balance_of(dev::bob(), GGX_ASSET_B)
            .await
            .expect("can't get Bob balance");
        assert_eq!(bob_dex_balance.amount, BOB_B_BALANCE);
        assert_eq!(bob_dex_balance.reserved, 0);
    }
}
