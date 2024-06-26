use color_eyre::eyre::Context;
use prettytable::Table;

use unc_jsonrpc_client::methods::{
    validators::RpcValidatorRequest, EXPERIMENTAL_genesis_config::RpcGenesisConfigRequest,
    EXPERIMENTAL_protocol_config::RpcProtocolConfigRequest,
};
use unc_primitives::types::{BlockId, BlockReference, EpochReference, Finality};

use crate::common::JsonRpcClientExt;

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = super::network_view_at_block::NetworkViewAtBlockArgsContext)]
#[interactive_clap(output_context = AtBlockHeightContext)]
pub struct AtBlockHeight {
    /// Type the block height:
    block_height: unc_primitives::types::BlockHeight,
}

#[derive(Debug, Clone)]
pub struct AtBlockHeightContext;

impl AtBlockHeightContext {
    pub fn from_previous_context(
        previous_context: super::network_view_at_block::NetworkViewAtBlockArgsContext,
        scope: &<AtBlockHeight as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        let epoch_reference = EpochReference::BlockId(BlockId::Height(scope.block_height));
        display_current_validators_info(epoch_reference, &previous_context.network_config)?;
        Ok(Self)
    }
}

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = super::network_view_at_block::NetworkViewAtBlockArgsContext)]
#[interactive_clap(output_context = AtBlockHashContext)]
pub struct AtBlockHash {
    /// Type the block hash:
    block_hash: crate::types::crypto_hash::CryptoHash,
}

#[derive(Debug, Clone)]
pub struct AtBlockHashContext;

impl AtBlockHashContext {
    pub fn from_previous_context(
        previous_context: super::network_view_at_block::NetworkViewAtBlockArgsContext,
        scope: &<AtBlockHash as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        let epoch_reference = EpochReference::BlockId(BlockId::Hash(scope.block_hash.into()));
        display_current_validators_info(epoch_reference, &previous_context.network_config)?;
        Ok(Self)
    }
}

pub fn display_current_validators_info(
    epoch_reference: unc_primitives::types::EpochReference,
    network_config: &crate::config::NetworkConfig,
) -> crate::CliResult {
    let block_reference = match &epoch_reference {
        EpochReference::Latest => BlockReference::Finality(Finality::Final),
        EpochReference::BlockId(block_id) => BlockReference::BlockId(block_id.clone()),
        _ => {
            return Err(color_eyre::eyre::ErrReport::msg(
                "BlockReference: incorrect value entered",
            ))
        }
    };

    let json_rpc_client = network_config.json_rpc_client();

    let mut current_validators = json_rpc_client
        .blocking_call(&RpcValidatorRequest { epoch_reference })
        .wrap_err("Failed to get epoch validators information request.")?
        .current_validators;
    current_validators.sort_by(|a, b| b.pledge.cmp(&a.pledge));

    let genesis_config = json_rpc_client
        .blocking_call(&RpcGenesisConfigRequest)
        .wrap_err("Failed to get genesis config.")?;

    let protocol_config = json_rpc_client
        .blocking_call(&RpcProtocolConfigRequest { block_reference })
        .wrap_err("Failed to get protocol config.")?;

    let max_number_of_seats = protocol_config.num_block_producer_seats
        + protocol_config
            .avg_hidden_validator_seats_per_shard
            .iter()
            .sum::<u64>();
    eprintln!(
        "Validators (total: {}, seat price: {})",
        current_validators.len(),
        crate::common::find_seat_price(
            current_validators
                .iter()
                .map(|current_validator| current_validator.pledge)
                .collect(),
            max_number_of_seats,
            genesis_config.minimum_pledge_ratio,
            protocol_config.protocol_version
        )?
    );

    let mut table = Table::new();
    table.set_titles(prettytable::row![Fg=>"Validator Id", "Pledge", "Online", "Blocks produced", "Blocks expected", "Chunks produced", "Chunks expected"]);

    for validator in &current_validators {
        let online = if validator.num_expected_blocks + validator.num_expected_chunks == 0 {
            "NaN".to_string()
        } else {
            format!(
                "{:>6.2} %",
                ((validator.num_produced_blocks + validator.num_produced_chunks) * 100) as f64
                    / (validator.num_expected_blocks + validator.num_expected_chunks) as f64
            )
        };
        table.add_row(prettytable::row![
            validator.account_id,
            crate::types::unc_token::UncToken::from_attounc(validator.pledge),
            online,
            validator.num_produced_blocks,
            validator.num_expected_blocks,
            validator.num_produced_chunks,
            validator.num_expected_chunks
        ]);
    }
    table.set_format(*prettytable::format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.printstd();
    Ok(())
}
