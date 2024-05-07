use bitcoincore_rpc::jsonrpc::serde_json;
use rust_decimal::Decimal;
use serde::Deserialize;
use testcontainers::ContainerAsync;
use testcontainers::{
    core::{Image, WaitFor},
    ImageArgs,
};

#[derive(Debug, Default, Clone, Eq, PartialEq)]
pub struct CosmosImage {}

#[derive(Debug, Clone)]
pub struct CosmosArgs {
    args: Vec<String>,
}

impl Default for CosmosArgs {
    fn default() -> Self {
        Self {
            args: vec!["ignite", "chain", "serve", "-f", "-v", "-c", "earth.yml"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}

impl ImageArgs for CosmosArgs {
    fn into_iterator(self) -> Box<dyn Iterator<Item = String>> {
        Box::new(self.args.into_iter())
    }
}

impl Image for CosmosImage {
    type Args = CosmosArgs;

    fn name(&self) -> String {
        "ggxdocker/cosmos".to_string()
    }

    fn tag(&self) -> String {
        "v1".to_string()
    }

    fn ready_conditions(&self) -> Vec<WaitFor> {
        vec![
            WaitFor::message_on_stderr("starting node with ABCI Tendermint in-process"),
            WaitFor::seconds(10),
        ]
    }

    fn expose_ports(&self) -> Vec<u16> {
        vec![
            26657, // tendermint node - rpc laddr
            1317,  // API
            9095,  // GRPC
            9096,  // GRPC-WEB
            4500,  // token faucet
        ]
    }
}

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Balance {
    pub denom: String,
    pub amount: Decimal,
}

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct BankGetAddressBalancesResponse {
    pub balances: Vec<Balance>,
    // there're also fields for pagination, but we will ignore them
}

#[derive(Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct ErrorResponse {
    pub code: i64,
    pub message: String,
}

pub struct CosmosContainer {
    pub container: ContainerAsync<CosmosImage>,
    pub host_network: bool,
}

impl CosmosContainer {
    pub async fn get_port(&self, port: u16) -> u16 {
        if self.host_network {
            port
        } else {
            self.container.get_host_port_ipv4(port).await
        }
    }
    pub async fn get_bank_balances_by_address(
        &self,
        address: &str,
    ) -> anyhow::Result<BankGetAddressBalancesResponse> {
        let url = format!(
            "http://localhost:{}/cosmos/bank/v1beta1/balances/{}",
            self.get_port(1317).await,
            address
        );

        let response: serde_json::Value = reqwest::get(&url).await?.json().await?;

        if let Ok(value) =
            serde_json::from_value::<BankGetAddressBalancesResponse>(response.clone())
        {
            Ok(value)
        } else if let Ok(error_response) = serde_json::from_value::<ErrorResponse>(response.clone())
        {
            Err(anyhow::anyhow!(
                "Error response from the API: code: {}, message: {}",
                error_response.code,
                error_response.message,
            ))
        } else {
            Err(anyhow::anyhow!(
                "Unknown response from the API: {:?}",
                response
            ))
        }
    }
}

#[cfg(test)]
mod cosmos_tests {
    use testcontainers::runners::AsyncRunner;
    use testcontainers::RunnableImage;

    use super::*;

    fn init() {
        let _ = env_logger::builder().try_init();
    }

    #[tokio::test]
    async fn test_cosmos_container() {
        init();

        let image = CosmosImage::default();
        let image = RunnableImage::from(image)
            .with_network("host")
            .with_container_name("cosmos");
        let container = image.start().await;
        let node = CosmosContainer {
            container,
            host_network: true,
        };

        const ALICE_COSMOS_ADDRESS: &str = "cosmos1xh2jvz9ecty8qdctlgscmys2dr5gz729k0l7x4";

        let balances = node
            .get_bank_balances_by_address(ALICE_COSMOS_ADDRESS)
            .await
            .expect("cannot get balances");

        assert_eq!(
            balances,
            BankGetAddressBalancesResponse {
                balances: vec![
                    Balance {
                        denom: "ERT".to_string(),
                        amount: Decimal::from_str_exact("200000000").unwrap(),
                    },
                    Balance {
                        denom: "stake".to_string(),
                        amount: Decimal::from_str_exact("100000000").unwrap(),
                    },
                ]
            }
        )
    }
}
