use inquire::CustomType;

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = crate::GlobalContext)]
#[interactive_clap(output_context = PledgeProposalContext)]
pub struct PledgeProposal {
    #[interactive_clap(skip_default_input_arg)]
    /// What is the validator account ID?
    validator: crate::types::account_id::AccountId,
    /// Validator key which will be used to sign transactions on behalf of signer_id:
    public_key: crate::types::public_key::PublicKey,
    #[interactive_clap(skip_default_input_arg)]
    /// Enter the amount to pledge:
    pledge: crate::types::unc_token::UncToken,
    #[interactive_clap(named_arg)]
    /// Select network
    network_config: crate::network_for_transaction::NetworkForTransactionArgs,
}

#[derive(Debug, Clone)]
pub struct PledgeProposalContext {
    global_context: crate::GlobalContext,
    validator: unc_primitives::types::AccountId,
    public_key: unc_crypto::PublicKey,
    pledge: crate::types::unc_token::UncToken,
}

impl PledgeProposalContext {
    pub fn from_previous_context(
        previous_context: crate::GlobalContext,
        scope: &<PledgeProposal as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        Ok(Self {
            global_context: previous_context,
            validator: scope.validator.clone().into(),
            public_key: scope.public_key.clone().into(),
            pledge: scope.pledge,
        })
    }
}

impl From<PledgeProposalContext> for crate::commands::ActionContext {
    fn from(item: PledgeProposalContext) -> Self {
        let on_after_getting_network_callback: crate::commands::OnAfterGettingNetworkCallback =
            {
                let validator = item.validator.clone();
                std::sync::Arc::new(move |_network_config| {
                Ok(crate::commands::PrepopulatedTransaction {
                    signer_id: validator.clone(),
                    receiver_id: validator.clone(),
                    actions: vec![unc_primitives::transaction::Action::Pledge(
                        Box::new(unc_primitives::transaction::PledgeAction {
                            pledge: item.pledge.as_attounc(),
                            public_key: item.public_key.clone(),
                        }),
                    )],
                })
            })};
        Self {
            global_context: item.global_context,
            interacting_with_account_ids: vec![item.validator],
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

impl PledgeProposal {
    pub fn input_validator(
        context: &crate::GlobalContext,
    ) -> color_eyre::eyre::Result<Option<crate::types::account_id::AccountId>> {
        crate::common::input_signer_account_id_from_used_account_list(
            &context.config.credentials_home_dir,
            "What is the validator account ID?",
        )
    }

    fn input_pledge(
        _context: &crate::GlobalContext,
    ) -> color_eyre::eyre::Result<Option<crate::types::unc_token::UncToken>> {
        let input_amount =
            CustomType::new("Enter the amount to pledge: (example: 10000unc)").prompt()?;
        Ok(Some(input_amount))
    }
}
