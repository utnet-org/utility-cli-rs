use serde_json::json;
use unc_primitives::account::id::AccountType;

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = super::AccountPropertiesContext)]
#[interactive_clap(output_context = SignerAccountIdContext)]
pub struct SignerAccountId {
    #[interactive_clap(skip_default_input_arg)]
    /// What is the signer account ID?
    signer_account_id: crate::types::account_id::AccountId,
    #[interactive_clap(named_arg)]
    /// Select network
    network_config: crate::network_for_transaction::NetworkForTransactionArgs,
}

#[derive(Clone)]
pub struct SignerAccountIdContext {
    global_context: crate::GlobalContext,
    account_properties: super::AccountProperties,
    signer_account_id: unc_primitives::types::AccountId,
    on_before_sending_transaction_callback:
        crate::transaction_signature_options::OnBeforeSendingTransactionCallback,
}

impl SignerAccountIdContext {
    pub fn from_previous_context(
        previous_context: super::AccountPropertiesContext,
        scope: &<SignerAccountId as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        Ok(Self {
            global_context: previous_context.global_context,
            account_properties: previous_context.account_properties,
            signer_account_id: scope.signer_account_id.clone().into(),
            on_before_sending_transaction_callback: previous_context
                .on_before_sending_transaction_callback,
        })
    }
}

impl From<SignerAccountIdContext> for crate::commands::ActionContext {
    fn from(item: SignerAccountIdContext) -> Self {
        let global_context = item.global_context.clone();

        let on_after_getting_network_callback: crate::commands::OnAfterGettingNetworkCallback =
            std::sync::Arc::new({
                let new_account_id = item.account_properties.new_account_id.clone();
                let signer_id = item.signer_account_id.clone();

                move |network_config| {
                    if !item.global_context.offline {
                        validate_new_account_id(network_config, &new_account_id)?;
                    }
                    let (actions, receiver_id) = if AccountType::UtilityAccount == new_account_id.get_account_type() {
                        (vec![
                                unc_primitives::transaction::Action::CreateAccount(
                                    unc_primitives::transaction::CreateAccountAction {},
                                ),
                                unc_primitives::transaction::Action::Transfer(
                                    unc_primitives::transaction::TransferAction {
                                        deposit: item.account_properties.initial_balance.as_attounc(),
                                    },
                                ),
                                unc_primitives::transaction::Action::AddKey(
                                    Box::new(unc_primitives::transaction::AddKeyAction {
                                        public_key: item.account_properties.public_key.clone(),
                                        access_key: unc_primitives::account::AccessKey {
                                            nonce: 0,
                                            permission:
                                                unc_primitives::account::AccessKeyPermission::FullAccess,
                                        },
                                    }),
                                ),
                            ],
                        new_account_id.clone())
                    } else {
                        let args = serde_json::to_vec(&json!({
                            "new_account_id": new_account_id.clone().to_string(),
                            "new_public_key": item.account_properties.public_key.to_string()
                        }))?;

                        if let Some(linkdrop_account_id) = &network_config.linkdrop_account_id {
                            if new_account_id.as_str().chars().count() > super::MIN_ALLOWED_TOP_LEVEL_ACCOUNT_LENGTH
                            {
                                (
                                    vec![unc_primitives::transaction::Action::FunctionCall(
                                        Box::new(unc_primitives::transaction::FunctionCallAction {
                                            method_name: "create_account".to_string(),
                                            args,
                                            gas: crate::common::UncGas::from_tgas(30).as_gas(),
                                            deposit: item
                                                .account_properties
                                                .initial_balance
                                                .as_attounc(),
                                        }),
                                    )],
                                    linkdrop_account_id.clone(),
                                )
                            } else {
                                return color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!(
                                    "\nSigner account <{}> does not have permission to create account <{}>.",
                                    signer_id,
                                    new_account_id
                                ));
                            }
                        } else {
                            return color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!(
                                "\nAccount <{}> cannot be created on network <{}> because a <linkdrop_account_id> is not specified in the configuration file.\nYou can learn about working with the configuration file: https://github.com/unc/unc-cli-rs/blob/master/docs/README.en.md#config. \nExample <linkdrop_account_id> in configuration file: https://github.com/unc/unc-cli-rs/blob/master/docs/media/linkdrop account_id.png",
                                new_account_id,
                                network_config.network_name
                            ));
                        }
                    };

                    Ok(crate::commands::PrepopulatedTransaction {
                        signer_id: signer_id.clone(),
                        receiver_id,
                        actions,
                    })
                }
            });

        let on_after_sending_transaction_callback: crate::transaction_signature_options::OnAfterSendingTransactionCallback =
            std::sync::Arc::new({
                let credentials_home_dir = global_context.config.credentials_home_dir.clone();

                move |outcome_view, _network_config| {
                    crate::common::update_used_account_list_as_signer(
                        &credentials_home_dir,
                        &outcome_view.transaction.receiver_id,
                    );
                    Ok(())
                }
            });

        Self {
            global_context,
            interacting_with_account_ids: vec![
                item.signer_account_id,
                item.account_properties.new_account_id,
            ],
            on_after_getting_network_callback,
            on_before_signing_callback: std::sync::Arc::new(
                |_prepolulated_unsinged_transaction, _network_config| Ok(()),
            ),
            on_before_sending_transaction_callback: item.on_before_sending_transaction_callback,
            on_after_sending_transaction_callback,
        }
    }
}

impl SignerAccountId {
    fn input_signer_account_id(
        context: &super::AccountPropertiesContext,
    ) -> color_eyre::eyre::Result<Option<crate::types::account_id::AccountId>> {
        crate::common::input_signer_account_id_from_used_account_list(
            &context.global_context.config.credentials_home_dir,
            "What is the signer account ID?",
        )
    }
}

fn validate_new_account_id(
    network_config: &crate::config::NetworkConfig,
    account_id: &unc_primitives::types::AccountId,
) -> crate::CliResult {
    for _ in 0..3 {
        let account_state = crate::common::get_account_state(
            network_config.clone(),
            account_id.clone(),
            unc_primitives::types::BlockReference::latest(),
        );
        if let Err(unc_jsonrpc_client::errors::JsonRpcError::TransportError(
            unc_jsonrpc_client::errors::RpcTransportError::SendError(_),
        )) = account_state
        {
            eprintln!("Transport error.\nPlease wait. The next try to send this query is happening right now ...");
            std::thread::sleep(std::time::Duration::from_millis(100))
        } else {
            match account_state {
                Ok(_) => {
                    return color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!(
                "\nAccount <{}> already exists in network <{}>. Therefore, it is not possible to create an account with this name.",
                account_id,
                network_config.network_name
            ));
                }
                Err(unc_jsonrpc_client::errors::JsonRpcError::ServerError(
                    unc_jsonrpc_client::errors::JsonRpcServerError::HandlerError(
                        unc_jsonrpc_primitives::types::query::RpcQueryError::UnknownAccount {
                            ..
                        },
                    ),
                )) => {
                    return Ok(());
                }
                Err(err) => {
                    return color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!(err.to_string()))
                }
            }
        }
    }
    eprintln!("\nTransport error.\nIt is currently possible to continue creating an account offline.\nYou can sign and send the created transaction later.");
    Ok(())
}
