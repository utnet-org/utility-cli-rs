use color_eyre::eyre::Context;
use prettytable::Table;

use unc_jsonrpc_client::methods::{
    validators::RpcValidatorRequest, EXPERIMENTAL_genesis_config::RpcGenesisConfigRequest,
    EXPERIMENTAL_protocol_config::RpcProtocolConfigRequest,
};
use unc_primitives::types::{BlockReference, EpochReference, Finality};

use crate::common::JsonRpcClientExt;

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = crate::GlobalContext)]
#[interactive_clap(output_context = ProposalsContext)]
pub struct Proposals {
    #[interactive_clap(named_arg)]
    /// Select network
    network_config: crate::network::Network,
}

#[derive(Clone)]
pub struct ProposalsContext(crate::network::NetworkContext);

impl ProposalsContext {
    pub fn from_previous_context(
        previous_context: crate::GlobalContext,
        _scope: &<Proposals as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        let on_after_getting_network_callback: crate::network::OnAfterGettingNetworkCallback =
            std::sync::Arc::new(display_proposals_info);
        Ok(Self(crate::network::NetworkContext {
            config: previous_context.config,
            interacting_with_account_ids: vec![],
            on_after_getting_network_callback,
        }))
    }
}

impl From<ProposalsContext> for crate::network::NetworkContext {
    fn from(item: ProposalsContext) -> Self {
        item.0
    }
}

pub fn display_proposals_info(
    network_config: &crate::config::NetworkConfig,
) -> crate::CliResult {
    let json_rpc_client = network_config.json_rpc_client();

    let epoch_validator_info = json_rpc_client
        .blocking_call(&RpcValidatorRequest {
            epoch_reference: EpochReference::Latest,
        })
        .wrap_err("Failed to get epoch validators information request.")?;

    let current_proposals = epoch_validator_info.current_pledge_proposals;
    let current_proposals_pledge: std::collections::HashMap<
        unc_primitives::types::AccountId,
        unc_primitives::types::Balance,
    > = current_proposals
        .clone()
        .into_iter()
        .map(|validator_pledge_view| {
            let validator_pledge = validator_pledge_view.into_validator_pledge();
            validator_pledge.account_and_pledge()
        })
        .collect();

    let current_validators = epoch_validator_info.current_validators;
    let current_validators_pledge: std::collections::HashMap<
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

    let next_validators = epoch_validator_info.next_validators;
    let mut next_validators_pledge: std::collections::HashMap<
        unc_primitives::types::AccountId,
        unc_primitives::types::Balance,
    > = next_validators
        .into_iter()
        .map(|next_epoch_validator_info| {
            (
                next_epoch_validator_info.account_id,
                next_epoch_validator_info.pledge,
            )
        })
        .collect();

    next_validators_pledge.extend(current_proposals_pledge.clone());

    let mut combine_validators_and_proposals: std::collections::HashMap<
        unc_primitives::types::AccountId,
        ProposalsTable,
    > = std::collections::HashMap::new();
    for (account_id, pledge) in next_validators_pledge {
        if let Some(new_pledge) = current_proposals_pledge.get(&account_id) {
            let proposals_table = ProposalsTable {
                account_id: account_id.clone(),
                status: "Proposal(Accepted)".to_string(),
                pledge,
                new_pledge: Some(*new_pledge),
            };
            combine_validators_and_proposals.insert(account_id, proposals_table)
        } else {
            let proposals_table = ProposalsTable {
                account_id: account_id.clone(),
                status: "Rollover".to_string(),
                pledge,
                new_pledge: None,
            };
            combine_validators_and_proposals.insert(account_id, proposals_table)
        };
    }
    let mut combine_validators_and_proposals_table: Vec<ProposalsTable> =
        combine_validators_and_proposals.into_values().collect();
    combine_validators_and_proposals_table.sort_by(|a, b| b.pledge.cmp(&a.pledge));

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

    let expected_seat_price = crate::common::find_seat_price(
        combine_validators_and_proposals_table
            .iter()
            .map(|proposal| proposal.pledge)
            .collect(),
        max_number_of_seats,
        genesis_config.minimum_pledge_ratio,
        protocol_config.protocol_version,
    )?;

    let passing_proposals = combine_validators_and_proposals_table
        .iter()
        .map(|proposals| match proposals.new_pledge {
            Some(new_pledge) => new_pledge,
            None => proposals.pledge,
        })
        .filter(|pledge| pledge >= &expected_seat_price.as_attounc())
        .count();

    eprintln!(
        "Proposals for the epoch after next (new: {}, passing: {}, expected seat price = {})",
        current_proposals.len(),
        passing_proposals,
        expected_seat_price
    );

    let mut table = Table::new();
    table.set_titles(prettytable::row![Fg=>"#", "Status", "Validator Id", "Pledge", "New Pledge"]);

    for (index, proposals) in combine_validators_and_proposals_table
        .into_iter()
        .enumerate()
    {
        let (new_pledge, status) = match proposals.new_pledge {
            Some(new_pledge) => {
                let status = if new_pledge <= expected_seat_price.as_attounc() {
                    "Proposal(Declined)".to_string()
                } else {
                    proposals.status
                };
                (
                    crate::types::unc_token::UncToken::from_attounc(new_pledge)
                        .to_string(),
                    status,
                )
            }
            None => {
                let status = if proposals.pledge <= expected_seat_price.as_attounc() {
                    "Kicked out".to_string()
                } else {
                    proposals.status
                };

                ("".to_string(), status)
            }
        };
        let pledge = match current_validators_pledge.get(&proposals.account_id) {
            Some(pledge) => {
                crate::types::unc_token::UncToken::from_attounc(*pledge).to_string()
            }
            None => "".to_string(),
        };

        table.add_row(prettytable::row![
            Fg->index + 1,
            status,
            proposals.account_id,
            pledge,
            new_pledge
        ]);
    }
    table.set_format(*prettytable::format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.printstd();
    Ok(())
}

#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct ProposalsTable {
    pub account_id: unc_primitives::types::AccountId,
    pub status: String,
    pub pledge: unc_primitives::types::Balance,
    pub new_pledge: Option<unc_primitives::types::Balance>,
}
