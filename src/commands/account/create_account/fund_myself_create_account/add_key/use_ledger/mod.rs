#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = super::super::NewAccountContext)]
#[interactive_clap(output_context = AddAccessWithLedgerContext)]
pub struct AddAccessWithLedger {
    #[interactive_clap(named_arg)]
    /// What is the signer account ID?
    sign_as: super::super::sign_as::SignerAccountId,
}

#[derive(Clone)]
pub struct AddAccessWithLedgerContext(super::super::AccountPropertiesContext);

impl AddAccessWithLedgerContext {
    pub fn from_previous_context(
        previous_context: super::super::NewAccountContext,
        _scope: &<AddAccessWithLedger as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        let seed_phrase_hd_path = crate::transaction_signature_options::sign_with_ledger::SignLedger::input_seed_phrase_hd_path()?.unwrap();
        eprintln!(
            "Please allow getting the PublicKey on Ledger device (HD Path: {})",
            seed_phrase_hd_path
        );
        let public_key = unc_ledger::get_public_key(seed_phrase_hd_path.into()).map_err(
            |unc_ledger_error| {
                color_eyre::Report::msg(format!(
                    "An error occurred while trying to get PublicKey from Ledger device: {:?}",
                    unc_ledger_error
                ))
            },
        )?;
        let public_key = unc_crypto::PublicKey::ED25519(unc_crypto::ED25519PublicKey::from(
            public_key.to_bytes(),
        ));

        let account_properties = super::super::AccountProperties {
            new_account_id: previous_context.new_account_id,
            initial_balance: previous_context.initial_balance,
            public_key,
        };

        Ok(Self(super::super::AccountPropertiesContext {
            global_context: previous_context.global_context,
            account_properties,
            on_before_sending_transaction_callback: std::sync::Arc::new(
                |_signed_transaction, _network_config, _message| Ok(()),
            ),
        }))
    }
}

impl From<AddAccessWithLedgerContext> for super::super::AccountPropertiesContext {
    fn from(item: AddAccessWithLedgerContext) -> Self {
        item.0
    }
}
