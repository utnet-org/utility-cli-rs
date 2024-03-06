#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = super::super::super::ConstructTransactionContext)]
#[interactive_clap(output_context = StakeActionContext)]
pub struct StakeAction {
    stake_amount: crate::types::unc_token::UncToken,
    public_key: crate::types::public_key::PublicKey,
    #[interactive_clap(subcommand)]
    next_action: super::super::super::add_action_last::NextAction,
}

#[derive(Debug, Clone)]
pub struct StakeActionContext(super::super::super::ConstructTransactionContext);

impl StakeActionContext {
    pub fn from_previous_context(
        previous_context: super::super::super::ConstructTransactionContext,
        scope: &<StakeAction as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        let action = unc_primitives::transaction::Action::Stake(Box::new(
            unc_primitives::transaction::StakeAction {
                stake: scope.stake_amount.as_yoctounc(),
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

impl From<StakeActionContext> for super::super::super::ConstructTransactionContext {
    fn from(item: StakeActionContext) -> Self {
        item.0
    }
}
