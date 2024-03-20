#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = super::TokensCommandsContext)]
#[interactive_clap(output_context = SendUncCommandContext)]
pub struct SendUncCommand {
    #[interactive_clap(skip_default_input_arg)]
    /// What is the receiver account ID?
    receiver_account_id: crate::types::account_id::AccountId,
    /// How many unc Tokens do you want to transfer? (example: 10unc or 0.5unc or 10000yoctounc)
    amount_in_unc: crate::types::unc_token::UncToken,
    #[interactive_clap(named_arg)]
    /// Select network
    network_config: crate::network_for_transaction::NetworkForTransactionArgs,
}

#[derive(Debug, Clone)]
pub struct SendUncCommandContext {
    global_context: crate::GlobalContext,
    signer_account_id: unc_primitives::types::AccountId,
    receiver_account_id: unc_primitives::types::AccountId,
    amount_in_unc: crate::types::unc_token::UncToken,
}

impl SendUncCommandContext {
    pub fn from_previous_context(
        previous_context: super::TokensCommandsContext,
        scope: &<SendUncCommand as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        Ok(Self {
            global_context: previous_context.global_context,
            signer_account_id: previous_context.owner_account_id,
            receiver_account_id: scope.receiver_account_id.clone().into(),
            amount_in_unc: scope.amount_in_unc,
        })
    }
}

impl From<SendUncCommandContext> for crate::commands::ActionContext {
    fn from(item: SendUncCommandContext) -> Self {
        let on_after_getting_network_callback: crate::commands::OnAfterGettingNetworkCallback =
            std::sync::Arc::new({
                let signer_account_id = item.signer_account_id.clone();
                let receiver_account_id = item.receiver_account_id.clone();

                move |_network_config| {
                    Ok(crate::commands::PrepopulatedTransaction {
                        signer_id: signer_account_id.clone(),
                        receiver_id: receiver_account_id.clone(),
                        actions: vec![unc_primitives::transaction::Action::Transfer(
                            unc_primitives::transaction::TransferAction {
                                deposit: item.amount_in_unc.as_yoctounc(),
                            },
                        )],
                    })
                }
            });

        Self {
            global_context: item.global_context,
            interacting_with_account_ids: vec![item.signer_account_id, item.receiver_account_id],
            on_after_getting_network_callback,
            on_before_signing_callback: std::sync::Arc::new(
                |_prepolulated_unsinged_transaction, _network_config| Ok(()),
            ),
            on_before_sending_transaction_callback: std::sync::Arc::new(
                |_signed_transaction, _network_config, _message| Ok(()),
            ),
            on_after_sending_transaction_callback: std::sync::Arc::new(
                |_outcome_view, _network_config| Ok(()),
            ),
        }
    }
}

impl SendUncCommand {
    pub fn input_receiver_account_id(
        context: &super::TokensCommandsContext,
    ) -> color_eyre::eyre::Result<Option<crate::types::account_id::AccountId>> {
        crate::common::input_non_signer_account_id_from_used_account_list(
            &context.global_context.config.credentials_home_dir,
            "What is the receiver account ID?",
        )
    }
}
