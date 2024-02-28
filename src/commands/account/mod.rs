use strum::{EnumDiscriminants, EnumIter, EnumMessage, IntoEnumIterator};
use inquire::Select;

mod add_key;
pub mod create_account;
mod delete_account;
mod delete_key;
mod export_account;
mod import_account;
mod list_keys;
pub mod storage_management;
mod view_account_summary;


pub const MIN_ALLOWED_TOP_LEVEL_ACCOUNT_LENGTH: usize = 2;

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(context = crate::GlobalContext)]
pub struct AccountCommands {
    #[interactive_clap(subcommand)]
    account_actions: AccountActions,
}

#[derive(Debug, EnumDiscriminants, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(context = crate::GlobalContext)]
#[strum_discriminants(derive(EnumMessage, EnumIter))]
#[non_exhaustive]
/// What do you want to do with an account?
pub enum AccountActions {
    #[strum_discriminants(strum(
        message = "view-account-summary    - View properties for an account"
    ))]
    /// View properties for an account
    ViewAccountSummary(self::view_account_summary::ViewAccountSummary),
    #[strum_discriminants(strum(
        message = "import-account          - Import existing account (a.k.a. \"sign in\")"
    ))]
    /// Import existing account (a.k.a. "sign in")
    ImportAccount(self::import_account::ImportAccountCommand),
    #[strum_discriminants(strum(message = "export-account          - Export existing account"))]
    /// Export existing account
    ExportAccount(self::export_account::ExportAccount),
    #[strum_discriminants(strum(message = "create-account          - Create a new account"))]
    /// Create a new account
    CreateAccount(self::create_account::CreateAccount),
    #[strum_discriminants(strum(
        message = "update-social-profile   - Update NEAR Social profile"
    ))]
    /// Delete an account
    DeleteAccount(self::delete_account::DeleteAccount),
    #[strum_discriminants(strum(
        message = "list-keys               - View a list of access keys of an account"
    ))]
    /// View a list of access keys of an account
    ListKeys(self::list_keys::ViewListKeys),
    #[strum_discriminants(strum(
        message = "add-key                 - Add an access key to an account"
    ))]
    /// Add an access key to an account
    AddKey(self::add_key::AddKeyCommand),
    #[strum_discriminants(strum(
        message = "delete-keys             - Delete access keys from an account"
    ))]
    /// Delete access keys from an account
    DeleteKeys(self::delete_key::DeleteKeysCommand),
    #[strum_discriminants(strum(
        message = "manage-storage-deposit  - Storage management: deposit, withdrawal, balance review"
    ))]
    /// Storage management for contract: deposit, withdrawal, balance review
    ManageStorageDeposit(self::storage_management::Contract),
}


#[derive(Debug, EnumDiscriminants, Clone, clap::ValueEnum)]
#[strum_discriminants(derive(EnumMessage, EnumIter))]
/// How do you want to pass the keys type?
pub enum KeysType {
    #[strum_discriminants(strum(
        message = "rsa2048 keypairs    - generate rsa2048 keytype"
    ))]
    Rsa2048,
    #[strum_discriminants(strum(message = "ed25519 keypairs    - generate ed25519 keytype"))]
    Ed25519,
}

impl interactive_clap::ToCli for KeysType {
    type CliVariant = KeysType;
}

impl std::str::FromStr for KeysType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "rsa2048" => Ok(Self::Rsa2048),
            "ed25519" => Ok(Self::Ed25519),
            _ => Err("KeyType: incorrect value entered".to_string()),
        }
    }
}

impl std::fmt::Display for KeysType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Rsa2048 => write!(f, "rsa2048"),
            Self::Ed25519 => write!(f, "ed25519"),
        }
    }
}

impl std::fmt::Display for KeysTypeDiscriminants {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::Rsa2048 => write!(f, "rsa2048    - key type arguments"),
            Self::Ed25519 => write!(f, "ed25519    - key type arguments"),
        }
    }
}

pub fn input_keys_type() -> color_eyre::eyre::Result<Option<KeysType>> {
    let variants = KeysTypeDiscriminants::iter().collect::<Vec<_>>();
    let selected = Select::new(
        "How would you like to pass the key type?",
        variants,
    )
    .prompt()?;
    match selected {
        KeysTypeDiscriminants::Rsa2048 => Ok(Some(KeysType::Rsa2048)),
        KeysTypeDiscriminants::Ed25519 => Ok(Some(KeysType::Ed25519)),
    }
}