mod metadata;

use hex::ToHex;
use metadata::ggx::runtime_types::{
    interbtc_primitives::{oracle::Key, CurrencyId, TokenSymbol},
    sp_arithmetic::fixed_point::FixedU128,
};
use std::time::Duration;
use subxt::{OnlineClient, PolkadotConfig};
use subxt_signer::sr25519::dev;
use testcontainers::core::WaitFor;
use testcontainers::runners::AsyncRunner;
use testcontainers::RunnableImage;
use testutil::containers::{
    btc::{
        bitcoincore_rpc::{
            bitcoin::{Address, Amount, Network, Script, Txid},
            Client as RpcClient, RpcApi,
        },
        BtcNodeContainer, BtcNodeImage,
    },
    ggx::{GgxNodeArgs, GgxNodeContainer, GgxNodeImage},
    interbtc_clients::{InterbtcClientsContainer, InterbtcClientsImage},
};
use tokio::time::timeout;

async fn start_btc() -> BtcNodeContainer {
    log::info!("Starting Bitcoin");
    let image = BtcNodeImage::default();
    let image = RunnableImage::from(image)
        .with_network("host")
        .with_container_name("bitcoin");
    BtcNodeContainer(image.start().await)
}

async fn start_vault(btc: &BtcNodeContainer, ggx_ws: String) -> InterbtcClientsContainer {
    log::info!("Starting Vault");

    let args = [
        "vault",
        "--no-prometheus",
        "--restart-policy=never",
        format!("--btc-parachain-url={}", ggx_ws).as_str(),
        "--auto-register=GGXT=500000000",
        "--bitcoin-connection-timeout-ms=300",
        "--bitcoin-rpc-url=http://127.0.0.1:18443",
        "--bitcoin-rpc-user",
        btc.get_username().as_str(),
        "--bitcoin-rpc-pass",
        btc.get_password().as_str(),
        "--keyring=alice",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect();

    let mut image = InterbtcClientsImage::brooklyn();
    image.wait_for.push(WaitFor::message_on_stderr(
        "vault::relay: Initializing at height",
    ));
    let image = (image, args);
    let image = RunnableImage::from(image)
        .with_network("host")
        .with_container_name("vault");
    InterbtcClientsContainer(image.start().await)
}

async fn set_oracle_exchange_rate(api: &OnlineClient<PolkadotConfig>) {
    // use subxt to connect to the parachain and set the exchange rate for GGXT.
    // normally `oracle` component does that, but in our setup it is not available.
    let tx = metadata::ggx::tx().oracle().feed_values(vec![(
        Key::ExchangeRate(CurrencyId::Token(TokenSymbol::GGXT)),
        FixedU128(1_000_000_000_000_000_000u128),
    )]);

    let pair = dev::alice();

    let wait = api
        .tx()
        .sign_and_submit_then_watch_default(&tx, &pair)
        .await
        .expect("cannot submit tx");

    wait.wait_for_finalized_success().await.unwrap();
}

async fn get_best_btc_block_hash(api: &OnlineClient<PolkadotConfig>) -> Option<String> {
    let query = metadata::ggx::storage().btc_relay().best_block();
    let result = api
        .storage()
        .at_latest()
        .await
        .expect("cannot get storage at latest")
        .fetch(&query)
        .await
        .expect("cannot get btc best block");

    result.map(|b| hex::encode_upper(b.content))
}

async fn wait_for_btc_tree_sync(
    bitcoin_api: &RpcClient,
    api: &OnlineClient<PolkadotConfig>,
    timeout_duration: Duration,
) {
    timeout(timeout_duration, async move {
        loop {
            // get last BTC block hash in Bitcoin
            let btc_best: String = bitcoin_api
                .get_best_block_hash()
                .unwrap()
                .encode_hex_upper();

            let best_btc_ggx = get_best_btc_block_hash(api).await;
            if let Some(btc_best_ggx) = best_btc_ggx {
                log::debug!(
					"Waiting for the parachain to ingest the last BTC block... Current: {}. BTC Best: {}",
					btc_best_ggx, btc_best
				);

                if btc_best_ggx == btc_best {
                    break;
                }
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }

        log::info!("Parachain and Bitcoin bitcoin best blocks are in sync")
    })
    .await
    .expect("timeout waiting for btc tree sync");
}

fn create_btc_address_with_50btc(bitcoin: &BtcNodeContainer) -> Address {
    // without this we cannot create new address
    let bitcoin_api = bitcoin.api_with_host_network(None);
    bitcoin_api
        .create_wallet("test", None, None, None, None)
        .expect("failed to create wallet");

    // specify the wallet for next wallet operations
    let bitcoin_api = bitcoin.api_with_host_network(Some("wallet/test"));

    let address = bitcoin_api
        .get_new_address(Some("test"), None)
        .expect("Failed to get new address")
        .require_network(Network::Regtest)
        .expect("Should use regtest network");

    // mine ourselves 50 BTC (coinbase is unlocked after 100 confirmations)
    bitcoin_api.generate_to_address(101, &address).unwrap();
    let balance = bitcoin_api.get_balance(None, None).unwrap();
    assert_eq!(balance.to_btc(), 50.0);

    address
}

async fn wait_until_btc_tx_finalized(
    bitcoin_api: &RpcClient,
    txid: &Txid,
    confirmations: i32,
    timeout_duration: Duration,
) {
    timeout(timeout_duration, async move {
        loop {
            let tx = bitcoin_api.get_transaction(txid, None).unwrap();
            if tx.info.confirmations >= confirmations {
                log::info!(
                    "BTC tx {} is finalized with {} confirmations (block {}:{:?})",
                    txid,
                    tx.info.confirmations,
                    tx.info.blockheight.unwrap(),
                    tx.info.blockhash.unwrap()
                );
                break;
            }

            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    })
    .await
    .expect("timeout waiting for btc tx to be finalized")
}

const AMOUNT: u64 = 500_000u64;

async fn deposit_btc_to_ggx(
    bitcoin_api: &RpcClient,
    api: &OnlineClient<PolkadotConfig>,
    bitcoin_user_address: &Address,
) {
    log::info!("Depositing some BTC to GGX");

    use metadata::ggx::runtime_types::interbtc_primitives::{VaultCurrencyPair, VaultId};

    let ggxt = CurrencyId::Token(TokenSymbol::GGXT);
    let kbtc = CurrencyId::Token(TokenSymbol::KBTC);

    let alice = dev::alice();
    let vault_id = VaultId {
        account_id: alice.public_key().to_account_id(),
        currencies: VaultCurrencyPair {
            collateral: ggxt,
            wrapped: kbtc,
        },
    };

    let tx = metadata::ggx::tx().issue().request_issue(
        AMOUNT.into(),
        vault_id,
        CurrencyId::Token(TokenSymbol::GGXT),
    );

    let wait = api
        .tx()
        .sign_and_submit_then_watch_default(&tx, &alice)
        .await
        .expect("cannot submit tx");

    let events = match wait.wait_for_finalized_success().await {
        Ok(result) => result,
        Err(err) => testutil::handle_tx_error(err),
    };

    let e = events
        .find_first::<metadata::ggx::issue::events::RequestIssue>()
        .expect("no RequestIssue event")
        .expect("Option is None");

    let amount = Amount::from_sat(AMOUNT);
    let script_pub_key = e.vault_address.0.to_script_pub_key();
    let script = Script::from_bytes(script_pub_key.as_bytes());
    let addr = Address::from_script(script, Network::Regtest).expect("bad address");
    let txid = bitcoin_api
        .send_to_address(
            &addr,
            amount,
            Some("deposit"),
            None,
            None,
            None,
            Some(6),
            None,
        )
        .expect("failed to send to address");

    // wait a bit
    tokio::time::sleep(Duration::from_secs(2)).await;

    // mine 10 new blocks to include txid into a block + mine some blocks on top of it
    bitcoin_api
        .generate_to_address(10, bitcoin_user_address)
        .unwrap();

    // check if tx is included in a block
    wait_until_btc_tx_finalized(bitcoin_api, &txid, 6, Duration::from_secs(60)).await;
}

async fn get_token_balance(
    api: &OnlineClient<PolkadotConfig>,
    account_id: subxt::utils::AccountId32,
    token: CurrencyId,
) -> Option<metadata::ggx::runtime_types::orml_tokens::AccountData<u128>> {
    let query = metadata::ggx::storage()
        .tokens()
        .accounts(account_id, token);
    api.storage()
        .at_latest()
        .await
        .expect("cannot get storage at latest")
        .fetch(&query)
        .await
        .expect("cannot get token balance")
}

async fn start_ggx() -> GgxNodeContainer {
    log::info!("Starting GGX");
    let mut args = GgxNodeArgs::default();
    args.args.push("--alice".to_string());
    let image = GgxNodeImage::brooklyn();
    let image = RunnableImage::from((image, args))
        .with_network("host")
        .with_container_name("alice");
    GgxNodeContainer(image.start().await)
}

#[cfg(test)]
mod e2e_btc_test {

    use crate::*;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[tokio::test]
    async fn e2e_btc_test() {
        init();

        // run in this order: Bitcoin, Parachain, Vault.
        let bitcoin = start_btc().await;
        let alice = start_ggx().await;
        // use subxt to connect to the parachain and set the exchange rate
        let api = OnlineClient::<PolkadotConfig>::from_url(alice.get_host_ws_url())
            .await
            .expect("failed to connect to the parachain");

        set_oracle_exchange_rate(&api).await;

        // let _faucet = start_faucet(&docker);
        let _vault = start_vault(&bitcoin, alice.get_host_ws_url()).await;

        let bitcoin_api = bitcoin.api_with_host_network(None);
        let address = create_btc_address_with_50btc(&bitcoin);

        // wait for the parachain to ingest the last BTC block (at most 60 sec).
        // at this point vault should initialize GGX BTC tree with last block (101).
        wait_for_btc_tree_sync(&bitcoin_api, &api, Duration::from_secs(60)).await;

        // mine another 20 blocks. Vault should send them to GGX automatically, 16 blocks at most at a time.
        // vault will send 2 batches...
        bitcoin_api.generate_to_address(20, &address).unwrap();

        // wait for sync again, to confirm that vault
        wait_for_btc_tree_sync(&bitcoin_api, &api, Duration::from_secs(60)).await;

        // transfer BTC to GGX (TBTC)
        deposit_btc_to_ggx(
            &bitcoin.api_with_host_network(Some("wallet/test")),
            &api,
            &address,
        )
        .await;

        // and wait again...
        wait_for_btc_tree_sync(&bitcoin_api, &api, Duration::from_secs(60)).await;

        // wait for ExecuteIssue event
        let e = testutil::wait_for_event::<metadata::ggx::issue::events::ExecuteIssue>(
            &api,
            Duration::from_secs(60),
        )
        .await;
        log::warn!("ExecuteIssue found: {:?}", e);

        // check if Alice has KBTC that we deposited
        let alice_pair = dev::alice();
        let balance = get_token_balance(
            &api,
            alice_pair.public_key().to_account_id(),
            CurrencyId::Token(TokenSymbol::KBTC),
        )
        .await;

        match balance {
            Some(b) => {
                assert!(b.free > 0);
                // we should deduct fees
                assert!(b.free < AMOUNT.into());
            }
            None => panic!("Alice's KBTC balance is None"),
        };
    }
}
