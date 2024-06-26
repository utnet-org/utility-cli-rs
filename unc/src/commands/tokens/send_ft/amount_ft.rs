use color_eyre::eyre::{Context, ContextCompat};
use inquire::CustomType;
use serde_json::{json, Value};

use crate::common::CallResultExt;
use crate::common::JsonRpcClientExt;

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = super::SendFtCommandContext)]
#[interactive_clap(output_context = AmountFtContext)]
pub struct AmountFt {
    #[interactive_clap(skip_default_input_arg)]
    /// Enter an amount FT to transfer:
    amount_ft: crate::types::ft_properties::FungibleToken,
    #[interactive_clap(named_arg)]
    /// Enter gas for function call
    prepaid_gas: PrepaidGas,
}

#[derive(Debug, Clone)]
pub struct AmountFtContext {
    global_context: crate::GlobalContext,
    signer_account_id: unc_primitives::types::AccountId,
    ft_contract_account_id: unc_primitives::types::AccountId,
    receiver_account_id: unc_primitives::types::AccountId,
    amount_ft: crate::types::ft_properties::FungibleToken,
}

impl AmountFtContext {
    pub fn from_previous_context(
        previous_context: super::SendFtCommandContext,
        scope: &<AmountFt as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        let network_config = crate::common::find_network_where_account_exist(
            &previous_context.global_context,
            previous_context.ft_contract_account_id.clone(),
        )
        .wrap_err_with(|| {
            format!(
                "Contract <{}> does not exist in networks",
                previous_context.ft_contract_account_id
            )
        })?;
        let ft_metadata = crate::types::ft_properties::params_ft_metadata(
            previous_context.ft_contract_account_id.clone(),
            &network_config,
            unc_primitives::types::Finality::Final.into(),
        )?;

        Ok(Self {
            global_context: previous_context.global_context,
            signer_account_id: previous_context.signer_account_id,
            ft_contract_account_id: previous_context.ft_contract_account_id,
            receiver_account_id: previous_context.receiver_account_id,
            amount_ft: scope.amount_ft.normalize(&ft_metadata)?,
        })
    }
}

impl AmountFt {
    fn input_amount_ft(
        context: &super::SendFtCommandContext,
    ) -> color_eyre::eyre::Result<Option<crate::types::ft_properties::FungibleToken>> {
        let network_config = crate::common::find_network_where_account_exist(
            &context.global_context,
            context.ft_contract_account_id.clone(),
        )
        .wrap_err_with(|| {
            format!(
                "Contract <{}> does not exist in networks",
                context.ft_contract_account_id
            )
        })?;

        let ft_metadata = crate::types::ft_properties::params_ft_metadata(
            context.ft_contract_account_id.clone(),
            &network_config,
            unc_primitives::types::Finality::Final.into(),
        )?;
        eprintln!();

        Ok(Some(
            CustomType::<crate::types::ft_properties::FungibleToken>::new(&format!(
                "Enter an FT amount to transfer (example: 10 {symbol} or 0.5 {symbol}):",
                symbol = ft_metadata.symbol
            ))
            .with_validator(move |ft: &crate::types::ft_properties::FungibleToken| {
                match ft.normalize(&ft_metadata) {
                    Err(err) => Ok(inquire::validator::Validation::Invalid(
                        inquire::validator::ErrorMessage::Custom(err.to_string()),
                    )),
                    Ok(_) => Ok(inquire::validator::Validation::Valid),
                }
            })
            .with_formatter(&|ft| ft.to_string())
            .prompt()?,
        ))
    }
}

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = AmountFtContext)]
#[interactive_clap(output_context = PrepaidGasContext)]
pub struct PrepaidGas {
    #[interactive_clap(skip_default_input_arg)]
    /// Enter gas for function call:
    gas: crate::common::UncGas,
    #[interactive_clap(named_arg)]
    /// Enter deposit for a function call
    attached_deposit: Deposit,
}

#[derive(Debug, Clone)]
pub struct PrepaidGasContext {
    global_context: crate::GlobalContext,
    signer_account_id: unc_primitives::types::AccountId,
    ft_contract_account_id: unc_primitives::types::AccountId,
    receiver_account_id: unc_primitives::types::AccountId,
    amount_ft: crate::types::ft_properties::FungibleToken,
    gas: crate::common::UncGas,
}

impl PrepaidGasContext {
    pub fn from_previous_context(
        previous_context: AmountFtContext,
        scope: &<PrepaidGas as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        Ok(Self {
            global_context: previous_context.global_context,
            signer_account_id: previous_context.signer_account_id,
            ft_contract_account_id: previous_context.ft_contract_account_id,
            receiver_account_id: previous_context.receiver_account_id,
            amount_ft: previous_context.amount_ft,
            gas: scope.gas,
        })
    }
}

impl PrepaidGas {
    fn input_gas(
        _context: &AmountFtContext,
    ) -> color_eyre::eyre::Result<Option<crate::common::UncGas>> {
        eprintln!();
        Ok(Some(
            CustomType::new("Enter gas for function call:")
                .with_starting_input("100 TeraGas")
                .with_validator(move |gas: &crate::common::UncGas| {
                    if gas > &unc_gas::UncGas::from_tgas(300) {
                        Ok(inquire::validator::Validation::Invalid(
                            inquire::validator::ErrorMessage::Custom(
                                "You need to enter a value of no more than 300 TeraGas".to_string(),
                            ),
                        ))
                    } else {
                        Ok(inquire::validator::Validation::Valid)
                    }
                })
                .prompt()?,
        ))
    }
}

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = PrepaidGasContext)]
#[interactive_clap(output_context = DepositContext)]
pub struct Deposit {
    #[interactive_clap(skip_default_input_arg)]
    /// Enter deposit for a function call:
    deposit: crate::types::unc_token::UncToken,
    #[interactive_clap(named_arg)]
    /// Select network
    network_config: crate::network_for_transaction::NetworkForTransactionArgs,
}

#[derive(Clone)]
pub struct DepositContext(crate::commands::ActionContext);

impl DepositContext {
    pub fn from_previous_context(
        previous_context: PrepaidGasContext,
        scope: &<Deposit as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        let on_after_getting_network_callback: crate::commands::OnAfterGettingNetworkCallback =
            std::sync::Arc::new({
                let signer_account_id = previous_context.signer_account_id.clone();
                let ft_contract_account_id = previous_context.ft_contract_account_id.clone();
                let receiver_account_id = previous_context.receiver_account_id.clone();
                let deposit = scope.deposit;
                let amount_ft = previous_context.amount_ft.clone();
                let action_ft_transfer = unc_primitives::transaction::Action::FunctionCall(
                    Box::new(unc_primitives::transaction::FunctionCallAction {
                        method_name: "ft_transfer".to_string(),
                        args: serde_json::to_vec(&json!({
                            "receiver_id": receiver_account_id.to_string(),
                            "amount": amount_ft.amount().to_string()
                        }))?,
                        gas: previous_context.gas.as_gas(),
                        deposit: deposit.as_attounc(),
                    }),
                );

                move |network_config| {
                    let args = serde_json::to_vec(&json!({
                    "account_id": receiver_account_id.to_string(),
                    }))?;
                    let call_result = network_config
                        .json_rpc_client()
                        .blocking_call_view_function(
                            &ft_contract_account_id,
                            "storage_balance_of",
                            args.clone(),
                            unc_primitives::types::Finality::Final.into(),
                        )
                        .wrap_err_with(||{
                            format!("Failed to fetch query for view method: 'storage_balance_of' (contract <{}> on network <{}>)",
                                ft_contract_account_id,
                                network_config.network_name
                            )
                        })?;

                    if call_result.parse_result_from_json::<Value>()?.is_null() {
                        let action_storage_deposit =
                            unc_primitives::transaction::Action::FunctionCall(Box::new(
                                unc_primitives::transaction::FunctionCallAction {
                                    method_name: "storage_deposit".to_string(),
                                    args,
                                    gas: previous_context.gas.as_gas(),
                                    deposit: unc_token::UncToken::from_milliunc(100).as_attounc(),
                                },
                            ));
                        return Ok(crate::commands::PrepopulatedTransaction {
                            signer_id: signer_account_id.clone(),
                            receiver_id: ft_contract_account_id.clone(),
                            actions: vec![action_storage_deposit, action_ft_transfer.clone()],
                        });
                    }

                    Ok(crate::commands::PrepopulatedTransaction {
                        signer_id: signer_account_id.clone(),
                        receiver_id: ft_contract_account_id.clone(),
                        actions: vec![action_ft_transfer.clone()],
                    })
                }
            });

        let on_after_sending_transaction_callback: crate::transaction_signature_options::OnAfterSendingTransactionCallback = std::sync::Arc::new({
            let signer_account_id = previous_context.signer_account_id.clone();
            let amount_ft = previous_context.amount_ft.clone();
            let ft_contract_account_id = previous_context.ft_contract_account_id.clone();
            let receiver_account_id = previous_context.receiver_account_id.clone();

            move |outcome_view, _network_config| {
                if let unc_primitives::views::FinalExecutionStatus::SuccessValue(_) = outcome_view.status {
                    eprintln!(
                        "<{signer_account_id}> has successfully transferred {amount_ft} (FT-contract: {ft_contract_account_id}) to <{receiver_account_id}>.",
                    );
                }
                Ok(())
            }
        });

        Ok(Self(crate::commands::ActionContext {
            global_context: previous_context.global_context,
            interacting_with_account_ids: vec![
                previous_context.ft_contract_account_id,
                previous_context.signer_account_id,
                previous_context.receiver_account_id,
            ],
            on_after_getting_network_callback,
            on_before_signing_callback: std::sync::Arc::new(
                |_prepolulated_unsinged_transaction, _network_config| Ok(()),
            ),
            on_before_sending_transaction_callback: std::sync::Arc::new(
                |_signed_transaction, _network_config, _message| Ok(()),
            ),
            on_after_sending_transaction_callback,
        }))
    }
}

impl From<DepositContext> for crate::commands::ActionContext {
    fn from(item: DepositContext) -> Self {
        item.0
    }
}

impl Deposit {
    fn input_deposit(
        _context: &PrepaidGasContext,
    ) -> color_eyre::eyre::Result<Option<crate::types::unc_token::UncToken>> {
        eprintln!();
        Ok(Some(
            CustomType::new(
                "Enter deposit for a function call (example: 10 UNC or 0.5 unc or 10000 attounc):",
            )
            .with_starting_input("1 attounc")
            .prompt()?,
        ))
    }
}
