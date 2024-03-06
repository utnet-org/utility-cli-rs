#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = super::ContractContext)]
#[interactive_clap(output_context = AccountContext)]
pub struct Account {
    #[interactive_clap(skip_default_input_arg)]
    /// What is your account ID?
    account_id: crate::types::account_id::AccountId,
    #[interactive_clap(named_arg)]
    /// Select network
    network_config: crate::network_view_at_block::NetworkViewAtBlockArgs,
}

#[derive(Clone)]
pub struct AccountContext(crate::network_view_at_block::ArgsForViewContext);

impl AccountContext {
    pub fn from_previous_context(
        previous_context: super::ContractContext,
        scope: &<Account as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        let on_after_getting_block_reference_callback: crate::network_view_at_block::OnAfterGettingBlockReferenceCallback =
            std::sync::Arc::new({
                let _account_id = scope.account_id.clone();

                move |network_config, _block_reference| {
                    let _contract_account_id = (previous_context.get_contract_account_id)(network_config)?;

                    Ok(())
                }
            });

        Ok(Self(crate::network_view_at_block::ArgsForViewContext {
            config: previous_context.global_context.config,
            interacting_with_account_ids: vec![scope.account_id.clone().into()],
            on_after_getting_block_reference_callback,
        }))
    }
}

impl From<AccountContext> for crate::network_view_at_block::ArgsForViewContext {
    fn from(item: AccountContext) -> Self {
        item.0
    }
}

impl Account {
    pub fn input_account_id(
        context: &super::ContractContext,
    ) -> color_eyre::eyre::Result<Option<crate::types::account_id::AccountId>> {
        crate::common::input_non_signer_account_id_from_used_account_list(
            &context.global_context.config.credentials_home_dir,
            "What is your account ID?",
        )
    }
}
