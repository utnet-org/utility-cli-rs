#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = super::PledgeDelegationContext)]
#[interactive_clap(output_context = PledgeContext)]
pub struct Pledge {
    /// Enter the amount to pledge from the inner account of the predecessor (example: 10unc or 0.5unc or 10000attounc):
    amount: crate::types::unc_token::UncToken,
    #[interactive_clap(skip_default_input_arg)]
    /// What is validator account ID?
    validator_account_id: crate::types::account_id::AccountId,
    #[interactive_clap(named_arg)]
    /// Select network
    network_config: crate::network_for_transaction::NetworkForTransactionArgs,
}

#[derive(Clone)]
pub struct PledgeContext(crate::commands::ActionContext);

impl PledgeContext {
    pub fn from_previous_context(
        previous_context: super::PledgeDelegationContext,
        scope: &<Pledge as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        let on_after_getting_network_callback: crate::commands::OnAfterGettingNetworkCallback =
            std::sync::Arc::new({
                let signer_id = previous_context.account_id.clone();
                let validator_account_id: unc_primitives::types::AccountId =
                    scope.validator_account_id.clone().into();
                let amount = scope.amount;

                move |_network_config| {
                    Ok(crate::commands::PrepopulatedTransaction {
                        signer_id: signer_id.clone(),
                        receiver_id: validator_account_id.clone(),
                        actions: vec![unc_primitives::transaction::Action::FunctionCall(Box::new(
                            unc_primitives::transaction::FunctionCallAction {
                                method_name: "pledge".to_string(),
                                args: serde_json::to_vec(&serde_json::json!({
                                    "amount": amount,
                                }))?,
                                gas: crate::common::UncGas::from_tgas(50).as_gas(),
                                deposit: 0,
                            },
                        ))],
                    })
                }
            });

        let on_after_sending_transaction_callback: crate::transaction_signature_options::OnAfterSendingTransactionCallback = std::sync::Arc::new({
            let signer_id = previous_context.account_id.clone();
            let validator_id = scope.validator_account_id.clone();
            let amount = scope.amount;

            move |outcome_view, _network_config| {
                if let unc_primitives::views::FinalExecutionStatus::SuccessValue(_) = outcome_view.status {
                    eprintln!("<{signer_id}> has successfully delegated  {amount} to pledge with <{validator_id}>.")
                }
                Ok(())
            }
        });

        Ok(Self(crate::commands::ActionContext {
            global_context: previous_context.global_context,
            interacting_with_account_ids: vec![
                previous_context.account_id,
                scope.validator_account_id.clone().into(),
            ],
            on_after_getting_network_callback,
            on_before_signing_callback: std::sync::Arc::new(
                |_prepolulated_unsinged_transaction, _network_config| Ok(()),
            ),
            on_before_sending_transaction_callback: std::sync::Arc::new(
                |_signed_transaction, _network_config, _message| Ok(()),
            ),
            on_after_sending_transaction_callback,
        }))
    }
}

impl From<PledgeContext> for crate::commands::ActionContext {
    fn from(item: PledgeContext) -> Self {
        item.0
    }
}

impl Pledge {
    pub fn input_validator_account_id(
        context: &super::PledgeDelegationContext,
    ) -> color_eyre::eyre::Result<Option<crate::types::account_id::AccountId>> {
        crate::common::input_pledging_pool_validator_account_id(&context.global_context.config)
    }
}
