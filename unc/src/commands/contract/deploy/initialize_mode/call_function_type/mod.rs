use inquire::CustomType;

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = super::super::ContractFileContext)]
#[interactive_clap(output_context = CallFunctionActionContext)]
pub struct CallFunctionAction {
    /// What is the name of the function?
    function_name: String,
    #[interactive_clap(value_enum)]
    #[interactive_clap(skip_default_input_arg)]
    /// How do you want to pass the function call arguments?
    function_args_type:
        super::super::super::call_function::call_function_args_type::FunctionArgsType,
    /// Enter the arguments to this function:
    function_args: String,
    #[interactive_clap(named_arg)]
    /// Enter gas for function call
    prepaid_gas: PrepaidGas,
}

#[derive(Debug, Clone)]
pub struct CallFunctionActionContext {
    global_context: crate::GlobalContext,
    receiver_account_id: unc_primitives::types::AccountId,
    signer_account_id: unc_primitives::types::AccountId,
    code: Vec<u8>,
    function_name: String,
    function_args: Vec<u8>,
}

impl CallFunctionActionContext {
    pub fn from_previous_context(
        previous_context: super::super::ContractFileContext,
        scope: &<CallFunctionAction as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        let function_args =
            super::super::super::call_function::call_function_args_type::function_args(
                scope.function_args.clone(),
                scope.function_args_type.clone(),
            )?;
        Ok(Self {
            global_context: previous_context.global_context,
            receiver_account_id: previous_context.receiver_account_id,
            signer_account_id: previous_context.signer_account_id,
            code: previous_context.code,
            function_name: scope.function_name.clone(),
            function_args,
        })
    }
}

impl CallFunctionAction {
    fn input_function_args_type(
        _context: &super::super::ContractFileContext,
    ) -> color_eyre::eyre::Result<
        Option<super::super::super::call_function::call_function_args_type::FunctionArgsType>,
    > {
        super::super::super::call_function::call_function_args_type::input_function_args_type()
    }
}

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = CallFunctionActionContext)]
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
    receiver_account_id: unc_primitives::types::AccountId,
    signer_account_id: unc_primitives::types::AccountId,
    code: Vec<u8>,
    function_name: String,
    function_args: Vec<u8>,
    gas: crate::common::UncGas,
}

impl PrepaidGasContext {
    pub fn from_previous_context(
        previous_context: CallFunctionActionContext,
        scope: &<PrepaidGas as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        Ok(Self {
            global_context: previous_context.global_context,
            receiver_account_id: previous_context.receiver_account_id,
            signer_account_id: previous_context.signer_account_id,
            code: previous_context.code,
            function_name: previous_context.function_name,
            function_args: previous_context.function_args,
            gas: scope.gas,
        })
    }
}

impl PrepaidGas {
    fn input_gas(
        _context: &CallFunctionActionContext,
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
        let deposit = scope.deposit;

        let on_after_getting_network_callback: crate::commands::OnAfterGettingNetworkCallback =
            std::sync::Arc::new({
                let signer_account_id = previous_context.signer_account_id.clone();
                let receiver_account_id = previous_context.receiver_account_id.clone();

                move |_network_config| {
                    Ok(crate::commands::PrepopulatedTransaction {
                        signer_id: signer_account_id.clone(),
                        receiver_id: receiver_account_id.clone(),
                        actions: vec![
                            unc_primitives::transaction::Action::DeployContract(
                                unc_primitives::transaction::DeployContractAction {
                                    code: previous_context.code.clone(),
                                },
                            ),
                            unc_primitives::transaction::Action::FunctionCall(Box::new(
                                unc_primitives::transaction::FunctionCallAction {
                                    method_name: previous_context.function_name.clone(),
                                    args: previous_context.function_args.clone(),
                                    gas: previous_context.gas.as_gas(),
                                    deposit: deposit.as_attounc(),
                                },
                            )),
                        ],
                    })
                }
            });

        Ok(Self(crate::commands::ActionContext {
            global_context: previous_context.global_context,
            interacting_with_account_ids: vec![
                previous_context.signer_account_id.clone(),
                previous_context.receiver_account_id.clone(),
            ],
            on_after_getting_network_callback,
            on_before_signing_callback: std::sync::Arc::new(
                |_prepolulated_unsinged_transaction, _network_config| Ok(()),
            ),
            on_before_sending_transaction_callback: std::sync::Arc::new(
                |_signed_transaction, _network_config, _message| Ok(()),
            ),
            on_after_sending_transaction_callback: std::sync::Arc::new(
                |_outcome_view, _network_config| Ok(()),
            ),
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
            .with_starting_input("0 unc")
            .prompt()?,
        ))
    }
}
