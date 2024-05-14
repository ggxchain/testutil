#[cfg(test)]
mod dex {

    use subxt_signer::sr25519::dev;

    use testutil::containers::ggx::assets_pallet::AssetsPallet;
    use testutil::containers::ggx::dex_pallet::DexPallet;
    use testutil::containers::ggx::start_ggx;

    use testutil::metadata::ggx::runtime_types::pallet_dex::OrderType;
    use testutil::vecs;

    fn init() {
        let _ = env_logger::builder().try_init();
    }

    const GGX_ASSET_A: u32 = 666;
    const GGX_ASSET_B: u32 = 777;

    #[tokio::test]
    async fn test_limit_order_between_two_assets_sunny_day() {
        init();

        let alice = start_ggx(vecs!["--alice", "--enable-offchain-indexing=true"]).await;

        log::info!("Creating cross assets A and B");
        const ALICE_A_BALANCE: u128 = 100;
        const BOB_B_BALANCE: u128 = 1000;
        alice.asset_force_create(dev::alice(), GGX_ASSET_A, 0).await;
        alice.asset_force_create(dev::bob(), GGX_ASSET_B, 0).await;

        alice
            .asset_mint(dev::alice(), GGX_ASSET_A, ALICE_A_BALANCE)
            .await;
        alice
            .asset_mint(dev::bob(), GGX_ASSET_B, BOB_B_BALANCE)
            .await;

        let alice_balance = alice
            .asset_get_balance(dev::alice(), GGX_ASSET_A)
            .await
            .expect("should have balance");
        assert_eq!(alice_balance.balance, ALICE_A_BALANCE);
        let bob_balance = alice
            .asset_get_balance(dev::bob(), GGX_ASSET_B)
            .await
            .expect("should have balance");
        assert_eq!(bob_balance.balance, BOB_B_BALANCE);

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

        log::info!("Alice wants to trade A for B (sell 10A for 50B)");
        let alice_order_id = alice
            .dex_make_order(
                dev::alice(),
                GGX_ASSET_A,
                GGX_ASSET_B,
                10,
                50,
                OrderType::SELL,
                u32::MAX,
            )
            .await;

        // list orders
        let orders = alice.dex_get_orders().await;
        assert_eq!(orders.len(), 1);
        assert_eq!(orders[0].counter, 0);
        assert_eq!(orders[0].amount_offered, 10);
        assert_eq!(orders[0].amout_requested, 50);
        assert_eq!(orders[0].pair, (GGX_ASSET_A, GGX_ASSET_B));

        log::info!("Bob fulfills order id={}", alice_order_id);
        alice.dex_take_order(dev::bob(), alice_order_id).await;

        log::info!("Done, order id={} executed", alice_order_id);
        let alice_dex_balance = alice
            .dex_balance_of(dev::alice(), GGX_ASSET_A)
            .await
            .expect("can't get Alice balance");
        assert_eq!(alice_dex_balance.amount, ALICE_A_BALANCE - 10);
        let alice_dex_balance = alice
            .dex_balance_of(dev::alice(), GGX_ASSET_B)
            .await
            .expect("can't get Alice balance");
        assert_eq!(alice_dex_balance.amount, 50);
        let bob_dex_balance = alice
            .dex_balance_of(dev::bob(), GGX_ASSET_A)
            .await
            .expect("can't get Bob balance");
        assert_eq!(bob_dex_balance.amount, 10);

        log::info!("Balances verified. Now Bob creates an order and cancels it");
        let _id1 = alice
            .dex_make_order(
                dev::bob(),
                GGX_ASSET_B,
                GGX_ASSET_A,
                2,
                1,
                OrderType::SELL,
                u32::MAX,
            )
            .await;
        let _id2 = alice
            .dex_make_order(
                dev::bob(),
                GGX_ASSET_B,
                GGX_ASSET_A,
                3,
                2,
                OrderType::SELL,
                u32::MAX,
            )
            .await;
        let id3 = alice
            .dex_make_order(
                dev::bob(),
                GGX_ASSET_B,
                GGX_ASSET_A,
                4,
                3,
                OrderType::SELL,
                u32::MAX,
            )
            .await;

        let orders = alice.dex_get_orders().await;
        assert_eq!(orders.len(), 3);
        orders.iter().find(|s| s.amount_offered == 2).unwrap();
        orders.iter().find(|s| s.amount_offered == 3).unwrap();
        orders.iter().find(|s| s.amount_offered == 4).unwrap();

        log::info!("Bob cancels order={}", id3);
        alice.dex_cancel_order(dev::bob(), id3).await;

        let orders = alice.dex_get_orders().await;
        assert_eq!(orders.len(), 2);
        orders.iter().find(|s| s.amount_offered == 2).unwrap();
        orders.iter().find(|s| s.amount_offered == 3).unwrap();
    }
}
