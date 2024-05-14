use crate::containers::ggx::{GgxNodeContainer, SubstrateApi};
use crate::metadata::ggx::runtime_types::pallet_dex::{Order, OrderType};
use crate::{handle_tx_error, metadata};
use async_trait::async_trait;
use std::time::Duration;
use subxt::utils::AccountId32;
use subxt_signer::sr25519::Keypair;

#[async_trait]
pub trait DexPallet: SubstrateApi {
    async fn dex_deposit(&self, owner: Keypair, asset_id: u32, amount: u128) {
        let tx = metadata::ggx::tx().dex().deposit(asset_id, amount);
        self.send_tx_and_wait_until_finalized(owner, tx).await;
    }

    async fn dex_withdraw(&self, owner: Keypair, asset_id: u32, amount: u128) {
        let tx = metadata::ggx::tx().dex().withdraw(asset_id, amount);
        self.send_tx_and_wait_until_finalized(owner, tx).await;
    }

    async fn dex_deposit_native(&self, owner: Keypair, amount: u128) {
        let tx = metadata::ggx::tx().dex().deposit_native(amount);
        self.send_tx_and_wait_until_finalized(owner, tx).await;
    }

    async fn dex_withdraw_native(&self, owner: Keypair, amount: u128) {
        let tx = metadata::ggx::tx().dex().withdraw_native(amount);
        self.send_tx_and_wait_until_finalized(owner, tx).await;
    }

    async fn dex_balance_of(
        &self,
        owner: Keypair,
        asset_id: u32,
    ) -> Option<metadata::ggx::runtime_types::pallet_dex::TokenInfo<u128>> {
        let account_id32: AccountId32 = owner.public_key().into();

        let q = metadata::ggx::storage()
            .dex()
            .user_token_infoes(account_id32, asset_id);

        self.api()
            .storage()
            .at_latest()
            .await
            .expect("cannot get storage at latest")
            .fetch(&q)
            .await
            .expect("cannot execute query")
    }

    async fn dex_get_orders(&self) -> Vec<Order<AccountId32, u128, u32>> {
        let query = metadata::ggx::storage().dex().orders_root();

        let mut it = self
            .api()
            .storage()
            .at_latest()
            .await
            .expect("cannot get storage at latest")
            .iter(query, 100)
            .await
            .expect("cannot iter");

        let mut orders = vec![];
        while let Ok(Some(v)) = it.next().await {
            orders.push(v.1);
        }
        orders
    }

    async fn dex_make_order(
        &self,
        user: Keypair,
        asset1: u32,
        asset2: u32,
        offered_amount: u128,
        requested_amount: u128,
        order_type: OrderType,
        expiration_block: u32,
    ) -> u64 {
        let tx = metadata::ggx::tx().dex().make_order(
            asset1,
            asset2,
            offered_amount,
            requested_amount,
            order_type,
            expiration_block,
        );

        let wait = self
            .api()
            .tx()
            .sign_and_submit_then_watch_default(&tx, &user)
            .await
            .expect("cannot submit tx");

        use metadata::ggx::dex::events::OrderCreated;
        let event = self
            .wait_for_event::<OrderCreated>(Duration::from_secs(60))
            .await;

        // this panics if tx was not accepted
        if let Err(e) = wait.wait_for_finalized_success().await {
            handle_tx_error(e);
        }

        event.order_index
    }

    async fn dex_cancel_order(&self, user: Keypair, order: u64) {
        let tx = metadata::ggx::tx().dex().cancel_order(order);
        self.send_tx_and_wait_until_finalized(user, tx).await;
    }

    async fn dex_take_order(&self, user: Keypair, order: u64) {
        let tx = metadata::ggx::tx().dex().take_order(order);
        self.send_tx_and_wait_until_finalized(user, tx).await;
    }
}

#[async_trait]
impl DexPallet for GgxNodeContainer {}
