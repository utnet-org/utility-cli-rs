use strum::{EnumDiscriminants, EnumIter, EnumMessage};

mod pledge_proposal;
mod unpledge_proposal;
mod view_pledge;

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(context = crate::GlobalContext)]
pub struct Pledging {
    #[interactive_clap(subcommand)]
    pledging_command: PledgingCommand,
}

#[derive(Debug, EnumDiscriminants, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(context = crate::GlobalContext)]
#[strum_discriminants(derive(EnumMessage, EnumIter))]
/// What are you up to? (select one of the options with the up-down arrows on your keyboard and press Enter)
pub enum PledgingCommand {
    #[strum_discriminants(strum(message = "view-pledge          -   View validator pledge"))]
    /// View validator pledge
    ViewPledge(self::view_pledge::ViewPledge),
    #[strum_discriminants(strum(
        message = "pledge-proposal      -   To pledge unc directly without a pledging pool"
    ))]
    /// To pledge unc directly without a pledging pool
    PledgeProposal(self::pledge_proposal::PledgeProposal),
    // #[strum_discriminants(strum(
    //     message = "unpledge-proposal    -   To unpledge unc directly without a pledging pool"
    // ))]
    // /// To unpledge unc directly without a pledging pool
    // UnpledgeProposal(self::unpledge_proposal::UnpledgeProposal),
}
