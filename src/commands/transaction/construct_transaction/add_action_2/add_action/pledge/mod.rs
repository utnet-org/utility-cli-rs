#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = super::super::super::ConstructTransactionContext)]
#[interactive_clap(output_context = PledgeActionContext)]
pub struct PledgeAction {
    pledge_amount: crate::types::unc_token::UncToken,
    public_key: crate::types::public_key::PublicKey,
    #[interactive_clap(subcommand)]
    next_action: super::super::super::add_action_3::NextAction,
}

#[derive(Debug, Clone)]
pub struct PledgeActionContext(super::super::super::ConstructTransactionContext);

impl PledgeActionContext {
    pub fn from_previous_context(
        previous_context: super::super::super::ConstructTransactionContext,
        scope: &<PledgeAction as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        let action = unc_primitives::transaction::Action::Pledge(Box::new(
            unc_primitives::transaction::PledgeAction {
                pledge: scope.pledge_amount.as_attounc(),
                public_key: scope.public_key.clone().into(),
            },
        ));
        let mut actions = previous_context.actions;
        actions.push(action);
        Ok(Self(super::super::super::ConstructTransactionContext {
            global_context: previous_context.global_context,
            signer_account_id: previous_context.signer_account_id,
            receiver_account_id: previous_context.receiver_account_id,
            actions,
        }))
    }
}

impl From<PledgeActionContext> for super::super::super::ConstructTransactionContext {
    fn from(item: PledgeActionContext) -> Self {
        item.0
    }
}
