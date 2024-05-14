use crate::containers::ggx::{GgxNodeContainer, SubstrateApi};
use crate::metadata;
use crate::metadata::ggx::runtime_types::pallet_assets::types::AssetAccount;
use async_trait::async_trait;
use subxt::utils::{AccountId32, MultiAddress};
use subxt_signer::sr25519::{dev, Keypair};

#[async_trait]
pub trait AssetsPallet: SubstrateApi {
    async fn asset_force_create(&self, owner: Keypair, asset_id: u32, min_balance: u128) {
        log::info!(
            "GGX: Creating asset with id={}, balance={}",
            asset_id,
            min_balance
        );

        type Call = metadata::ggx::runtime_types::ggxchain_runtime_brooklyn::RuntimeCall;
        type AssetsCall = metadata::ggx::runtime_types::pallet_assets::pallet::Call;
        let call = Call::Assets(AssetsCall::force_create {
            id: asset_id,
            owner: MultiAddress::Id(owner.public_key().into()),
            is_sufficient: true,
            min_balance,
        });

        let sudo_tx = metadata::ggx::tx().sudo().sudo(call);

        let sudoer = dev::alice(); // user with sudo
        self.send_tx_and_wait_until_finalized(sudoer, sudo_tx).await;
    }

    async fn asset_get_balance(
        &self,
        owner: Keypair,
        asset_id: u32,
    ) -> Option<AssetAccount<u128, u128, (), AccountId32>> {
        let account_id: AccountId32 = owner.public_key().into();

        let query = metadata::ggx::storage()
            .assets()
            .account(asset_id, account_id);

        self.api()
            .storage()
            .at_latest()
            .await
            .expect("cannot get storage at latest")
            .fetch(&query)
            .await
            .expect("cannot get asset balance")
    }

    async fn asset_mint(&self, owner: Keypair, asset_id: u32, amount: u128) {
        log::info!("Minting asset {} amount {}", asset_id, amount);
        let user = MultiAddress::Id(owner.public_key().into());
        let tx = metadata::ggx::tx().assets().mint(asset_id, user, amount);
        self.send_tx_and_wait_until_finalized(dev::alice(), tx)
            .await;
    }
}

#[async_trait]
impl AssetsPallet for GgxNodeContainer {}
