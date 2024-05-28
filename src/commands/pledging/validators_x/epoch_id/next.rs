use color_eyre::eyre::Context;
use prettytable::Table;

use unc_jsonrpc_client::methods::{
    validators::RpcValidatorRequest, EXPERIMENTAL_genesis_config::RpcGenesisConfigRequest,
    EXPERIMENTAL_protocol_config::RpcProtocolConfigRequest,
};
use unc_primitives::types::{BlockReference, EpochReference, Finality};

use crate::common::JsonRpcClientExt;

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = super::super::network_view_at_block::NetworkViewAtBlockArgsContext)]
#[interactive_clap(output_context = NextContext)]
pub struct Next {}

#[derive(Debug, Clone)]
pub struct NextContext;

impl NextContext {
    pub fn from_previous_context(
        previous_context: super::super::network_view_at_block::NetworkViewAtBlockArgsContext,
        _scope: &<Next as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        display_next_validators_info(&previous_context.network_config)?;
        Ok(Self)
    }
}

fn display_next_validators_info(network_config: &crate::config::NetworkConfig) -> crate::CliResult {
    let json_rpc_client = network_config.json_rpc_client();

    let epoch_validator_info = json_rpc_client
        .blocking_call(&RpcValidatorRequest {
            epoch_reference: EpochReference::Latest,
        })
        .wrap_err("Failed to get epoch validators information request.")?;

    let current_validators = epoch_validator_info.current_validators;
    let mut current_validators_pledge: std::collections::HashMap<
        unc_primitives::types::AccountId,
        unc_primitives::types::Balance,
    > = current_validators
        .into_iter()
        .map(|current_epoch_validator_info| {
            (
                current_epoch_validator_info.account_id,
                current_epoch_validator_info.pledge,
            )
        })
        .collect();

    let mut next_validators = epoch_validator_info.next_validators;
    next_validators.sort_by(|a, b| b.pledge.cmp(&a.pledge));

    let genesis_config = json_rpc_client
        .blocking_call(&RpcGenesisConfigRequest)
        .wrap_err("Failed to get genesis config.")?;

    let protocol_config = json_rpc_client
        .blocking_call(&RpcProtocolConfigRequest {
            block_reference: BlockReference::Finality(Finality::Final),
        })
        .wrap_err("Failed to get protocol config.")?;

    let max_number_of_seats = protocol_config.num_block_producer_seats
        + protocol_config
            .avg_hidden_validator_seats_per_shard
            .iter()
            .sum::<u64>();
    let seat_price = crate::common::find_seat_price(
        next_validators
            .iter()
            .map(|next_validator| next_validator.pledge)
            .collect(),
        max_number_of_seats,
        genesis_config.minimum_pledge_ratio,
        protocol_config.protocol_version,
    )?;
    eprintln!(
        "Next validators (total: {}, seat price: {}):",
        next_validators.len(),
        seat_price
    );

    let mut table = Table::new();
    table.set_titles(
        prettytable::row![Fg=>"#", "Status", "Validator Id", "Previous Pledge", "Pledge"],
    );

    for (index, validator) in next_validators.into_iter().enumerate() {
        let mut previous_pledge = "".to_string();
        let mut status = "New".to_string();
        if let Some(pledge) = current_validators_pledge.remove(&validator.account_id) {
            previous_pledge = crate::types::unc_token::UncToken::from_attounc(pledge).to_string();
            status = "Rewarded".to_string();
        };
        table.add_row(prettytable::row![
            Fg->index + 1,
            status,
            validator.account_id,
            previous_pledge,
            crate::types::unc_token::UncToken::from_attounc(validator.pledge),
        ]);
    }
    for (account_id, previous_pledge) in current_validators_pledge {
        table.add_row(prettytable::row![
            "",
            "Kicked out",
            account_id,
            crate::types::unc_token::UncToken::from_attounc(previous_pledge),
            ""
        ]);
    }

    table.set_format(*prettytable::format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.printstd();
    Ok(())
}
