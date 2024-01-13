use strum::{EnumDiscriminants, EnumIter, EnumMessage};

mod call_function_args_type;

#[derive(Debug, Clone, EnumDiscriminants, interactive_clap_derive::InteractiveClap)]
#[interactive_clap(context = super::RsaFileContext)]
#[strum_discriminants(derive(EnumMessage, EnumIter))]
/// Select the need for initialization:
pub enum ConstructorMode {
    /// Add an initialize
    #[strum_discriminants(strum(message = "with-init-args  - Add an initialize"))]
    WithInitCall(Initialize),
}

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = super::RsaFileContext)]
#[interactive_clap(output_context = InitializeContext)]
pub struct Initialize {
    #[interactive_clap(value_enum)]
    #[interactive_clap(skip_default_input_arg)]
    /// How do you want to pass the call arguments?
    function_args_type: call_function_args_type::FunctionArgsType,
    /// Enter the arguments to this call:
    function_args: String,

    #[interactive_clap(named_arg)]
    /// Select network
    network_config: crate::network_for_transaction::NetworkForTransactionArgs,
}

impl Initialize {
    fn input_function_args_type(
        _context: &super::RsaFileContext,
    ) -> color_eyre::eyre::Result<
        Option<call_function_args_type::FunctionArgsType>,
    > {
        call_function_args_type::input_function_args_type()
    }
}

#[derive(Debug, Clone)]
pub struct InitializeContext {
    ctx: super::RsaFileContext,
    op_type: u8,
    pass_args: Vec<u8>,
}

impl InitializeContext {
    pub fn from_previous_context(
        previous_context: super::RsaFileContext,
        scope: &<Initialize as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        let pass_args = call_function_args_type::function_args(
                scope.function_args.clone(),
                scope.function_args_type.clone(),
            )?;
            Ok(Self {
                ctx:  super::RsaFileContext {
                    global_context: previous_context.global_context,
                    receiver_account_id: previous_context.receiver_account_id,
                    signer_account_id: previous_context.signer_account_id,
                    public_key: previous_context.public_key,
                    private_key: previous_context.private_key,
                },
                op_type: 0u8,
                pass_args,
            })

    }
}

impl From<InitializeContext> for crate::commands::ActionContext {
    fn from(item: InitializeContext) -> Self {
        let on_after_getting_network_callback: crate::commands::OnAfterGettingNetworkCallback =
            std::sync::Arc::new({
                let signer_account_id = item.ctx.signer_account_id.clone();
                let receiver_account_id = item.ctx.receiver_account_id.clone();

                move |_network_config| {
                    Ok(crate::commands::PrepopulatedTransaction {
                        signer_id: signer_account_id.clone(),
                        receiver_id: receiver_account_id.clone(),
                        actions: vec![near_primitives::transaction::Action::RegisterRsa2048Keys(
                            Box::new(near_primitives::transaction::RegisterRsa2048KeysAction {
                                public_key: item.ctx.public_key.clone(), 
                                operation_type: item.op_type.clone(), 
                                args: item.pass_args.clone(), })
                        )],
                    })
                }
            });

        Self {
            global_context: item.ctx.global_context,
            interacting_with_account_ids: vec![
                item.ctx.signer_account_id,
                item.ctx.receiver_account_id,
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
        }
    }
}
