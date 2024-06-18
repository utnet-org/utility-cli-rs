use serde::Deserialize;

pub mod add_key;
pub mod network;

#[derive(Clone)]
pub struct SponsorServiceContext {
    pub config: crate::config::Config,
    pub new_account_id: crate::types::account_id::AccountId,
    pub public_key: unc_crypto::PublicKey,
    pub on_after_getting_network_callback: self::network::OnAfterGettingNetworkCallback,
    pub on_before_creating_account_callback: self::network::OnBeforeCreatingAccountCallback,
}

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = crate::GlobalContext)]
#[interactive_clap(output_context = NewAccountContext)]
pub struct NewAccount {
    #[interactive_clap(skip_default_input_arg)]
    /// What is the new account ID?
    new_account_id: crate::types::account_id::AccountId,
    #[interactive_clap(subcommand)]
    access_key_mode: add_key::AccessKeyMode,
}

#[derive(Clone)]
pub struct NewAccountContext {
    pub config: crate::config::Config,
    pub new_account_id: crate::types::account_id::AccountId,
    pub on_before_creating_account_callback: self::network::OnBeforeCreatingAccountCallback,
}

impl NewAccountContext {
    pub fn from_previous_context(
        previous_context: crate::GlobalContext,
        scope: &<NewAccount as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        let credentials_home_dir = previous_context.config.credentials_home_dir.clone();
        let on_before_creating_account_callback: self::network::OnBeforeCreatingAccountCallback =
            std::sync::Arc::new({
                move |network_config, new_account_id, public_key| {
                    before_creating_account(
                        network_config,
                        new_account_id,
                        public_key,
                        &credentials_home_dir,
                    )
                }
            });

        Ok(Self {
            config: previous_context.config,
            new_account_id: scope.new_account_id.clone(),
            on_before_creating_account_callback,
        })
    }
}

impl NewAccount {
    fn input_new_account_id(
        context: &crate::GlobalContext,
    ) -> color_eyre::eyre::Result<Option<crate::types::account_id::AccountId>> {
        super::fund_myself_create_account::NewAccount::input_new_account_id(context)
    }
}

pub fn before_creating_account(
    network_config: &crate::config::NetworkConfig,
    new_account_id: &crate::types::account_id::AccountId,
    _public_key: &unc_crypto::PublicKey,
    credentials_home_dir: &std::path::Path,
) -> crate::CliResult {
    let faucet_service_url = match &network_config.faucet_url {
        Some(url) => url,
        None => return Err(color_eyre::Report::msg(format!(
            "The <{}> network does not have a faucet (helper service) that can sponsor the creation of an account.",
            &network_config.network_name
        )))
    };
    let mut data = std::collections::HashMap::new();
    data.insert("contractId", "4e0375672ec30f2efe3a6c5a14ff81d37f1271c439501eac2fb445df262b2c32".to_string());
    data.insert("receiverId", new_account_id.to_string());
    data.insert("amount", "10".to_string());

    let client = reqwest::blocking::Client::new();
    match client.post(faucet_service_url.clone()).json(&data).send() {
        Ok(response) => {
            if response.status() >= reqwest::StatusCode::BAD_REQUEST {
                return Err(color_eyre::Report::msg(format!(
                    "The faucet (helper service) server failed with status code <{}>",
                    response.status()
                )));
            }

            let account_creation_transaction =
                response.json::<Transaction>()?;

            crate::common::update_used_account_list_as_signer(
                credentials_home_dir,
                new_account_id.as_ref(),
            );
            eprintln!("New account <{}> created successfully.", &new_account_id);
            eprintln!("Processing transaction...\nPlease wait for 6 blocks to confirm, use command: unc transaction view-status <tx_hash>");
            eprintln!("Transaction ID: {id}\nTo see the transaction in the transaction explorer, please open this url in your browser:\n{path}{id}\n",
            id=account_creation_transaction.txh,
            path=network_config.explorer_transaction_url);
            Ok(())
        }
        Err(err) => Err(color_eyre::Report::msg(err.to_string())),
    }
}

#[derive(Debug, Deserialize)]
struct Transaction {
    txh: String,
}
