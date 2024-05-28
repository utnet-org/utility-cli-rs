use strum::{EnumDiscriminants, EnumIter, EnumMessage};

mod deposit_and_pledge;
mod pledge;
mod pledge_all;
mod unpledge;
mod unpledge_all;
pub mod view_balance;
mod withdraw;
mod withdraw_all;

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = crate::GlobalContext)]
#[interactive_clap(output_context = PledgeDelegationContext)]
pub struct PledgeDelegation {
    #[interactive_clap(skip_default_input_arg)]
    /// Enter the account that you want to manage delegated pledge for:
    account_id: crate::types::account_id::AccountId,
    #[interactive_clap(subcommand)]
    delegate_pledge_command: PledgeDelegationCommand,
}

#[derive(Debug, Clone)]
pub struct PledgeDelegationContext {
    global_context: crate::GlobalContext,
    account_id: unc_primitives::types::AccountId,
}

impl PledgeDelegationContext {
    pub fn from_previous_context(
        previous_context: crate::GlobalContext,
        scope: &<PledgeDelegation as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        Ok(Self {
            global_context: previous_context,
            account_id: scope.account_id.clone().into(),
        })
    }
}

impl PledgeDelegation {
    pub fn input_account_id(
        context: &crate::GlobalContext,
    ) -> color_eyre::eyre::Result<Option<crate::types::account_id::AccountId>> {
        crate::common::input_non_signer_account_id_from_used_account_list(
            &context.config.credentials_home_dir,
            "Enter the account that you want to manage delegated pledge for:",
        )
    }
}

#[derive(Debug, EnumDiscriminants, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(context = PledgeDelegationContext)]
#[strum_discriminants(derive(EnumMessage, EnumIter))]
#[non_exhaustive]
/// Select actions with delegated pledging:
pub enum PledgeDelegationCommand {
    #[strum_discriminants(strum(
        message = "view-balance         - View the delegated pledge balance for a given account"
    ))]
    /// View the delegated pledge balance for a given account
    ViewBalance(self::view_balance::ViewBalance),
    #[strum_discriminants(strum(
        message = "deposit-and-pledge    - Delegate unc tokens to a validator's pledging pool"
    ))]
    /// Delegate unc tokens to a validator's pledging pool
    DepositAndPledge(self::deposit_and_pledge::DepositAndPledge),
    #[strum_discriminants(strum(
        message = "pledge                - Delegate a certain amount of previously deposited or unpledged unc tokens to a validator's pledging pool"
    ))]
    /// Delegate a certain amount of previously deposited or unpledged unc tokens to a validator's pledging pool
    Pledge(self::pledge::Pledge),
    #[strum_discriminants(strum(
        message = "pledge-all            - Delegate all previously deposited or unpledged unc tokens to a validator's pledging pool"
    ))]
    /// Delegate all previously deposited or unpledged unc tokens to a validator's pledging pool
    PledgeAll(self::pledge_all::PledgeAll),
    #[strum_discriminants(strum(
        message = "unpledge              - Unpledge a certain amount of delegated unc tokens from a avalidator's pledging pool"
    ))]
    /// Unpledge a certain amount of delegated unc tokens from a avalidator's pledging pool
    Unpledge(self::unpledge::Unpledge),
    #[strum_discriminants(strum(
        message = "unpledge-all          - Unpledge all delegated unc tokens from a avalidator's pledging pool"
    ))]
    /// Unpledge all delegated unc tokens from a avalidator's pledging pool
    UnpledgeAll(self::unpledge_all::UnpledgeAll),
    #[strum_discriminants(strum(
        message = "withdraw             - Withdraw a certain amount of unpledged unc tokens from a avalidator's pledging pool"
    ))]
    /// Withdraw a certain amount of unpledged unc tokens from a avalidator's pledging pool
    Withdraw(self::withdraw::Withdraw),
    #[strum_discriminants(strum(
        message = "withdraw-all         - Withdraw all unpledged unc tokens from a avalidator's pledging pool"
    ))]
    /// Withdraw all unpledged unc tokens from a avalidator's pledging pool
    WithdrawAll(self::withdraw_all::WithdrawAll),
}
