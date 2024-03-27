pub mod containers;
pub mod metadata;

// re-export publicly
pub use testcontainers::{clients::Cli, Container};

use log;
use std::time::Duration;
use subxt;
use tokio::time::timeout;

pub fn handle_tx_error(e: subxt::Error) -> ! {
    match e {
        subxt::Error::Runtime(subxt::error::DispatchError::Module(error)) => {
            let details = error.details().expect("cannot get details");
            let pallet = details.pallet.name();
            let error = &details.variant;
            panic!("Extrinsic failed with an error: {pallet}::{error:?}")
        }
        _ => {
            panic!("Extrinsic failed with an error: {}", e)
        }
    };
}

pub async fn wait_for_event<T>(
    api: &subxt::OnlineClient<subxt::PolkadotConfig>,
    timeout_duration: Duration,
) -> T
where
    T: std::fmt::Debug + subxt::events::StaticEvent,
{
    // wait for ExecuteIssue event
    timeout(timeout_duration, async {
        loop {
            let events = api.events().at_latest().await.expect("cannot get events");
            match events.find_first::<T>() {
                Ok(Some(e)) => {
                    log::info!("Event found: {:?}", e);
                    return e;
                }
                _ => {
                    log::debug!("Waiting for an event...");
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        }
    })
    .await
    .expect("timeout waiting for event")
}
