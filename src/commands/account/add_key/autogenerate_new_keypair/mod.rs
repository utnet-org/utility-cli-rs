use std::str::FromStr;
use strum::{EnumDiscriminants, EnumIter, EnumMessage};

mod print_keypair_to_terminal;
mod save_keypair_to_keychain;
mod save_keypair_to_legacy_keychain;

#[derive(Debug, Clone, interactive_clap_derive::InteractiveClap)]
#[interactive_clap(input_context = super::access_key_type::AccessTypeContext)]
#[interactive_clap(output_context = GenerateKeypairContext)]
pub struct GenerateKeypair {
    #[interactive_clap(value_enum)]
    #[interactive_clap(skip_default_input_arg)]
    /// How do you want to pass the keys type?
    key_type: super::super::KeysType,

    #[interactive_clap(subcommand)]
    save_mode: SaveMode,
}

impl GenerateKeypair {
    fn input_key_type(
        _context: &super::access_key_type::AccessTypeContext,
    ) -> color_eyre::eyre::Result<Option<super::super::KeysType>> {
        super::super::input_keys_type()
    }
}

#[derive(Debug, Clone)]
pub struct GenerateKeypairContext {
    global_context: crate::GlobalContext,
    signer_account_id: unc_primitives::types::AccountId,
    permission: unc_primitives::account::AccessKeyPermission,
    key_pair_properties: crate::common::KeyPairProperties,
    public_key: unc_crypto::PublicKey,
}

impl GenerateKeypairContext {
    pub fn from_previous_context(
        previous_context: super::access_key_type::AccessTypeContext,
        scope: &<GenerateKeypair as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        let key_type = scope.key_type.clone();
        let key_pair_properties = match key_type {
            super::super::KeysType::Rsa2048 => crate::common::generate_rsa2048_keypair()?,
            super::super::KeysType::Ed25519 => crate::common::generate_ed25519_keypair()?,
        };
        let public_key = unc_crypto::PublicKey::from_str(&key_pair_properties.public_key_str)?;
        Ok(Self {
            global_context: previous_context.global_context,
            signer_account_id: previous_context.signer_account_id,
            permission: previous_context.permission,
            key_pair_properties,
            public_key,
        })
    }
}

#[derive(Debug, Clone, EnumDiscriminants, interactive_clap::InteractiveClap)]
#[interactive_clap(context = GenerateKeypairContext)]
#[strum_discriminants(derive(EnumMessage, EnumIter))]
/// Save an access key for this account:
pub enum SaveMode {
    #[strum_discriminants(strum(
        message = "save-to-keychain   - Save automatically generated key pair to keychain"
    ))]
    /// Save automatically generated key pair to keychain
    SaveToKeychain(self::save_keypair_to_keychain::SaveKeypairToKeychain),
    #[strum_discriminants(strum(
        message = "save-to-legacy-keychain         - Save automatically generated key pair to the legacy keychain (compatible with JS CLI)"
    ))]
    /// Save automatically generated key pair to the legacy keychain (compatible with JS CLI)
    SaveToLegacyKeychain(self::save_keypair_to_legacy_keychain::SaveKeypairToLegacyKeychain),
    #[strum_discriminants(strum(
        message = "print-to-terminal        - Print automatically generated key pair in terminal"
    ))]
    /// Print automatically generated key pair in terminal
    PrintToTerminal(self::print_keypair_to_terminal::PrintKeypairToTerminal),
}
