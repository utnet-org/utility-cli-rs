use strum::{EnumDiscriminants, EnumIter, EnumMessage};

#[derive(Debug, Clone, EnumDiscriminants, interactive_clap_derive::InteractiveClap)]
#[interactive_clap(context = super::PemFileContext)]
#[strum_discriminants(derive(EnumMessage, EnumIter))]
/// Select the need for initialization:
pub enum InitializeMode {
    /// Don't add an initialize
    #[strum_discriminants(strum(message = "without-init-call  - Don't add an initialize"))]
    WithoutInitCall(NoInitialize),
}

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = super::PemFileContext)]
#[interactive_clap(output_context = NoInitializeContext)]
pub struct NoInitialize {
    #[interactive_clap(named_arg)]
    /// Select network
    network_config: crate::network_for_transaction::NetworkForTransactionArgs,
}

#[derive(Debug, Clone)]
pub struct NoInitializeContext(super::PemFileContext);

impl NoInitializeContext {
    pub fn from_previous_context(
        previous_context: super::PemFileContext,
        _scope: &<NoInitialize as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        Ok(Self(super::PemFileContext {
            global_context: previous_context.global_context,
            receiver_account_id: previous_context.receiver_account_id,
            signer_account_id: previous_context.signer_account_id,
            public_key: previous_context.public_key,
            private_key: previous_context.private_key,
        }))
    }
}

impl From<NoInitializeContext> for crate::commands::ActionContext {
    fn from(item: NoInitializeContext) -> Self {
        let on_after_getting_network_callback: crate::commands::OnAfterGettingNetworkCallback =
            std::sync::Arc::new({
                let signer_account_id = item.0.signer_account_id.clone();
                let receiver_account_id = item.0.receiver_account_id.clone();

                move |_network_config| {
                    Ok(crate::commands::PrepopulatedTransaction {
                        signer_id: signer_account_id.clone(),
                        receiver_id: receiver_account_id.clone(),
                        actions: vec![near_primitives::transaction::Action::CreateRsa2048Challenge(
                            Box::new(near_primitives::transaction::CreateRsa2048ChallengeAction {
                                public_key: item.0.public_key.clone(), 
                                args: vec![],
                            }),
                        )],
                    })
                }
            });

        Self {
            global_context: item.0.global_context,
            interacting_with_account_ids: vec![
                item.0.signer_account_id,
                item.0.receiver_account_id,
            ],
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
