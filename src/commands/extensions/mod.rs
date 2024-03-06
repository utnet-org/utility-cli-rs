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
/// What do you want to do with a unc CLI?
pub enum ExtensionsActions {
    #[strum_discriminants(strum(
        message = "register-rsa-keys   - Register TPU rsa keys (root account only)"
    ))]
    RegisterRsaKeys(self::register_rsa_keys::RegisterRsaKeysCommand),

    #[strum_discriminants(strum(
        message = "create-challenge-rsa   - create challenge rsa keys (real miner account)"
    ))]
    CreateChallengeRsa(self::create_challenge_rsa::CreateChallengeRsaCommand),

    #[strum_discriminants(strum(message = "self-update             - Self update unc CLI"))]
    /// Self update unc CLI
    SelfUpdate(self::self_update::SelfUpdateCommand),
}

#[derive(Debug, Deserialize)]
pub struct Rsa2048KeyPair {
    pub challenge_key: unc_crypto::PublicKey,
    pub public_key: unc_crypto::PublicKey,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Miner {
    pub miner_id: String,
    pub public_key: unc_crypto::PublicKey,
    pub power: u64,
    pub sn: String,
    pub bus_id: String,
    pub p2key: String,
}
