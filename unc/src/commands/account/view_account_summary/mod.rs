use color_eyre::eyre::Context;
use futures::{StreamExt, TryStreamExt};

use crate::common::{CallResultExt, JsonRpcClientExt, RpcQueryResponseExt};

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = crate::GlobalContext)]
#[interactive_clap(output_context = ViewAccountSummaryContext)]
pub struct ViewAccountSummary {
    #[interactive_clap(skip_default_input_arg)]
    /// What Account ID do you need to view?
    account_id: crate::types::account_id::AccountId,
    #[interactive_clap(named_arg)]
    /// Select network
    network_config: crate::network_view_at_block::NetworkViewAtBlockArgs,
}

#[derive(Clone)]
pub struct ViewAccountSummaryContext(crate::network_view_at_block::ArgsForViewContext);

impl ViewAccountSummaryContext {
    pub fn from_previous_context(
        previous_context: crate::GlobalContext,
        scope: &<ViewAccountSummary as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        let on_after_getting_block_reference_callback: crate::network_view_at_block::OnAfterGettingBlockReferenceCallback = std::sync::Arc::new({
            let account_id: unc_primitives::types::AccountId = scope.account_id.clone().into();

            move |network_config, block_reference| {
                let json_rpc_client = network_config.json_rpc_client();

                let rpc_query_response = json_rpc_client
                    .blocking_call_view_account(&account_id.clone(), block_reference.clone())
                    .wrap_err_with(|| {
                        format!(
                            "Failed to fetch query ViewAccount for account <{}> on network <{}>",
                            account_id,
                            network_config.network_name
                        )
                    })?;
                let account_view = rpc_query_response.account_view()?;

                let access_key_list = network_config
                    .json_rpc_client()
                    .blocking_call_view_access_key_list(
                        &account_id,
                        block_reference.clone(),
                    )
                    .wrap_err_with(|| {
                        format!(
                            "Failed to fetch ViewAccessKeyList for {}",
                            &account_id
                        )
                    })?
                    .access_key_list_view()?;

                let validators_pledge = crate::common::get_validators_pledge(&json_rpc_client)?;

                let runtime = tokio::runtime::Builder::new_multi_thread()
                    .enable_all()
                    .build()?;
                let concurrency = 10;
                let delegated_pledge: std::collections::BTreeMap<unc_primitives::types::AccountId, unc_token::UncToken> = runtime
                    .block_on(
                        futures::stream::iter(validators_pledge.into_keys())
                        .map(|validator_account_id| async {
                            let balance = get_delegated_pledged_balance(&json_rpc_client, block_reference, &validator_account_id, &account_id).await?;
                            Ok::<_, color_eyre::eyre::Report>((
                                validator_account_id,
                                balance,
                            ))
                        })
                        .buffer_unordered(concurrency)
                        .filter(|balance_result| futures::future::ready(
                            if let Ok((_, balance)) = balance_result {
                                !balance.is_zero()
                            } else {
                                true
                            }
                        ))
                        .try_collect(),
                    )?;

                crate::common::display_account_info(
                    &rpc_query_response.block_hash,
                    &rpc_query_response.block_height,
                    &account_id,
                    &delegated_pledge,
                    &account_view,
                    &access_key_list.keys
                );

                Ok(())
            }
        });
        Ok(Self(crate::network_view_at_block::ArgsForViewContext {
            config: previous_context.config,
            interacting_with_account_ids: vec![scope.account_id.clone().into()],
            on_after_getting_block_reference_callback,
        }))
    }
}

impl From<ViewAccountSummaryContext> for crate::network_view_at_block::ArgsForViewContext {
    fn from(item: ViewAccountSummaryContext) -> Self {
        item.0
    }
}

impl ViewAccountSummary {
    pub fn input_account_id(
        context: &crate::GlobalContext,
    ) -> color_eyre::eyre::Result<Option<crate::types::account_id::AccountId>> {
        crate::common::input_non_signer_account_id_from_used_account_list(
            &context.config.credentials_home_dir,
            "What Account ID do you need to view?",
        )
    }
}

async fn get_delegated_pledged_balance(
    json_rpc_client: &unc_jsonrpc_client::JsonRpcClient,
    block_reference: &unc_primitives::types::BlockReference,
    pledging_pool_account_id: &unc_primitives::types::AccountId,
    account_id: &unc_primitives::types::AccountId,
) -> color_eyre::eyre::Result<unc_token::UncToken> {
    let account_pledged_balance_response = json_rpc_client
        .call(unc_jsonrpc_client::methods::query::RpcQueryRequest {
            block_reference: block_reference.clone(),
            request: unc_primitives::views::QueryRequest::CallFunction {
                account_id: pledging_pool_account_id.clone(),
                method_name: "get_account_pledged_balance".to_string(),
                args: unc_primitives::types::FunctionArgs::from(serde_json::to_vec(
                    &serde_json::json!({
                        "account_id": account_id,
                    }),
                )?),
            },
        })
        .await;
    match account_pledged_balance_response {
        Ok(response) => Ok(unc_token::UncToken::from_attounc(
            response
                .call_result()?
                .parse_result_from_json::<String>()
                .wrap_err("Failed to parse return value of view function call for String.")?
                .parse::<u128>()?,
        )),
        Err(unc_jsonrpc_client::errors::JsonRpcError::ServerError(
            unc_jsonrpc_client::errors::JsonRpcServerError::HandlerError(
                unc_jsonrpc_client::methods::query::RpcQueryError::NoContractCode { .. }
                | unc_jsonrpc_client::methods::query::RpcQueryError::ContractExecutionError {
                    ..
                },
            ),
        )) => Ok(unc_token::UncToken::from_attounc(0)),
        Err(err) => Err(err.into()),
    }
}
