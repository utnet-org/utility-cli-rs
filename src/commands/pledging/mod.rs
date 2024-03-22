use strum::{EnumDiscriminants, EnumIter, EnumMessage};

pub mod delegate;
mod validator_list;

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(context = crate::GlobalContext)]
pub struct Pledging {
    #[interactive_clap(subcommand)]
    pledge: PledgingType,
}

#[derive(Debug, EnumDiscriminants, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(context = crate::GlobalContext)]
#[strum_discriminants(derive(EnumMessage, EnumIter))]
#[non_exhaustive]
/// Select the type of pledge:
pub enum PledgingType {
    #[strum_discriminants(strum(
        message = "validator-list   - View the list of validators to delegate"
    ))]
    /// View the list of validators to delegate
    ValidatorList(self::validator_list::ValidatorList),
    #[strum_discriminants(strum(message = "delegation       - Delegation management"))]
    /// Delegation management
    Delegation(self::delegate::PledgeDelegation),
}
