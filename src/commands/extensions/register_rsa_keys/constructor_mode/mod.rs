use color_eyre::eyre::Context;
use strum::{EnumDiscriminants, EnumIter, EnumMessage};

const ONE_TERA: u64 = 10u64.pow(12);

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
    #[interactive_clap(named_arg)]
    /// Select network
    network_config: crate::network_for_transaction::NetworkForTransactionArgs,
}

#[derive(Debug, Clone)]
pub struct InitializeContext {
    ctx: super::RsaFileContext,
    action: Vec<unc_primitives::transaction::Action>,
}

impl InitializeContext {
    pub fn from_previous_context(
        previous_context: super::RsaFileContext,
        _scope: &<Initialize as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {

        let mut actions = Vec::new();
        for m in previous_context.miners.iter() {
            let data = format!(r#"
            {{
                "power": "{}",
                "sn": "{}",
                "bus_id": "{}",
                "p2key": "{}"
            }}
            "#, m.power, m.sn, m.bus_id, m.p2key);

            let data_json: serde_json::Value = serde_json::from_str(&data).unwrap();
            let args = serde_json::to_vec(&data_json).wrap_err("Internal error!").unwrap();

            actions.push(
                unc_primitives::transaction::Action::RegisterRsa2048Keys(
                    Box::new(unc_primitives::transaction::RegisterRsa2048KeysAction {
                        public_key: m.public_key.clone(),
                        operation_type: 0u8,
                        args: args.clone(),
                    }),
                )
            )
            
        }
        Ok(Self {
            ctx:  super::RsaFileContext {
                global_context: previous_context.global_context,
                receiver_account_id: previous_context.receiver_account_id,
                signer_account_id: previous_context.signer_account_id,
                miners: previous_context.miners,
            },
            action: actions,
        })
    }
}

impl From<InitializeContext> for crate::commands::ActionContext {
    fn from(item: InitializeContext) -> Self {
        let on_after_getting_network_callback: crate::commands::OnAfterGettingNetworkCallback =
            std::sync::Arc::new({
                let signer_account_id = item.ctx.signer_account_id.clone();
                let receiver_account_id = item.ctx.receiver_account_id.clone();
                let actions = item.action.clone();
                move |_network_config| {
                    Ok(crate::commands::PrepopulatedTransaction {
                        signer_id: signer_account_id.clone(),
                        receiver_id: receiver_account_id.clone(),
                        actions: actions.clone(),
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
