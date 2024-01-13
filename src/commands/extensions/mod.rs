use serde::Deserialize;
use strum::{EnumDiscriminants, EnumIter, EnumMessage};

pub mod register_rsa_keys;
pub mod create_challenge_rsa;
pub mod self_update;

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(context = crate::GlobalContext)]
pub struct ExtensionsCommands {
    #[interactive_clap(subcommand)]
    pub extensions_actions: ExtensionsActions,
}

#[derive(Debug, EnumDiscriminants, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(context = crate::GlobalContext)]
#[strum_discriminants(derive(EnumMessage, EnumIter))]
#[non_exhaustive]
/// What do you want to do with a near CLI?
pub enum ExtensionsActions {
    #[strum_discriminants(strum(
        message = "register-rsa-keys   - Register TPU rsa keys (root account only)"
    ))]
    RegisterRsaKeys(self::register_rsa_keys::RegisterRsaKeysCommand),

    #[strum_discriminants(strum(
        message = "create-challenge-rsa   - create challenge rsa keys (real miner account)"
    ))]
    CreateChallengeRsa(self::create_challenge_rsa::CreateChallengeRsaCommand),

    #[strum_discriminants(strum(message = "self-update             - Self update near CLI"))]
    /// Self update near CLI
    SelfUpdate(self::self_update::SelfUpdateCommand),
}

#[derive(Debug, Deserialize)]
pub struct Rsa2048KeyPair {
    pub public_key: near_crypto::PublicKey,
    pub private_key: String, // aes encrypted only read from system keychain
}