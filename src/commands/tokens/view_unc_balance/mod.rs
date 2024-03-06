#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = super::TokensCommandsContext)]
#[interactive_clap(output_context = ViewuncBalanceContext)]
pub struct ViewuncBalance {
    #[interactive_clap(named_arg)]
    /// Select network
    network_config: crate::network_view_at_block::NetworkViewAtBlockArgs,
}

#[derive(Clone)]
pub struct ViewuncBalanceContext(crate::network_view_at_block::ArgsForViewContext);

impl ViewuncBalanceContext {
    pub fn from_previous_context(
        previous_context: super::TokensCommandsContext,
        _scope: &<ViewuncBalance as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        let on_after_getting_block_reference_callback: crate::network_view_at_block::OnAfterGettingBlockReferenceCallback = std::sync::Arc::new({
            let owner_account_id = previous_context.owner_account_id.clone();

            move |network_config, block_reference| {
                let account_transfer_allowance = crate::common::get_account_transfer_allowance(
                    network_config.clone(),
                    owner_account_id.clone(),
                    block_reference.clone(),
                )?;
                eprintln!("{account_transfer_allowance}");
                Ok(())
            }
        });

        Ok(Self(crate::network_view_at_block::ArgsForViewContext {
            config: previous_context.global_context.config,
            interacting_with_account_ids: vec![previous_context.owner_account_id],
            on_after_getting_block_reference_callback,
        }))
    }
}

impl From<ViewuncBalanceContext> for crate::network_view_at_block::ArgsForViewContext {
    fn from(item: ViewuncBalanceContext) -> Self {
        item.0
    }
}
