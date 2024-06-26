use strum::{EnumDiscriminants, EnumIter, EnumMessage};

pub mod delegate;
mod directly;
mod proposals;
mod validators;
mod validators_x;

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
        message = "validator-list   - View the list of current validators to delegate"
    ))]
    /// View the list of validators to delegate
    ValidatorList(self::validators::ValidatorList),
    #[strum_discriminants(strum(message = "delegation       - Delegation management"))]
    /// Delegation management
    Delegation(self::delegate::PledgeDelegation),

    #[strum_discriminants(strum(message = "validators   -   Lookup validators for next epoch"))]
    /// Lookup validators for given epoch
    Validators(self::validators_x::Validators),
    #[strum_discriminants(strum(
        message = "proposals    -   Show both new proposals in the current epoch as well as current validators who are implicitly proposing"
    ))]
    /// Show both new proposals in the current epoch as well as current validators who are implicitly proposing
    Proposals(self::proposals::Proposals),
    #[strum_discriminants(strum(
        message = "directly      -   For validators, there is an option to pledging without deploying a pledging pool smart contract (pledge, unpledge, view pledge)"
    ))]
    /// For validators, there is an option to pledging without deploying a pledging pool smart contract (pledge, unpledge, view pledge)
    Directly(self::directly::Pledging),
}
