use strum::{EnumDiscriminants, EnumIter, EnumMessage};

pub mod delegate;
mod current_validators;
mod directly;
mod proposals;
mod next_validators;

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
    ValidatorList(self::current_validators::ValidatorList),
    #[strum_discriminants(strum(message = "delegation       - Delegation management"))]
    /// Delegation management
    Delegation(self::delegate::PledgeDelegation),

    #[strum_discriminants(strum(message = "validators   -   Lookup validators for next epoch"))]
    /// Lookup validators for given epoch
    Validators(self::next_validators::Validators),
    #[strum_discriminants(strum(
        message = "proposals    -   Show both new proposals in the current epoch as well as current validators who are implicitly proposing"
    ))]
    /// Show both new proposals in the current epoch as well as current validators who are implicitly proposing
    Proposals(self::proposals::Proposals),
    #[strum_discriminants(strum(
        message = "pledging      -   For validators, there is an option to pledging without deploying a pledging pool smart contract (pledge, unpledge, view pledge)"
    ))]
    /// For validators, there is an option to pledging without deploying a pledging pool smart contract (pledge, unpledge, view pledge)
    Pledging(self::directly::Pledging),
}
