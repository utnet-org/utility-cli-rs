use color_eyre::eyre::Context;

use crate::common::{JsonRpcClientExt, RpcQueryResponseExt};

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = crate::GlobalContext)]
#[interactive_clap(output_context = ViewPledgeContext)]
pub struct ViewPledge {
    #[interactive_clap(skip_default_input_arg)]
    /// Enter validator account ID to view pledge:
    validator_account_id: crate::types::account_id::AccountId,
    #[interactive_clap(named_arg)]
    /// Select network
    network_config: crate::network_view_at_block::NetworkViewAtBlockArgs,
}

#[derive(Clone)]
pub struct ViewPledgeContext(crate::network_view_at_block::ArgsForViewContext);

impl ViewPledgeContext {
    pub fn from_previous_context(
        previous_context: crate::GlobalContext,
        scope: &<ViewPledge as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        let on_after_getting_block_reference_callback: crate::network_view_at_block::OnAfterGettingBlockReferenceCallback = std::sync::Arc::new({
            let validator_account_id: unc_primitives::types::AccountId = scope.validator_account_id.clone().into();

            move |network_config, block_reference| {
                let json_rpc_client = network_config.json_rpc_client();

                let rpc_query_response = json_rpc_client
                    .blocking_call_view_account(&validator_account_id.clone(), block_reference.clone())
                    .wrap_err_with(|| {
                        format!(
                            "Failed to fetch query ViewAccount for <{}>",
                            &validator_account_id
                        )
                    })?;
                let account_view = rpc_query_response.account_view()?;
                eprintln!("Validator {validator_account_id} pledged amount {}",
                    crate::types::unc_token::UncToken::from_attounc(account_view.pledging)
                );

                Ok(())
            }
        });
        Ok(Self(
            crate::network_view_at_block::ArgsForViewContext {
                config: previous_context.config,
                interacting_with_account_ids: vec![scope.validator_account_id.clone().into()],
                on_after_getting_block_reference_callback,
            },
        ))
    }
}

impl From<ViewPledgeContext> for crate::network_view_at_block::ArgsForViewContext {
    fn from(item: ViewPledgeContext) -> Self {
        item.0
    }
}

impl ViewPledge {
    pub fn input_validator_account_id(
        context: &crate::GlobalContext,
    ) -> color_eyre::eyre::Result<Option<crate::types::account_id::AccountId>> {
        crate::common::input_non_signer_account_id_from_used_account_list(
            &context.config.credentials_home_dir,
            "What Account ID do you need to view?",
        )
    }
}
