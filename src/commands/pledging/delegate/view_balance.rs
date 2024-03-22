use color_eyre::eyre::WrapErr;

use crate::common::{CallResultExt, JsonRpcClientExt};

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = super::PledgeDelegationContext)]
#[interactive_clap(output_context = ViewBalanceContext)]
pub struct ViewBalance {
    #[interactive_clap(skip_default_input_arg)]
    /// What is validator account ID?
    validator_account_id: crate::types::account_id::AccountId,
    #[interactive_clap(named_arg)]
    /// Select network
    network_config: crate::network_view_at_block::NetworkViewAtBlockArgs,
}

#[derive(Clone)]
pub struct ViewBalanceContext(crate::network_view_at_block::ArgsForViewContext);

impl ViewBalanceContext {
    pub fn from_previous_context(
        previous_context: super::PledgeDelegationContext,
        scope: &<ViewBalance as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        let account_id = previous_context.account_id.clone();
        let validator_account_id: unc_primitives::types::AccountId =
            scope.validator_account_id.clone().into();
        let interacting_with_account_ids = vec![account_id.clone(), validator_account_id.clone()];

        let on_after_getting_block_reference_callback: crate::network_view_at_block::OnAfterGettingBlockReferenceCallback = std::sync::Arc::new({

            move |network_config, block_reference| {
                let user_pledged_balance: u128 = get_user_pledged_balance(network_config, block_reference, &validator_account_id, &account_id)?;
                let user_unpledged_balance: u128 = get_user_unpledged_balance(network_config, block_reference, &validator_account_id, &account_id)?;
                let user_total_balance: u128 = get_user_total_balance(network_config, block_reference, &validator_account_id, &account_id)?;
                let withdrawal_availability_message = match is_account_unpledged_balance_available_for_withdrawal(network_config, &validator_account_id, &account_id)? {
                    true if user_unpledged_balance > 0  => "(available for withdrawal)",
                    false if user_unpledged_balance > 0 => "(not available for withdrawal in the current epoch)",
                    _ => ""
                };

                eprintln!("Delegated pledge balance with validator <{validator_account_id}> by <{account_id}>:");
                eprintln!("      Pledged balance:     {:>38}", unc_token::UncToken::from_yoctounc(user_pledged_balance).to_string());
                eprintln!("      Unpledged balance:   {:>38} {withdrawal_availability_message}", unc_token::UncToken::from_yoctounc(user_unpledged_balance).to_string());
                eprintln!("      Total balance:      {:>38}", unc_token::UncToken::from_yoctounc(user_total_balance).to_string());

                Ok(())
            }
        });
        Ok(Self(crate::network_view_at_block::ArgsForViewContext {
            config: previous_context.global_context.config,
            interacting_with_account_ids,
            on_after_getting_block_reference_callback,
        }))
    }
}

impl From<ViewBalanceContext> for crate::network_view_at_block::ArgsForViewContext {
    fn from(item: ViewBalanceContext) -> Self {
        item.0
    }
}

impl ViewBalance {
    pub fn input_validator_account_id(
        context: &super::PledgeDelegationContext,
    ) -> color_eyre::eyre::Result<Option<crate::types::account_id::AccountId>> {
        crate::common::input_pledging_pool_validator_account_id(&context.global_context.config)
    }
}

pub fn get_user_pledged_balance(
    network_config: &crate::config::NetworkConfig,
    block_reference: &unc_primitives::types::BlockReference,
    validator_account_id: &unc_primitives::types::AccountId,
    account_id: &unc_primitives::types::AccountId,
) -> color_eyre::eyre::Result<u128> {
    Ok(network_config
        .json_rpc_client()
        .blocking_call_view_function(
            validator_account_id,
            "get_account_pledged_balance",
            serde_json::to_vec(&serde_json::json!({
                "account_id": account_id,
            }))?,
            block_reference.clone(),
        )
        .wrap_err_with(||{
            format!("Failed to fetch query for view method: 'get_account_pledged_balance' (contract <{}> on network <{}>)",
                validator_account_id,
                network_config.network_name
            )
        })?
        .parse_result_from_json::<String>()
        .wrap_err("Failed to parse return value of view function call for String.")?
        .parse::<u128>()?)
}

pub fn get_user_unpledged_balance(
    network_config: &crate::config::NetworkConfig,
    block_reference: &unc_primitives::types::BlockReference,
    validator_account_id: &unc_primitives::types::AccountId,
    account_id: &unc_primitives::types::AccountId,
) -> color_eyre::eyre::Result<u128> {
    Ok(network_config
        .json_rpc_client()
        .blocking_call_view_function(
            validator_account_id,
            "get_account_unpledged_balance",
            serde_json::to_vec(&serde_json::json!({
                "account_id": account_id,
            }))?,
            block_reference.clone(),
        )
        .wrap_err_with(||{
            format!("Failed to fetch query for view method: 'get_account_unpledged_balance' (contract <{}> on network <{}>)",
                validator_account_id,
                network_config.network_name
            )
        })?
        .parse_result_from_json::<String>()
        .wrap_err("Failed to parse return value of view function call for String.")?
        .parse::<u128>()?)
}

pub fn get_user_total_balance(
    network_config: &crate::config::NetworkConfig,
    block_reference: &unc_primitives::types::BlockReference,
    validator_account_id: &unc_primitives::types::AccountId,
    account_id: &unc_primitives::types::AccountId,
) -> color_eyre::eyre::Result<u128> {
    Ok(network_config
        .json_rpc_client()
        .blocking_call_view_function(
            validator_account_id,
            "get_account_total_balance",
            serde_json::to_vec(&serde_json::json!({
                "account_id": account_id,
            }))?,
            block_reference.clone(),
        )
        .wrap_err_with(||{
            format!("Failed to fetch query for view method: 'get_account_total_balance' (contract <{}> on network <{}>)",
                validator_account_id,
                network_config.network_name
            )
        })?
        .parse_result_from_json::<String>()
        .wrap_err("Failed to parse return value of view function call for String.")?
        .parse::<u128>()?)
}

pub fn is_account_unpledged_balance_available_for_withdrawal(
    network_config: &crate::config::NetworkConfig,
    validator_account_id: &unc_primitives::types::AccountId,
    account_id: &unc_primitives::types::AccountId,
) -> color_eyre::eyre::Result<bool> {
    network_config
        .json_rpc_client()
        .blocking_call_view_function(
            validator_account_id,
            "is_account_unpledged_balance_available",
            serde_json::to_vec(&serde_json::json!({
                "account_id": account_id.to_string(),
            }))?,
            unc_primitives::types::BlockReference::Finality(
                unc_primitives::types::Finality::Final,
            ),
        )
        .wrap_err_with(||{
            format!("Failed to fetch query for view method: 'is_account_unpledged_balance_available' (contract <{}> on network <{}>)",
                validator_account_id,
                network_config.network_name
            )
        })?
        .parse_result_from_json::<bool>()
        .wrap_err("Failed to parse return value of view function call for bool value.")
}
