use color_eyre::eyre::WrapErr;
use std::str::FromStr;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Config {
    pub credentials_home_dir: std::path::PathBuf,
    pub network_connection: linked_hash_map::LinkedHashMap<String, NetworkConfig>,
}

impl Default for Config {
    fn default() -> Self {
        let home_dir = dirs::home_dir().expect("Impossible to get your home dir!");
        let mut credentials_home_dir = std::path::PathBuf::from(&home_dir);
        credentials_home_dir.push(".unc-credentials");

        let mut network_connection = linked_hash_map::LinkedHashMap::new();
        network_connection.insert(
            "mainnet".to_string(),
            NetworkConfig {
                network_name: "mainnet".to_string(),
                rpc_url: "https://archival-rpc.mainnet.unc.org".parse().unwrap(),
                wallet_url: "https://app.wallet.com/".parse().unwrap(),
                explorer_transaction_url: "https://explorer.unc.org/transactions/"
                    .parse()
                    .unwrap(),
                rpc_api_key: None,
                linkdrop_account_id: Some("unc".parse().unwrap()),
                unc_social_db_contract_account_id: Some("social.unc".parse().unwrap()),
                faucet_url: None,
                meta_transaction_relayer_url: None,
            },
        );
        network_connection.insert(
            "testnet".to_string(),
            NetworkConfig {
                network_name: "testnet".to_string(),
                rpc_url: "http://127.0.0.1:3030".parse().unwrap(),
                wallet_url: "https://testnet.wallet.com/".parse().unwrap(),
                explorer_transaction_url: "https://explorer.testnet.unc.org/transactions/"
                    .parse()
                    .unwrap(),
                rpc_api_key: None,
                linkdrop_account_id: Some("testnet".parse().unwrap()),
                unc_social_db_contract_account_id: Some("v1.social08.testnet".parse().unwrap()),
                faucet_url: Some("https://helper.unc.com/account".parse().unwrap()),
                meta_transaction_relayer_url: None,
            },
        
        );
        network_connection.insert(
            "custom".to_string(),
            NetworkConfig {
                network_name: "betanet".to_string(),
                rpc_url: "http://43.198.88.81:3031".parse().unwrap(),
                wallet_url: "https://testnet.wallet.com/".parse().unwrap(),
                explorer_transaction_url: "https://explorer.testnet.unc.org/transactions/"
                    .parse()
                    .unwrap(),
                rpc_api_key: None,
                linkdrop_account_id: Some("testnet".parse().unwrap()),
                unc_social_db_contract_account_id: None,
                faucet_url: Some("https://helper.unc.com/account".parse().unwrap()),
                meta_transaction_relayer_url: None,
            },
        
        );
        Self {
            credentials_home_dir,
            network_connection,
        }
    }
}

impl Config {
    pub fn network_names(&self) -> Vec<String> {
        self.network_connection
            .iter()
            .map(|(_, network_config)| network_config.network_name.clone())
            .collect()
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NetworkConfig {
    pub network_name: String,
    pub rpc_url: url::Url,
    pub rpc_api_key: Option<crate::types::api_key::ApiKey>,
    pub wallet_url: url::Url,
    pub explorer_transaction_url: url::Url,
    // https://github.com/unc/unc-cli-rs/issues/116
    pub linkdrop_account_id: Option<unc_primitives::types::AccountId>,
    // https://docs.unc.org/social/contract
    pub unc_social_db_contract_account_id: Option<unc_primitives::types::AccountId>,
    pub faucet_url: Option<url::Url>,
    pub meta_transaction_relayer_url: Option<url::Url>,
}

impl NetworkConfig {
    pub fn json_rpc_client(&self) -> unc_jsonrpc_client::JsonRpcClient {
        let mut json_rpc_client =
            unc_jsonrpc_client::JsonRpcClient::connect(self.rpc_url.as_ref());
        if let Some(rpc_api_key) = &self.rpc_api_key {
            json_rpc_client =
                json_rpc_client.header(unc_jsonrpc_client::auth::ApiKey::from(rpc_api_key.clone()))
        };
        json_rpc_client
    }

    pub fn get_unc_social_account_id_from_network(
        &self,
    ) -> color_eyre::eyre::Result<unc_primitives::types::AccountId> {
        if let Some(account_id) = self.unc_social_db_contract_account_id.clone() {
            return Ok(account_id);
        }
        match self.network_name.as_str() {
            "mainnet" => unc_primitives::types::AccountId::from_str("social.unc")
                .wrap_err("Internal error"),
            "testnet" => unc_primitives::types::AccountId::from_str("v1.social08.testnet")
                .wrap_err("Internal error"),
            _ => color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!(
                "This network does not provide the \"unc-social\" contract"
            )),
        }
    }
}
