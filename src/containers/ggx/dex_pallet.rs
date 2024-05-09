use crate::containers::ggx::{GgxNodeContainer, SubstrateApi};
use crate::metadata;
use async_trait::async_trait;
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
}

#[async_trait]
impl DexPallet for GgxNodeContainer {}
