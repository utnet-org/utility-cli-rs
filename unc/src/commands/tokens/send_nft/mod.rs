use inquire::CustomType;
use serde_json::json;

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = super::TokensCommandsContext)]
#[interactive_clap(output_context = SendNftCommandContext)]
pub struct SendNftCommand {
    #[interactive_clap(skip_default_input_arg)]
    /// What is the nft-contract account ID?
    nft_contract_account_id: crate::types::account_id::AccountId,
    #[interactive_clap(skip_default_input_arg)]
    /// What is the receiver account ID?
    receiver_account_id: crate::types::account_id::AccountId,
    /// Enter an token_id for NFT:
    token_id: String,
    #[interactive_clap(long = "prepaid-gas")]
    #[interactive_clap(skip_default_input_arg)]
    /// Enter gas for function call:
    gas: crate::common::UncGas,
    #[interactive_clap(long = "attached-deposit")]
    #[interactive_clap(skip_default_input_arg)]
    /// Enter deposit for a function call:
    deposit: crate::types::unc_token::UncToken,
    #[interactive_clap(named_arg)]
    /// Select network
    network_config: crate::network_for_transaction::NetworkForTransactionArgs,
}

#[derive(Debug, Clone)]
pub struct SendNftCommandContext {
    global_context: crate::GlobalContext,
    signer_account_id: unc_primitives::types::AccountId,
    nft_contract_account_id: unc_primitives::types::AccountId,
    receiver_account_id: unc_primitives::types::AccountId,
    token_id: String,
    gas: crate::common::UncGas,
    deposit: crate::types::unc_token::UncToken,
}

impl SendNftCommandContext {
    pub fn from_previous_context(
        previous_context: super::TokensCommandsContext,
        scope: &<SendNftCommand as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        Ok(Self {
            global_context: previous_context.global_context,
            signer_account_id: previous_context.owner_account_id,
            nft_contract_account_id: scope.nft_contract_account_id.clone().into(),
            receiver_account_id: scope.receiver_account_id.clone().into(),
            token_id: scope.token_id.clone(),
            gas: scope.gas,
            deposit: scope.deposit,
        })
    }
}

impl From<SendNftCommandContext> for crate::commands::ActionContext {
    fn from(item: SendNftCommandContext) -> Self {
        let on_after_getting_network_callback: crate::commands::OnAfterGettingNetworkCallback =
            std::sync::Arc::new({
                let signer_account_id = item.signer_account_id.clone();
                let nft_contract_account_id = item.nft_contract_account_id.clone();
                let receiver_account_id = item.receiver_account_id.clone();
                let token_id = item.token_id.clone();

                move |_network_config| {
                    Ok(crate::commands::PrepopulatedTransaction {
                        signer_id: signer_account_id.clone(),
                        receiver_id: nft_contract_account_id.clone(),
                        actions: vec![unc_primitives::transaction::Action::FunctionCall(Box::new(
                            unc_primitives::transaction::FunctionCallAction {
                                method_name: "nft_transfer".to_string(),
                                args: serde_json::to_vec(&json!({
                                    "receiver_id": receiver_account_id.to_string(),
                                    "token_id": token_id
                                }))?,
                                gas: item.gas.as_gas(),
                                deposit: item.deposit.as_attounc(),
                            },
                        ))],
                    })
                }
            });

        let on_after_sending_transaction_callback: crate::transaction_signature_options::OnAfterSendingTransactionCallback = std::sync::Arc::new({
            let signer_account_id = item.signer_account_id.clone();
            let nft_contract_account_id = item.nft_contract_account_id.clone();
            let receiver_account_id = item.receiver_account_id.clone();
            let token_id = item.token_id.clone();

            move |outcome_view, _network_config| {
                if let unc_primitives::views::FinalExecutionStatus::SuccessValue(_) = outcome_view.status {
                    eprintln!(
                        "<{signer_account_id}> has successfully transferred NFT token_id=\"{token_id}\" to <{receiver_account_id}> on contract <{nft_contract_account_id}>.",
                    );
                }
                Ok(())
            }
        });

        Self {
            global_context: item.global_context,
            interacting_with_account_ids: vec![
                item.nft_contract_account_id.clone(),
                item.signer_account_id.clone(),
                item.receiver_account_id.clone(),
            ],
            on_after_getting_network_callback,
            on_before_signing_callback: std::sync::Arc::new(
                |_prepolulated_unsinged_transaction, _network_config| Ok(()),
            ),
            on_before_sending_transaction_callback: std::sync::Arc::new(
                |_signed_transaction, _network_config, _message| Ok(()),
            ),
            on_after_sending_transaction_callback,
        }
    }
}

impl SendNftCommand {
    pub fn input_nft_contract_account_id(
        context: &super::TokensCommandsContext,
    ) -> color_eyre::eyre::Result<Option<crate::types::account_id::AccountId>> {
        crate::common::input_non_signer_account_id_from_used_account_list(
            &context.global_context.config.credentials_home_dir,
            "What is the nft-contract account ID?",
        )
    }

    pub fn input_receiver_account_id(
        context: &super::TokensCommandsContext,
    ) -> color_eyre::eyre::Result<Option<crate::types::account_id::AccountId>> {
        crate::common::input_non_signer_account_id_from_used_account_list(
            &context.global_context.config.credentials_home_dir,
            "What is the receiver account ID?",
        )
    }

    fn input_gas(
        _context: &super::TokensCommandsContext,
    ) -> color_eyre::eyre::Result<Option<crate::common::UncGas>> {
        eprintln!();
        Ok(Some(
            CustomType::new("Enter gas for function call:")
                .with_starting_input("100 TeraGas")
                .with_validator(move |gas: &crate::common::UncGas| {
                    if gas > &unc_gas::UncGas::from_tgas(300) {
                        Ok(inquire::validator::Validation::Invalid(
                            inquire::validator::ErrorMessage::Custom(
                                "You need to enter a value of no more than 300 TeraGas".to_string(),
                            ),
                        ))
                    } else {
                        Ok(inquire::validator::Validation::Valid)
                    }
                })
                .prompt()?,
        ))
    }

    fn input_deposit(
        _context: &super::TokensCommandsContext,
    ) -> color_eyre::eyre::Result<Option<crate::types::unc_token::UncToken>> {
        eprintln!();
        Ok(Some(
            CustomType::new(
                "Enter deposit for a function call (example: 10 UNC or 0.5 unc or 10000 attounc):",
            )
            .with_starting_input("1 attounc")
            .prompt()?,
        ))
    }
}
