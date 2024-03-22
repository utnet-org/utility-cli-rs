use std::collections::VecDeque;
use std::convert::{TryFrom, TryInto};
use std::io::Write;
use std::str::FromStr;

use color_eyre::eyre::{ContextCompat, WrapErr};
use futures::{StreamExt, TryStreamExt};
use prettytable::Table;

use unc_primitives::{hash::CryptoHash, types::BlockReference, views::AccessKeyPermissionView};

pub type CliResult = color_eyre::eyre::Result<()>;

use inquire::{Select, Text};
use strum::IntoEnumIterator;

use rand::Rng;
use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey};
use rsa::{RsaPrivateKey, RsaPublicKey};

pub fn get_unc_exec_path() -> String {
    std::env::args()
        .next()
        .unwrap_or_else(|| "./unc".to_owned())
}

#[derive(
    Debug,
    Clone,
    strum_macros::IntoStaticStr,
    strum_macros::EnumString,
    strum_macros::EnumVariantNames,
    smart_default::SmartDefault,
)]
#[strum(serialize_all = "snake_case")]
pub enum OutputFormat {
    #[default]
    Plaintext,
    Json,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OutputFormat::Plaintext => write!(f, "plaintext"),
            OutputFormat::Json => write!(f, "json"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlockHashAsBase58 {
    pub inner: unc_primitives::hash::CryptoHash,
}

impl std::str::FromStr for BlockHashAsBase58 {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self {
            inner: bs58::decode(s)
                .into_vec()
                .map_err(|err| format!("base58 block hash sequence is invalid: {}", err))?
                .as_slice()
                .try_into()
                .map_err(|err| format!("block hash could not be collected: {}", err))?,
        })
    }
}

impl std::fmt::Display for BlockHashAsBase58 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BlockHash {}", self.inner)
    }
}

pub use unc_gas::UncGas;

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd)]
pub struct TransferAmount {
    amount: unc_token::UncToken,
}

impl interactive_clap::ToCli for TransferAmount {
    type CliVariant = unc_token::UncToken;
}

impl std::fmt::Display for TransferAmount {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.amount)
    }
}

impl TransferAmount {
    pub fn from(
        amount: unc_token::UncToken,
        account_transfer_allowance: &AccountTransferAllowance,
    ) -> color_eyre::eyre::Result<Self> {
        if amount <= account_transfer_allowance.transfer_allowance() {
            Ok(Self { amount })
        } else {
            Err(color_eyre::Report::msg(
                "the amount exceeds the transfer allowance",
            ))
        }
    }

    pub fn from_unchecked(amount: unc_token::UncToken) -> Self {
        Self { amount }
    }

    pub fn as_yoctounc(&self) -> u128 {
        self.amount.as_yoctounc()
    }
}

impl From<TransferAmount> for unc_token::UncToken {
    fn from(item: TransferAmount) -> Self {
        item.amount
    }
}

#[derive(Debug)]
pub struct AccountTransferAllowance {
    account_id: unc_primitives::types::AccountId,
    account_liquid_balance: unc_token::UncToken,
    account_locked_balance: unc_token::UncToken,
    storage_pledge: unc_token::UncToken,
    pessimistic_transaction_fee: unc_token::UncToken,
}

impl std::fmt::Display for AccountTransferAllowance {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt,
            "\n{} account has {} available for transfer (the total balance is {}, but {} is locked for storage and the transfer transaction fee is ~{})",
            self.account_id,
            self.transfer_allowance(),
            self.account_liquid_balance,
            self.liquid_storage_pledge(),
            self.pessimistic_transaction_fee
        )
    }
}

impl AccountTransferAllowance {
    pub fn liquid_storage_pledge(&self) -> unc_token::UncToken {
        self.storage_pledge
            .saturating_sub(self.account_locked_balance)
    }

    pub fn transfer_allowance(&self) -> unc_token::UncToken {
        self.account_liquid_balance
            .saturating_sub(self.liquid_storage_pledge())
            .saturating_sub(self.pessimistic_transaction_fee)
    }
}

pub fn get_account_transfer_allowance(
    network_config: crate::config::NetworkConfig,
    account_id: unc_primitives::types::AccountId,
    block_reference: BlockReference,
) -> color_eyre::eyre::Result<AccountTransferAllowance> {
    let account_view = if let Ok(account_view) =
        get_account_state(network_config.clone(), account_id.clone(), block_reference)
    {
        account_view
    } else if !account_id.get_account_type().is_implicit() {
        return color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!(
            "Account <{}> does not exist on network <{}>.",
            account_id,
            network_config.network_name
        ));
    } else {
        return Ok(AccountTransferAllowance {
            account_id,
            account_liquid_balance: unc_token::UncToken::from_unc(0),
            account_locked_balance: unc_token::UncToken::from_unc(0),
            storage_pledge: unc_token::UncToken::from_unc(0),
            pessimistic_transaction_fee: unc_token::UncToken::from_unc(0),
        });
    };
    let storage_amount_per_byte = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(network_config.json_rpc_client().call(
            unc_jsonrpc_client::methods::EXPERIMENTAL_protocol_config::RpcProtocolConfigRequest {
                block_reference: unc_primitives::types::Finality::Final.into(),
            },
        ))
        .wrap_err("RpcError")?
        .runtime_config
        .storage_amount_per_byte;

    Ok(AccountTransferAllowance {
        account_id,
        account_liquid_balance: unc_token::UncToken::from_yoctounc(account_view.amount),
        account_locked_balance: unc_token::UncToken::from_yoctounc(account_view.pledging),
        storage_pledge: unc_token::UncToken::from_yoctounc(
            u128::from(account_view.storage_usage) * storage_amount_per_byte,
        ),
        // pessimistic_transaction_fee = 10^21 - this value is set temporarily
        // In the future, its value will be calculated by the function: fn tx_cost(...)
        // https://github.com/unc/unccore/blob/8a377fda0b4ce319385c463f1ae46e4b0b29dcd9/runtime/runtime/src/config.rs#L178-L232
        pessimistic_transaction_fee: unc_token::UncToken::from_milliunc(1),
    })
}

pub fn verify_account_access_key(
    account_id: unc_primitives::types::AccountId,
    public_key: unc_crypto::PublicKey,
    network_config: crate::config::NetworkConfig,
) -> color_eyre::eyre::Result<
    unc_primitives::views::AccessKeyView,
    unc_jsonrpc_client::errors::JsonRpcError<unc_jsonrpc_primitives::types::query::RpcQueryError>,
> {
    loop {
        match network_config
            .json_rpc_client()
            .blocking_call_view_access_key(
                &account_id,
                &public_key,
                unc_primitives::types::BlockReference::latest(),
            ) {
            Ok(rpc_query_response) => {
                if let unc_jsonrpc_primitives::types::query::QueryResponseKind::AccessKey(result) =
                    rpc_query_response.kind
                {
                    return Ok(result);
                } else {
                    return Err(unc_jsonrpc_client::errors::JsonRpcError::TransportError(unc_jsonrpc_client::errors::RpcTransportError::RecvError(
                        unc_jsonrpc_client::errors::JsonRpcTransportRecvError::UnexpectedServerResponse(
                            unc_jsonrpc_primitives::message::Message::error(unc_jsonrpc_primitives::errors::RpcError::parse_error("Transport error: unexpected server response".to_string()))
                        ),
                    )));
                }
            }
            Err(
                err @ unc_jsonrpc_client::errors::JsonRpcError::ServerError(
                    unc_jsonrpc_client::errors::JsonRpcServerError::HandlerError(
                        unc_jsonrpc_primitives::types::query::RpcQueryError::UnknownAccessKey {
                            ..
                        },
                    ),
                ),
            ) => {
                return Err(err);
            }
            Err(unc_jsonrpc_client::errors::JsonRpcError::TransportError(err)) => {
                eprintln!("\nAccount information ({}) cannot be fetched on <{}> network due to connectivity issue.",
                    account_id, network_config.network_name
                );
                if !need_check_account() {
                    return Err(unc_jsonrpc_client::errors::JsonRpcError::TransportError(
                        err,
                    ));
                }
            }
            Err(unc_jsonrpc_client::errors::JsonRpcError::ServerError(err)) => {
                eprintln!("\nAccount information ({}) cannot be fetched on <{}> network due to server error.",
                    account_id, network_config.network_name
                );
                if !need_check_account() {
                    return Err(unc_jsonrpc_client::errors::JsonRpcError::ServerError(err));
                }
            }
        }
    }
}

pub fn is_account_exist(
    networks: &linked_hash_map::LinkedHashMap<String, crate::config::NetworkConfig>,
    account_id: unc_primitives::types::AccountId,
) -> bool {
    for (_, network_config) in networks {
        if get_account_state(
            network_config.clone(),
            account_id.clone(),
            unc_primitives::types::Finality::Final.into(),
        )
        .is_ok()
        {
            return true;
        }
    }
    false
}

pub fn find_network_where_account_exist(
    context: &crate::GlobalContext,
    new_account_id: unc_primitives::types::AccountId,
) -> Option<crate::config::NetworkConfig> {
    for (_, network_config) in context.config.network_connection.iter() {
        if get_account_state(
            network_config.clone(),
            new_account_id.clone(),
            unc_primitives::types::BlockReference::latest(),
        )
        .is_ok()
        {
            return Some(network_config.clone());
        }
    }
    None
}

pub fn ask_if_different_account_id_wanted() -> color_eyre::eyre::Result<bool> {
    #[derive(strum_macros::Display, PartialEq)]
    enum ConfirmOptions {
        #[strum(to_string = "Yes, I want to enter a new name for account ID.")]
        Yes,
        #[strum(to_string = "No, I want to keep using this name for account ID.")]
        No,
    }
    let select_choose_input = Select::new(
        "Do you want to enter a different name for the new account ID?",
        vec![ConfirmOptions::Yes, ConfirmOptions::No],
    )
    .prompt()?;
    Ok(select_choose_input == ConfirmOptions::Yes)
}

pub fn get_account_state(
    network_config: crate::config::NetworkConfig,
    account_id: unc_primitives::types::AccountId,
    block_reference: BlockReference,
) -> color_eyre::eyre::Result<
    unc_primitives::views::AccountView,
    unc_jsonrpc_client::errors::JsonRpcError<unc_jsonrpc_primitives::types::query::RpcQueryError>,
> {
    loop {
        let query_view_method_response = network_config
            .json_rpc_client()
            .blocking_call_view_account(&account_id.clone(), block_reference.clone());
        match query_view_method_response {
            Ok(rpc_query_response) => {
                if let unc_jsonrpc_primitives::types::query::QueryResponseKind::ViewAccount(
                    account_view,
                ) = rpc_query_response.kind
                {
                    return Ok(account_view);
                } else {
                    return Err(unc_jsonrpc_client::errors::JsonRpcError::TransportError(unc_jsonrpc_client::errors::RpcTransportError::RecvError(
                        unc_jsonrpc_client::errors::JsonRpcTransportRecvError::UnexpectedServerResponse(
                            unc_jsonrpc_primitives::message::Message::error(unc_jsonrpc_primitives::errors::RpcError::parse_error("Transport error: unexpected server response".to_string()))
                        ),
                    )));
                }
            }
            Err(
                err @ unc_jsonrpc_client::errors::JsonRpcError::ServerError(
                    unc_jsonrpc_client::errors::JsonRpcServerError::HandlerError(
                        unc_jsonrpc_primitives::types::query::RpcQueryError::UnknownAccount {
                            ..
                        },
                    ),
                ),
            ) => {
                return Err(err);
            }
            Err(unc_jsonrpc_client::errors::JsonRpcError::TransportError(err)) => {
                eprintln!("\nAccount information ({}) cannot be fetched on <{}> network due to connectivity issue.",
                    account_id, network_config.network_name
                );
                if !need_check_account() {
                    return Err(unc_jsonrpc_client::errors::JsonRpcError::TransportError(
                        err,
                    ));
                }
            }
            Err(unc_jsonrpc_client::errors::JsonRpcError::ServerError(err)) => {
                eprintln!("\nAccount information ({}) cannot be fetched on <{}> network due to server error.",
                    account_id, network_config.network_name
                );
                if !need_check_account() {
                    return Err(unc_jsonrpc_client::errors::JsonRpcError::ServerError(err));
                }
            }
        }
    }
}

fn need_check_account() -> bool {
    #[derive(strum_macros::Display, PartialEq)]
    enum ConfirmOptions {
        #[strum(to_string = "Yes, I want to check the account again.")]
        Yes,
        #[strum(to_string = "No, I want to skip the check and use the specified account ID.")]
        No,
    }
    let select_choose_input = Select::new(
        "Do you want to try again?",
        vec![ConfirmOptions::Yes, ConfirmOptions::No],
    )
    .prompt()
    .unwrap_or(ConfirmOptions::Yes);
    select_choose_input == ConfirmOptions::Yes
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KeyPairProperties {
    pub seed_phrase_hd_path: crate::types::slip10::BIP32Path,
    pub master_seed_phrase: String,
    pub implicit_account_id: unc_primitives::types::AccountId,
    #[serde(rename = "public_key")]
    pub public_key_str: String,
    #[serde(rename = "private_key")]
    pub secret_keypair_str: String,
}

pub fn get_key_pair_properties_from_seed_phrase(
    seed_phrase_hd_path: crate::types::slip10::BIP32Path,
    master_seed_phrase: String,
) -> color_eyre::eyre::Result<KeyPairProperties> {
    let master_seed = bip39::Mnemonic::parse(&master_seed_phrase)?.to_seed("");
    let derived_private_key = slip10::derive_key_from_path(
        &master_seed,
        slip10::Curve::Ed25519,
        &seed_phrase_hd_path.clone().into(),
    )
    .map_err(|err| {
        color_eyre::Report::msg(format!(
            "Failed to derive a key from the master key: {}",
            err
        ))
    })?;

    let secret_keypair = {
        let secret = ed25519_dalek::SecretKey::from_bytes(&derived_private_key.key)?;
        let public = ed25519_dalek::PublicKey::from(&secret);
        ed25519_dalek::Keypair { secret, public }
    };

    let implicit_account_id =
        unc_primitives::types::AccountId::try_from(hex::encode(secret_keypair.public))?;
    let public_key_str = format!(
        "ed25519:{}",
        bs58::encode(&secret_keypair.public).into_string()
    );
    let secret_keypair_str = format!(
        "ed25519:{}",
        bs58::encode(secret_keypair.to_bytes()).into_string()
    );
    let key_pair_properties: KeyPairProperties = KeyPairProperties {
        seed_phrase_hd_path,
        master_seed_phrase,
        implicit_account_id,
        public_key_str,
        secret_keypair_str,
    };
    Ok(key_pair_properties)
}

pub fn get_public_key_from_seed_phrase(
    seed_phrase_hd_path: slip10::BIP32Path,
    master_seed_phrase: &str,
) -> color_eyre::eyre::Result<unc_crypto::PublicKey> {
    let master_seed = bip39::Mnemonic::parse(master_seed_phrase)?.to_seed("");
    let derived_private_key =
        slip10::derive_key_from_path(&master_seed, slip10::Curve::Ed25519, &seed_phrase_hd_path)
            .map_err(|err| {
                color_eyre::Report::msg(format!(
                    "Failed to derive a key from the master key: {}",
                    err
                ))
            })?;
    let secret_keypair = {
        let secret = ed25519_dalek::SecretKey::from_bytes(&derived_private_key.key)?;
        let public = ed25519_dalek::PublicKey::from(&secret);
        ed25519_dalek::Keypair { secret, public }
    };
    let public_key_str = format!(
        "ed25519:{}",
        bs58::encode(&secret_keypair.public).into_string()
    );
    Ok(unc_crypto::PublicKey::from_str(&public_key_str)?)
}

pub fn generate_ed25519_keypair() -> color_eyre::eyre::Result<KeyPairProperties> {
    let generate_keypair: crate::utils_command::generate_keypair_subcommand::CliGenerateKeypair =
        crate::utils_command::generate_keypair_subcommand::CliGenerateKeypair::default();
    let (master_seed_phrase, master_seed) =
        if let Some(master_seed_phrase) = generate_keypair.master_seed_phrase.as_deref() {
            (
                master_seed_phrase.to_owned(),
                bip39::Mnemonic::parse(master_seed_phrase)?.to_seed(""),
            )
        } else {
            let mnemonic =
                bip39::Mnemonic::generate(generate_keypair.new_master_seed_phrase_words_count)?;
            let master_seed_phrase = mnemonic.word_iter().collect::<Vec<&str>>().join(" ");
            (master_seed_phrase, mnemonic.to_seed(""))
        };

    let derived_private_key = slip10::derive_key_from_path(
        &master_seed,
        slip10::Curve::Ed25519,
        &generate_keypair.seed_phrase_hd_path.clone().into(),
    )
    .map_err(|err| {
        color_eyre::Report::msg(format!(
            "Failed to derive a key from the master key: {}",
            err
        ))
    })?;

    let secret_keypair = {
        let secret = ed25519_dalek::SecretKey::from_bytes(&derived_private_key.key)?;
        let public = ed25519_dalek::PublicKey::from(&secret);
        ed25519_dalek::Keypair { secret, public }
    };

    let implicit_account_id =
        unc_primitives::types::AccountId::try_from(hex::encode(secret_keypair.public))?;
    let public_key_str = format!(
        "ed25519:{}",
        bs58::encode(&secret_keypair.public).into_string()
    );
    let secret_keypair_str = format!(
        "ed25519:{}",
        bs58::encode(secret_keypair.to_bytes()).into_string()
    );
    let key_pair_properties: KeyPairProperties = KeyPairProperties {
        seed_phrase_hd_path: generate_keypair.seed_phrase_hd_path,
        master_seed_phrase,
        implicit_account_id,
        public_key_str,
        secret_keypair_str,
    };
    Ok(key_pair_properties)
}


/// The length of an rsa `RsaPublicKey`, in bytes.
pub const RAW_PUBLIC_KEY_RSA_2048_LENGTH: usize = 294;

// FIXME: generate keypair use trait object
pub fn generate_rsa2048_keypair() -> color_eyre::eyre::Result<KeyPairProperties> {
    let generate_keypair: crate::utils_command::generate_keypair_subcommand::CliGenerateKeypair =
        crate::utils_command::generate_keypair_subcommand::CliGenerateKeypair::default();
    let (master_seed_phrase, _master_seed) =
        if let Some(master_seed_phrase) = generate_keypair.master_seed_phrase.as_deref() {
            (
                master_seed_phrase.to_owned(),
                bip39::Mnemonic::parse(master_seed_phrase)?.to_seed(""),
            )
        } else {
            let mnemonic =
                bip39::Mnemonic::generate(generate_keypair.new_master_seed_phrase_words_count)?;
            let master_seed_phrase = mnemonic.word_iter().collect::<Vec<&str>>().join(" ");
            (master_seed_phrase, mnemonic.to_seed(""))
        };

    let mut rng = rand::thread_rng();
    let bits = 2048;
    let priv_key = RsaPrivateKey::new(&mut rng, bits)?;
    let pub_key = RsaPublicKey::from(&priv_key);

    let implicit_account_id =
        unc_primitives::types::AccountId::try_from(format!("test{}", rng.gen_range(0..10000)))?;
    
    let der_pk_encoded = pub_key.to_public_key_der().unwrap();
    let public_key_str = format!(
        "rsa2048:{}",
        bs58::encode(&der_pk_encoded.as_bytes()).into_string()
    );

    let der_sk_encoded = priv_key.to_pkcs8_der().unwrap().to_bytes();
    let secret_keypair_str = format!(
        "rsa2048:{}",
        bs58::encode(der_sk_encoded.as_slice()).into_string()
    );
    let key_pair_properties: KeyPairProperties = KeyPairProperties {
        seed_phrase_hd_path: generate_keypair.seed_phrase_hd_path,
        master_seed_phrase,
        implicit_account_id,
        public_key_str,
        secret_keypair_str,
    };
    Ok(key_pair_properties)
}

pub fn print_full_signed_transaction(transaction: unc_primitives::transaction::SignedTransaction) {
    eprintln!("{:<25} {}\n", "signature:", transaction.signature);
    crate::common::print_full_unsigned_transaction(transaction.transaction);
}

pub fn print_full_unsigned_transaction(transaction: unc_primitives::transaction::Transaction) {
    eprintln!(
        "Unsigned transaction hash (Base58-encoded SHA-256 hash): {}\n\n",
        transaction.get_hash_and_size().0
    );

    eprintln!("{:<13} {}", "public_key:", &transaction.public_key);
    eprintln!("{:<13} {}", "nonce:", &transaction.nonce);
    eprintln!("{:<13} {}", "block_hash:", &transaction.block_hash);

    let prepopulated = crate::commands::PrepopulatedTransaction::from(transaction);
    print_unsigned_transaction(&prepopulated);
}

pub fn print_unsigned_transaction(transaction: &crate::commands::PrepopulatedTransaction) {
    eprintln!("{:<13} {}", "signer_id:", &transaction.signer_id);
    eprintln!("{:<13} {}", "receiver_id:", &transaction.receiver_id);
    if transaction
        .actions
        .iter()
        .any(|action| matches!(action, unc_primitives::transaction::Action::Delegate(_)))
    {
        eprintln!("signed delegate action:");
    } else {
        eprintln!("actions:");
    };

    for action in &transaction.actions {
        match action {
            unc_primitives::transaction::Action::CreateAccount(_) => {
                eprintln!(
                    "{:>5} {:<20} {}",
                    "--", "create account:", &transaction.receiver_id
                )
            }
            unc_primitives::transaction::Action::DeployContract(_) => {
                eprintln!("{:>5} {:<20}", "--", "deploy contract")
            }
            unc_primitives::transaction::Action::FunctionCall(function_call_action) => {
                eprintln!("{:>5} {:<20}", "--", "function call:");
                eprintln!(
                    "{:>18} {:<13} {}",
                    "", "method name:", &function_call_action.method_name
                );
                eprintln!(
                    "{:>18} {:<13} {}",
                    "",
                    "args:",
                    match serde_json::from_slice::<serde_json::Value>(&function_call_action.args) {
                        Ok(parsed_args) => {
                            serde_json::to_string_pretty(&parsed_args)
                                .unwrap_or_else(|_| "".to_string())
                                .replace('\n', "\n                                 ")
                        }
                        Err(_) => {
                            if let Ok(args) = String::from_utf8(function_call_action.args.clone()) {
                                args
                            } else {
                                format!(
                                    "<non-printable data ({})>",
                                    bytesize::ByteSize(function_call_action.args.len() as u64)
                                )
                            }
                        }
                    }
                );
                eprintln!(
                    "{:>18} {:<13} {}",
                    "",
                    "gas:",
                    crate::common::UncGas::from_gas(function_call_action.gas)
                );
                eprintln!(
                    "{:>18} {:<13} {}",
                    "",
                    "deposit:",
                    crate::types::unc_token::UncToken::from_yoctounc(
                        function_call_action.deposit
                    )
                );
            }
            unc_primitives::transaction::Action::Transfer(transfer_action) => {
                eprintln!(
                    "{:>5} {:<20} {}",
                    "--",
                    "transfer deposit:",
                    crate::types::unc_token::UncToken::from_yoctounc(transfer_action.deposit)
                );
            }
            unc_primitives::transaction::Action::Pledge(pledge_action) => {
                eprintln!("{:>5} {:<20}", "--", "pledge:");
                eprintln!(
                    "{:>18} {:<13} {}",
                    "", "public key:", &pledge_action.public_key
                );
                eprintln!(
                    "{:>18} {:<13} {}",
                    "",
                    "pledge:",
                    crate::types::unc_token::UncToken::from_yoctounc(pledge_action.pledge)
                );
            }
            unc_primitives::transaction::Action::AddKey(add_key_action) => {
                eprintln!("{:>5} {:<20}", "--", "add access key:");
                eprintln!(
                    "{:>18} {:<13} {}",
                    "", "public key:", &add_key_action.public_key
                );
                eprintln!(
                    "{:>18} {:<13} {}",
                    "", "nonce:", &add_key_action.access_key.nonce
                );
                eprintln!(
                    "{:>18} {:<13} {:?}",
                    "", "permission:", &add_key_action.access_key.permission
                );
            }
            unc_primitives::transaction::Action::DeleteKey(delete_key_action) => {
                eprintln!("{:>5} {:<20}", "--", "delete access key:");
                eprintln!(
                    "{:>18} {:<13} {}",
                    "", "public key:", &delete_key_action.public_key
                );
            }
            unc_primitives::transaction::Action::DeleteAccount(delete_account_action) => {
                eprintln!(
                    "{:>5} {:<20} {}",
                    "--", "delete account:", &transaction.receiver_id
                );
                eprintln!(
                    "{:>5} {:<20} {}",
                    "", "beneficiary id:", &delete_account_action.beneficiary_id
                );
            }
            unc_primitives::transaction::Action::Delegate(signed_delegate_action) => {
                let prepopulated_transaction = crate::commands::PrepopulatedTransaction {
                    signer_id: signed_delegate_action.delegate_action.sender_id.clone(),
                    receiver_id: signed_delegate_action.delegate_action.receiver_id.clone(),
                    actions: signed_delegate_action.delegate_action.get_actions(),
                };
                print_unsigned_transaction(&prepopulated_transaction);
            }
            unc_primitives::transaction::Action::RegisterRsa2048Keys(register_rsa2048_action) => {
                eprintln!("{:>5} {:<20}", "--", "register rsa2048 key:");
                eprintln!(
                    "{:>18} {:<13} {}",
                    "", "public key:", &register_rsa2048_action.public_key
                );
                eprintln!(
                    "{:>18} {:<13} {}",
                    "", "op type:", &register_rsa2048_action.operation_type
                );
            },
            unc_primitives::transaction::Action::CreateRsa2048Challenge(create_rsa2048keys_challenge_action) => {
                eprintln!(
                    "{:>18} {:<13} {}",
                    "", "public key:", &create_rsa2048keys_challenge_action.public_key
                );
                eprintln!(
                    "{:>18} {:<13} {}",
                    "",
                    "args:",
                    match serde_json::from_slice::<serde_json::Value>(&create_rsa2048keys_challenge_action.args) {
                        Ok(parsed_args) => {
                            serde_json::to_string_pretty(&parsed_args)
                                .unwrap_or_else(|_| "".to_string())
                                .replace('\n', "\n                                 ")
                        }
                        Err(_) => {
                            format!(
                                "<non-printable data ({})>",
                                bytesize::ByteSize(create_rsa2048keys_challenge_action.args.len() as u64)
                            )
                        }
                    }
                );
            },
        }
    }
}

fn print_value_successful_transaction(
    transaction_info: unc_primitives::views::FinalExecutionOutcomeView,
) {
    for action in transaction_info.transaction.actions {
        match action {
            unc_primitives::views::ActionView::CreateAccount => {
                eprintln!(
                    "New account <{}> has been successfully created.",
                    transaction_info.transaction.receiver_id,
                );
            }
            unc_primitives::views::ActionView::DeployContract { code: _ } => {
                eprintln!("Contract code has been successfully deployed.",);
            }
            unc_primitives::views::ActionView::FunctionCall {
                method_name,
                args: _,
                gas: _,
                deposit: _,
            } => {
                eprintln!(
                    "The \"{}\" call to <{}> on behalf of <{}> succeeded.",
                    method_name,
                    transaction_info.transaction.receiver_id,
                    transaction_info.transaction.signer_id,
                );
            }
            unc_primitives::views::ActionView::Transfer { deposit } => {
                eprintln!(
                    "<{}> has transferred {} to <{}> successfully.",
                    transaction_info.transaction.signer_id,
                    crate::types::unc_token::UncToken::from_yoctounc(deposit),
                    transaction_info.transaction.receiver_id,
                );
            }
            unc_primitives::views::ActionView::Pledge {
                pledge,
                public_key: _,
            } => {
                if pledge == 0 {
                    eprintln!(
                        "Validator <{}> successfully unpledged.",
                        transaction_info.transaction.signer_id,
                    );
                } else {
                    eprintln!(
                        "Validator <{}> has successfully pledged {}.",
                        transaction_info.transaction.signer_id,
                        crate::types::unc_token::UncToken::from_yoctounc(pledge),
                    );
                }
            }
            unc_primitives::views::ActionView::AddKey {
                public_key,
                access_key: _,
            } => {
                eprintln!(
                    "Added access key = {} to {}.",
                    public_key, transaction_info.transaction.receiver_id,
                );
            }
            unc_primitives::views::ActionView::DeleteKey { public_key } => {
                eprintln!(
                    "Access key <{}> for account <{}> has been successfully deleted.",
                    public_key, transaction_info.transaction.signer_id,
                );
            }
            unc_primitives::views::ActionView::DeleteAccount { beneficiary_id: _ } => {
                eprintln!(
                    "Account <{}> has been successfully deleted.",
                    transaction_info.transaction.signer_id,
                );
            }
            unc_primitives::views::ActionView::Delegate {
                delegate_action,
                signature: _,
            } => {
                eprintln!(
                    "Actions delegated for <{}> completed successfully.",
                    delegate_action.sender_id,
                );
            }
            unc_primitives::views::ActionView::RegisterRsa2048Keys { public_key, operation_type, args: _, } => {
                eprintln!(
                    "Rsa2048 key <{}>, op_type <{}> for account <{}> has been successfully registered.",
                    public_key, operation_type, transaction_info.transaction.signer_id,
                );
            },
            unc_primitives::views::ActionView::CreateRsa2048Challenge { public_key, challenge_key, args: _, } => {
                eprintln!(
                    "Rsa2048  <{}> with ChallengeKey <{}> for account <{}> has been successfully challenge created.",
                    public_key, challenge_key, transaction_info.transaction.signer_id,
                );
            },
        }
    }
}

pub fn rpc_transaction_error(
    err: unc_jsonrpc_client::errors::JsonRpcError<
        unc_jsonrpc_client::methods::broadcast_tx_commit::RpcTransactionError,
    >,
) -> CliResult {
    match &err {
        unc_jsonrpc_client::errors::JsonRpcError::TransportError(_rpc_transport_error) => {
            eprintln!("Transport error transaction.\nPlease wait. The next try to send this transaction is happening right now ...");
        }
        unc_jsonrpc_client::errors::JsonRpcError::ServerError(rpc_server_error) => match rpc_server_error {
            unc_jsonrpc_client::errors::JsonRpcServerError::HandlerError(rpc_transaction_error) => match rpc_transaction_error {
                unc_jsonrpc_client::methods::broadcast_tx_commit::RpcTransactionError::TimeoutError => {
                    eprintln!("Timeout error transaction.\nPlease wait. The next try to send this transaction is happening right now ...");
                }
                unc_jsonrpc_client::methods::broadcast_tx_commit::RpcTransactionError::InvalidTransaction { context } => {
                    return handler_invalid_tx_error(context);
                }
                unc_jsonrpc_client::methods::broadcast_tx_commit::RpcTransactionError::DoesNotTrackShard => {
                    return color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("RPC Server Error: {}", err));
                }
                unc_jsonrpc_client::methods::broadcast_tx_commit::RpcTransactionError::RequestRouted{transaction_hash} => {
                    return color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("RPC Server Error for transaction with hash {}\n{}", transaction_hash, err));
                }
                unc_jsonrpc_client::methods::broadcast_tx_commit::RpcTransactionError::UnknownTransaction{requested_transaction_hash} => {
                    return color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("RPC Server Error for transaction with hash {}\n{}", requested_transaction_hash, err));
                }
                unc_jsonrpc_client::methods::broadcast_tx_commit::RpcTransactionError::InternalError{debug_info} => {
                    return color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("RPC Server Error: {}", debug_info));
                }
            }
            unc_jsonrpc_client::errors::JsonRpcServerError::RequestValidationError(rpc_request_validation_error) => {
                return color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Incompatible request with the server: {:#?}",  rpc_request_validation_error));
            }
            unc_jsonrpc_client::errors::JsonRpcServerError::InternalError{ info } => {
                eprintln!("Internal server error: {}.\nPlease wait. The next try to send this transaction is happening right now ...", info.clone().unwrap_or_default());
            }
            unc_jsonrpc_client::errors::JsonRpcServerError::NonContextualError(rpc_error) => {
                return color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Unexpected response: {}", rpc_error));
            }
            unc_jsonrpc_client::errors::JsonRpcServerError::ResponseStatusError(json_rpc_server_response_status_error) => match json_rpc_server_response_status_error {
                unc_jsonrpc_client::errors::JsonRpcServerResponseStatusError::Unauthorized => {
                    return color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("JSON RPC server requires authentication. Please, authenticate unc CLI with the JSON RPC server you use."));
                }
                unc_jsonrpc_client::errors::JsonRpcServerResponseStatusError::TooManyRequests => {
                    eprintln!("JSON RPC server is currently busy.\nPlease wait. The next try to send this transaction is happening right now ...");
                }
                unc_jsonrpc_client::errors::JsonRpcServerResponseStatusError::Unexpected{status} => {
                    return color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("JSON RPC server responded with an unexpected status code: {}", status));
                }
            }
        }
    }
    Ok(())
}

pub fn rpc_async_transaction_error(
    _err: unc_jsonrpc_client::errors::JsonRpcError<
        unc_jsonrpc_client::methods::broadcast_tx_async::RpcBroadcastTxAsyncError,
    >,
) -> CliResult {
    Ok(())
}

pub fn print_action_error(action_error: &unc_primitives::errors::ActionError) -> crate::CliResult {
    match &action_error.kind {
        unc_primitives::errors::ActionErrorKind::AccountAlreadyExists { account_id } => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: Create Account action tries to create an account with account ID <{}> which already exists in the storage.", account_id))
        }
        unc_primitives::errors::ActionErrorKind::AccountDoesNotExist { account_id } => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!(
                "Error: TX receiver ID <{}> doesn't exist (but action is not \"Create Account\").",
                account_id
            ))
        }
        unc_primitives::errors::ActionErrorKind::CreateAccountOnlyByRegistrar {
            account_id: _,
            registrar_account_id: _,
            predecessor_id: _,
        } => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: A top-level account ID can only be created by registrar."))
        }
        unc_primitives::errors::ActionErrorKind::CreateAccountNotAllowed {
            account_id,
            predecessor_id,
        } => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: A newly created account <{}> must be under a namespace of the creator account <{}>.", account_id, predecessor_id))
        }
        unc_primitives::errors::ActionErrorKind::ActorNoPermission {
            account_id: _,
            actor_id: _,
        } => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: Administrative actions can be proceed only if sender=receiver or the first TX action is a \"Create Account\" action."))
        }
        unc_primitives::errors::ActionErrorKind::DeleteKeyDoesNotExist {
            account_id,
            public_key,
        } => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!(
                "Error: Account <{}>  tries to remove an access key <{}> that doesn't exist.",
                account_id, public_key
            ))
        }
        unc_primitives::errors::ActionErrorKind::AddKeyAlreadyExists {
            account_id,
            public_key,
        } => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!(
                "Error: Public key <{}> is already used for an existing account ID <{}>.",
                public_key, account_id
            ))
        }
        unc_primitives::errors::ActionErrorKind::DeleteAccountStaking { account_id } => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!(
                "Error: Account <{}> is pledging and can not be deleted",
                account_id
            ))
        }
        unc_primitives::errors::ActionErrorKind::LackBalanceForState { account_id, amount } => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: Receipt action can't be completed, because the remaining balance will not be enough to cover storage.\nAn account which needs balance: <{}>\nBalance required to complete the action: <{}>",
                account_id,
                crate::types::unc_token::UncToken::from_yoctounc(*amount)
            ))
        }
        unc_primitives::errors::ActionErrorKind::TriesToUnpledge { account_id } => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!(
                "Error: Account <{}> is not yet pledged, but tries to unpledge.",
                account_id
            ))
        }
        unc_primitives::errors::ActionErrorKind::TriesToPledge {
            account_id,
            pledge,
            pledging: _,
            balance,
        } => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!(
                "Error: Account <{}> doesn't have enough balance ({}) to increase the pledge ({}).",
                account_id,
                crate::types::unc_token::UncToken::from_yoctounc(*balance),
                crate::types::unc_token::UncToken::from_yoctounc(*pledge)
            ))
        }
        unc_primitives::errors::ActionErrorKind::InsufficientPledge {
            account_id: _,
            pledge,
            minimum_pledge,
        } => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!(
                "Error: Insufficient pledge {}.\nThe minimum rate must be {}.",
                crate::types::unc_token::UncToken::from_yoctounc(*pledge),
                crate::types::unc_token::UncToken::from_yoctounc(*minimum_pledge)
            ))
        }
        unc_primitives::errors::ActionErrorKind::FunctionCallError(function_call_error_ser) => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: An error occurred during a `FunctionCall` Action, parameter is debug message.\n{:?}", function_call_error_ser))
        }
        unc_primitives::errors::ActionErrorKind::NewReceiptValidationError(
            receipt_validation_error,
        ) => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: Error occurs when a new `ActionReceipt` created by the `FunctionCall` action fails.\n{:?}", receipt_validation_error))
        }
        unc_primitives::errors::ActionErrorKind::OnlyImplicitAccountCreationAllowed {
            account_id: _,
        } => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: `CreateAccount` action is called on hex-characters account of length 64.\nSee implicit account creation NEP: https://github.com/unc/NEPs/pull/71"))
        }
        unc_primitives::errors::ActionErrorKind::DeleteAccountWithLargeState { account_id } => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!(
                "Error: Delete account <{}> whose state is large is temporarily banned.",
                account_id
            ))
        }
        unc_primitives::errors::ActionErrorKind::DelegateActionInvalidSignature => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: Invalid Signature on DelegateAction"))
        }
        unc_primitives::errors::ActionErrorKind::DelegateActionSenderDoesNotMatchTxReceiver {
            sender_id,
            receiver_id,
        } => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: Delegate Action sender {sender_id} does not match transaction receiver {receiver_id}"))
        }
        unc_primitives::errors::ActionErrorKind::DelegateActionExpired => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: DelegateAction Expired"))
        }
        unc_primitives::errors::ActionErrorKind::DelegateActionAccessKeyError(_) => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: The given public key doesn't exist for the sender"))
        }
        unc_primitives::errors::ActionErrorKind::DelegateActionInvalidNonce {
            delegate_nonce,
            ak_nonce,
        } => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: DelegateAction Invalid Delegate Nonce: {delegate_nonce} ak_nonce: {ak_nonce}"))
        }
        unc_primitives::errors::ActionErrorKind::DelegateActionNonceTooLarge {
            delegate_nonce,
            upper_bound,
        } => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: DelegateAction Invalid Delegate Nonce: {delegate_nonce} upper bound: {upper_bound}"))
        }
        unc_primitives::errors::ActionErrorKind::RsaKeysNotFound { account_id, public_key } => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: RSA key not found for account <{}> and public key <{}>.", account_id, public_key))
        },
    }
}

pub fn handler_invalid_tx_error(
    invalid_tx_error: &unc_primitives::errors::InvalidTxError,
) -> crate::CliResult {
    match invalid_tx_error {
        unc_primitives::errors::InvalidTxError::InvalidAccessKeyError(invalid_access_key_error) => {
            match invalid_access_key_error {
                unc_primitives::errors::InvalidAccessKeyError::AccessKeyNotFound{account_id, public_key} => {
                    color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: Public key {} doesn't exist for the account <{}>.", public_key, account_id))
                },
                unc_primitives::errors::InvalidAccessKeyError::ReceiverMismatch{tx_receiver, ak_receiver} => {
                    color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: Transaction for <{}> doesn't match the access key for <{}>.", tx_receiver, ak_receiver))
                },
                unc_primitives::errors::InvalidAccessKeyError::MethodNameMismatch{method_name} => {
                    color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: Transaction method name <{}> isn't allowed by the access key.", method_name))
                },
                unc_primitives::errors::InvalidAccessKeyError::RequiresFullAccess => {
                    color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: Transaction requires a full permission access key."))
                },
                unc_primitives::errors::InvalidAccessKeyError::NotEnoughAllowance{account_id, public_key, allowance, cost} => {
                    color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: Access Key <{}> for account <{}> does not have enough allowance ({}) to cover transaction cost ({}).",
                        public_key,
                        account_id,
                        crate::types::unc_token::UncToken::from_yoctounc(*allowance),
                        crate::types::unc_token::UncToken::from_yoctounc(*cost)
                    ))
                },
                unc_primitives::errors::InvalidAccessKeyError::DepositWithFunctionCall => {
                    color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: Having a deposit with a function call action is not allowed with a function call access key."))
                }
            }
        },
        unc_primitives::errors::InvalidTxError::InvalidSignerId { signer_id } => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: TX signer ID <{}> is not in a valid format or does not satisfy requirements\nSee \"unc_runtime_utils::utils::is_valid_account_id\".", signer_id))
        },
        unc_primitives::errors::InvalidTxError::SignerDoesNotExist { signer_id } => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: TX signer ID <{}> is not found in the storage.", signer_id))
        },
        unc_primitives::errors::InvalidTxError::InvalidNonce { tx_nonce, ak_nonce } => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: Transaction nonce ({}) must be account[access_key].nonce ({}) + 1.", tx_nonce, ak_nonce))
        },
        unc_primitives::errors::InvalidTxError::NonceTooLarge { tx_nonce, upper_bound } => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: Transaction nonce ({}) is larger than the upper bound ({}) given by the block height.", tx_nonce, upper_bound))
        },
        unc_primitives::errors::InvalidTxError::InvalidReceiverId { receiver_id } => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: TX receiver ID ({}) is not in a valid format or does not satisfy requirements\nSee \"unc_runtime_utils::is_valid_account_id\".", receiver_id))
        },
        unc_primitives::errors::InvalidTxError::InvalidSignature => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: TX signature is not valid"))
        },
        unc_primitives::errors::InvalidTxError::NotEnoughBalance {signer_id, balance, cost} => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: Account <{}> does not have enough balance ({}) to cover TX cost ({}).",
                signer_id,
                crate::types::unc_token::UncToken::from_yoctounc(*balance),
                crate::types::unc_token::UncToken::from_yoctounc(*cost)
            ))
        },
        unc_primitives::errors::InvalidTxError::LackBalanceForState {signer_id, amount} => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: Signer account <{}> doesn't have enough balance ({}) after transaction.",
                signer_id,
                crate::types::unc_token::UncToken::from_yoctounc(*amount)
            ))
        },
        unc_primitives::errors::InvalidTxError::CostOverflow => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: An integer overflow occurred during transaction cost estimation."))
        },
        unc_primitives::errors::InvalidTxError::InvalidChain => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: Transaction parent block hash doesn't belong to the current chain."))
        },
        unc_primitives::errors::InvalidTxError::Expired => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: Transaction has expired."))
        },
        unc_primitives::errors::InvalidTxError::ActionsValidation(actions_validation_error) => {
            match actions_validation_error {
                unc_primitives::errors::ActionsValidationError::DeleteActionMustBeFinal => {
                    color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: The delete action must be the final action in transaction."))
                },
                unc_primitives::errors::ActionsValidationError::TotalPrepaidGasExceeded {total_prepaid_gas, limit} => {
                    color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: The total prepaid gas ({}) for all given actions exceeded the limit ({}).",
                    total_prepaid_gas,
                    limit
                    ))
                },
                unc_primitives::errors::ActionsValidationError::TotalNumberOfActionsExceeded {total_number_of_actions, limit} => {
                    color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: The number of actions ({}) exceeded the given limit ({}).", total_number_of_actions, limit))
                },
                unc_primitives::errors::ActionsValidationError::AddKeyMethodNamesNumberOfBytesExceeded {total_number_of_bytes, limit} => {
                    color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: The total number of bytes ({}) of the method names exceeded the limit ({}) in a Add Key action.", total_number_of_bytes, limit))
                },
                unc_primitives::errors::ActionsValidationError::AddKeyMethodNameLengthExceeded {length, limit} => {
                    color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: The length ({}) of some method name exceeded the limit ({}) in a Add Key action.", length, limit))
                },
                unc_primitives::errors::ActionsValidationError::IntegerOverflow => {
                    color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: Integer overflow."))
                },
                unc_primitives::errors::ActionsValidationError::InvalidAccountId {account_id} => {
                    color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: Invalid account ID <{}>.", account_id))
                },
                unc_primitives::errors::ActionsValidationError::ContractSizeExceeded {size, limit} => {
                    color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: The size ({}) of the contract code exceeded the limit ({}) in a DeployContract action.", size, limit))
                },
                unc_primitives::errors::ActionsValidationError::FunctionCallMethodNameLengthExceeded {length, limit} => {
                    color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: The length ({}) of the method name exceeded the limit ({}) in a Function Call action.", length, limit))
                },
                unc_primitives::errors::ActionsValidationError::FunctionCallArgumentsLengthExceeded {length, limit} => {
                    color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: The length ({}) of the arguments exceeded the limit ({}) in a Function Call action.", length, limit))
                },
                unc_primitives::errors::ActionsValidationError::UnsuitableStakingKey {public_key} => {
                    color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: An attempt to pledge with a public key <{}> that is not convertible to ristretto.", public_key))
                },
                unc_primitives::errors::ActionsValidationError::FunctionCallZeroAttachedGas => {
                    color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: The attached amount of gas in a FunctionCall action has to be a positive number."))
                }
                unc_primitives::errors::ActionsValidationError::DelegateActionMustBeOnlyOne => {
                    color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: DelegateActionMustBeOnlyOne"))
                }
                unc_primitives::errors::ActionsValidationError::UnsupportedProtocolFeature { protocol_feature, version } => {
                    color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: Protocol Feature {} is unsupported in version {}", protocol_feature, version))
                }
            }
        },
        unc_primitives::errors::InvalidTxError::TransactionSizeExceeded { size, limit } => {
            color_eyre::eyre::Result::Err(color_eyre::eyre::eyre!("Error: The size ({}) of serialized transaction exceeded the limit ({}).", size, limit))
        }
    }
}

pub fn print_transaction_error(
    tx_execution_error: &unc_primitives::errors::TxExecutionError,
) -> crate::CliResult {
    eprintln!("Failed transaction");
    match tx_execution_error {
        unc_primitives::errors::TxExecutionError::ActionError(action_error) => {
            print_action_error(action_error)
        }
        unc_primitives::errors::TxExecutionError::InvalidTxError(invalid_tx_error) => {
            handler_invalid_tx_error(invalid_tx_error)
        }
    }
}

pub fn print_transaction_status(
    transaction_info: &unc_primitives::views::FinalExecutionOutcomeView,
    network_config: &crate::config::NetworkConfig,
) -> crate::CliResult {
    eprintln!("--- Logs ---------------------------");
    for receipt in transaction_info.receipts_outcome.iter() {
        if receipt.outcome.logs.is_empty() {
            eprintln!("Logs [{}]:   No logs", receipt.outcome.executor_id);
        } else {
            eprintln!("Logs [{}]:", receipt.outcome.executor_id);
            eprintln!("  {}", receipt.outcome.logs.join("\n  "));
        };
    }
    match &transaction_info.status {
        unc_primitives::views::FinalExecutionStatus::NotStarted
        | unc_primitives::views::FinalExecutionStatus::Started => unreachable!(),
        unc_primitives::views::FinalExecutionStatus::Failure(tx_execution_error) => {
            return print_transaction_error(tx_execution_error);
        }
        unc_primitives::views::FinalExecutionStatus::SuccessValue(bytes_result) => {
            eprintln!("--- Result -------------------------");
            if bytes_result.is_empty() {
                eprintln!("Empty result");
            } else if let Ok(json_result) =
                serde_json::from_slice::<serde_json::Value>(bytes_result)
            {
                println!("{}", serde_json::to_string_pretty(&json_result)?);
            } else if let Ok(string_result) = String::from_utf8(bytes_result.clone()) {
                println!("{string_result}");
            } else {
                eprintln!("The returned value is not printable (binary data)");
            }
            eprintln!("------------------------------------\n");
            print_value_successful_transaction(transaction_info.clone())
        }
    };
    eprintln!("Transaction ID: {id}\nTo see the transaction in the transaction explorer, please open this url in your browser:\n{path}{id}\n",
        id=transaction_info.transaction_outcome.id,
        path=network_config.explorer_transaction_url
    );
    Ok(())
}

pub fn print_async_transaction_status(
    tx_hash: &CryptoHash,
    network_config: &crate::config::NetworkConfig,
) -> crate::CliResult {
    eprintln!("--- Logs ---------------------------");
    eprintln!("Transaction ID: {id}\nTo see the transaction in the transaction explorer, please open this url in your browser:\n{path}{id}\n",
        id=tx_hash,
        path=network_config.explorer_transaction_url
    );
    Ok(())
}

pub fn save_access_key_to_keychain(
    network_config: crate::config::NetworkConfig,
    key_pair_properties_buf: &str,
    public_key_str: &str,
    account_id: &str,
) -> color_eyre::eyre::Result<String> {
    let service_name = std::borrow::Cow::Owned(format!(
        "unc-{}-{}",
        network_config.network_name, account_id
    ));

    keyring::Entry::new(&service_name, &format!("{}:{}", account_id, public_key_str))
        .wrap_err("Failed to open keychain")?
        .set_password(key_pair_properties_buf)
        .wrap_err("Failed to save password to keychain")?;

    Ok("The data for the access key is saved in the keychain".to_string())
}

pub fn save_access_key_to_legacy_keychain(
    network_config: crate::config::NetworkConfig,
    credentials_home_dir: std::path::PathBuf,
    key_pair_properties_buf: &str,
    public_key_str: &str,
    account_id: &str,
) -> color_eyre::eyre::Result<String> {
    let dir_name = network_config.network_name.as_str();
    let file_with_key_name: std::path::PathBuf =
        format!("{}.json", public_key_str.replace(':', "_")).into();
    let mut path_with_key_name = std::path::PathBuf::from(&credentials_home_dir);
    path_with_key_name.push(dir_name);
    path_with_key_name.push(account_id);
    std::fs::create_dir_all(&path_with_key_name)?;
    path_with_key_name.push(file_with_key_name);
    let message_1 = if path_with_key_name.exists() {
        format!(
            "The file: {} already exists! Therefore it was not overwritten.",
            &path_with_key_name.display()
        )
    } else {
        std::fs::File::create(&path_with_key_name)
            .wrap_err_with(|| format!("Failed to create file: {:?}", path_with_key_name))?
            .write(key_pair_properties_buf.as_bytes())
            .wrap_err_with(|| format!("Failed to write to file: {:?}", path_with_key_name))?;
        format!(
            "The data for the access key is saved in a file {}",
            &path_with_key_name.display()
        )
    };

    let file_with_account_name: std::path::PathBuf = format!("{}.json", account_id).into();
    let mut path_with_account_name = std::path::PathBuf::from(&credentials_home_dir);
    path_with_account_name.push(dir_name);
    path_with_account_name.push(file_with_account_name);
    if path_with_account_name.exists() {
        Ok(format!(
            "{}\nThe file: {} already exists! Therefore it was not overwritten.",
            message_1,
            &path_with_account_name.display()
        ))
    } else {
        std::fs::File::create(&path_with_account_name)
            .wrap_err_with(|| format!("Failed to create file: {:?}", path_with_account_name))?
            .write(key_pair_properties_buf.as_bytes())
            .wrap_err_with(|| format!("Failed to write to file: {:?}", path_with_account_name))?;
        Ok(format!(
            "{}\nThe data for the access key is saved in a file {}",
            message_1,
            &path_with_account_name.display()
        ))
    }
}

pub fn get_config_toml() -> color_eyre::eyre::Result<crate::config::Config> {
    if let Some(mut path_config_toml) = dirs::config_dir() {
        path_config_toml.extend(&["unc-cli", "config.toml"]);

        if !path_config_toml.is_file() {
            write_config_toml(crate::config::Config::default())?;
        };
        let config_toml = std::fs::read_to_string(&path_config_toml)?;
        toml::from_str(&config_toml).or_else(|err| {
            eprintln!("Warning: `unc` CLI configuration file stored at {path_config_toml:?} could not be parsed due to: {err}");
            eprintln!("Note: The default configuration printed below will be used instead:\n");
            let default_config = crate::config::Config::default();
            eprintln!("{}", toml::to_string(&default_config)?);
            Ok(default_config)
        })
    } else {
        Ok(crate::config::Config::default())
    }
}
pub fn write_config_toml(config: crate::config::Config) -> CliResult {
    let config_toml = toml::to_string(&config)?;
    let mut path_config_toml = dirs::config_dir().wrap_err("Impossible to get your config dir!")?;
    path_config_toml.push("unc-cli");
    std::fs::create_dir_all(&path_config_toml)?;
    path_config_toml.push("config.toml");
    std::fs::File::create(&path_config_toml)
        .wrap_err_with(|| format!("Failed to create file: {path_config_toml:?}"))?
        .write(config_toml.as_bytes())
        .wrap_err_with(|| format!("Failed to write to file: {path_config_toml:?}"))?;
    eprintln!("Note: `unc` CLI configuration is stored in {path_config_toml:?}");
    Ok(())
}

pub fn try_external_subcommand_execution(error: clap::Error) -> CliResult {
    let (subcommand, args) = {
        let mut args = std::env::args().skip(1);
        let subcommand = args
            .next()
            .ok_or_else(|| color_eyre::eyre::eyre!("subcommand is not provided"))?;
        (subcommand, args.collect::<Vec<String>>())
    };
    let is_top_level_command_known = crate::commands::TopLevelCommandDiscriminants::iter()
        .map(|x| format!("{:?}", &x).to_lowercase())
        .any(|x| x == subcommand);
    if is_top_level_command_known {
        error.exit()
    }
    let subcommand_exe = format!("unc-{}{}", subcommand, std::env::consts::EXE_SUFFIX);

    let path = path_directories()
        .iter()
        .map(|dir| dir.join(&subcommand_exe))
        .find(|file| is_executable(file));

    let command = path.ok_or_else(|| {
        color_eyre::eyre::eyre!(
            "{} command or {} extension does not exist",
            subcommand,
            subcommand_exe
        )
    })?;

    let err = match cargo_util::ProcessBuilder::new(command)
        .args(&args)
        .exec_replace()
    {
        Ok(()) => return Ok(()),
        Err(e) => e,
    };

    if let Some(perr) = err.downcast_ref::<cargo_util::ProcessError>() {
        if let Some(code) = perr.code {
            return Err(color_eyre::eyre::eyre!("perror occurred, code: {}", code));
        }
    }
    Err(color_eyre::eyre::eyre!(err))
}

fn is_executable<P: AsRef<std::path::Path>>(path: P) -> bool {
    #[cfg(target_family = "unix")]
    {
        use std::os::unix::prelude::*;
        std::fs::metadata(path)
            .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(target_family = "windows")]
    path.as_ref().is_file()
}

fn path_directories() -> Vec<std::path::PathBuf> {
    if let Some(val) = std::env::var_os("PATH") {
        std::env::split_paths(&val).collect()
    } else {
        Vec::new()
    }
}

pub fn get_delegated_validator_list_from_mainnet(
    network_connection: &linked_hash_map::LinkedHashMap<String, crate::config::NetworkConfig>,
) -> color_eyre::eyre::Result<std::collections::BTreeSet<unc_primitives::types::AccountId>> {
    let network_config = network_connection
        .get("mainnet")
        .wrap_err("There is no 'mainnet' network in your configuration.")?;

    let epoch_validator_info = network_config
        .json_rpc_client()
        .blocking_call(
            &unc_jsonrpc_client::methods::validators::RpcValidatorRequest {
                epoch_reference: unc_primitives::types::EpochReference::Latest,
            },
        )
        .wrap_err("Failed to get epoch validators information request.")?;

    Ok(epoch_validator_info
        .current_pledge_proposals
        .into_iter()
        .map(|current_proposal| current_proposal.take_account_id())
        .chain(
            epoch_validator_info
                .current_validators
                .into_iter()
                .map(|current_validator| current_validator.account_id),
        )
        .chain(
            epoch_validator_info
                .next_validators
                .into_iter()
                .map(|next_validator| next_validator.account_id),
        )
        .collect())
}

pub fn get_used_delegated_validator_list(
    config: &crate::config::Config,
) -> color_eyre::eyre::Result<VecDeque<unc_primitives::types::AccountId>> {
    let used_account_list: VecDeque<UsedAccount> =
        get_used_account_list(&config.credentials_home_dir);
    let mut delegated_validator_list =
        get_delegated_validator_list_from_mainnet(&config.network_connection)?;
    let mut used_delegated_validator_list: VecDeque<unc_primitives::types::AccountId> =
        VecDeque::new();

    for used_account in used_account_list {
        if delegated_validator_list.remove(&used_account.account_id) {
            used_delegated_validator_list.push_back(used_account.account_id);
        }
    }

    used_delegated_validator_list.extend(delegated_validator_list);
    Ok(used_delegated_validator_list)
}

pub fn input_pledging_pool_validator_account_id(
    config: &crate::config::Config,
) -> color_eyre::eyre::Result<Option<crate::types::account_id::AccountId>> {
    let used_delegated_validator_list = get_used_delegated_validator_list(config)?
        .into_iter()
        .map(String::from)
        .collect::<Vec<_>>();
    let validator_account_id_str = match Text::new("What is delegated validator account ID?")
        .with_autocomplete(move |val: &str| {
            Ok(used_delegated_validator_list
                .iter()
                .filter(|s| s.contains(val))
                .cloned()
                .collect())
        })
        .with_validator(|account_id_str: &str| {
            match unc_primitives::types::AccountId::validate(account_id_str) {
                Ok(_) => Ok(inquire::validator::Validation::Valid),
                Err(err) => Ok(inquire::validator::Validation::Invalid(
                    inquire::validator::ErrorMessage::Custom(format!("Invalid account ID: {err}")),
                )),
            }
        })
        .prompt()
    {
        Ok(value) => value,
        Err(
            inquire::error::InquireError::OperationCanceled
            | inquire::error::InquireError::OperationInterrupted,
        ) => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    let validator_account_id =
        crate::types::account_id::AccountId::from_str(&validator_account_id_str)?;
    update_used_account_list_as_non_signer(
        &config.credentials_home_dir,
        validator_account_id.as_ref(),
    );
    Ok(Some(validator_account_id))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StakingPoolInfo {
    pub validator_id: unc_primitives::types::AccountId,
    pub fee: Option<RewardFeeFraction>,
    pub delegators: Option<u64>,
    pub pledge: unc_primitives::types::Balance,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct RewardFeeFraction {
    pub numerator: u32,
    pub denominator: u32,
}

pub fn get_validator_list(
    network_config: &crate::config::NetworkConfig,
) -> color_eyre::eyre::Result<Vec<StakingPoolInfo>> {
    let json_rpc_client = network_config.json_rpc_client();

    let validators_pledge = get_validators_pledge(&json_rpc_client)?;

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let concurrency = 10;

    let mut validator_list = runtime.block_on(
        futures::stream::iter(validators_pledge.iter())
            .map(|(validator_account_id, pledge)| async {
                get_pledging_pool_info(
                    &json_rpc_client.clone(),
                    validator_account_id.clone(),
                    *pledge,
                )
                .await
            })
            .buffer_unordered(concurrency)
            .try_collect::<Vec<_>>(),
    )?;
    validator_list.sort_by(|a, b| b.pledge.cmp(&a.pledge));
    Ok(validator_list)
}

pub fn get_validators_pledge(
    json_rpc_client: &unc_jsonrpc_client::JsonRpcClient,
) -> color_eyre::eyre::Result<
    std::collections::HashMap<unc_primitives::types::AccountId, unc_primitives::types::Balance>,
> {
    let epoch_validator_info = json_rpc_client
        .blocking_call(
            &unc_jsonrpc_client::methods::validators::RpcValidatorRequest {
                epoch_reference: unc_primitives::types::EpochReference::Latest,
            },
        )
        .wrap_err("Failed to get epoch validators information request.")?;

    Ok(epoch_validator_info
        .current_pledge_proposals
        .into_iter()
        .map(|validator_pledge_view| {
            let validator_pledge = validator_pledge_view.into_validator_pledge();
            validator_pledge.account_and_pledge()
        })
        .chain(epoch_validator_info.current_validators.into_iter().map(
            |current_epoch_validator_info| {
                (
                    current_epoch_validator_info.account_id,
                    current_epoch_validator_info.pledge,
                )
            },
        ))
        .chain(
            epoch_validator_info
                .next_validators
                .into_iter()
                .map(|next_epoch_validator_info| {
                    (
                        next_epoch_validator_info.account_id,
                        next_epoch_validator_info.pledge,
                    )
                }),
        )
        .collect())
}

async fn get_pledging_pool_info(
    json_rpc_client: &unc_jsonrpc_client::JsonRpcClient,
    validator_account_id: unc_primitives::types::AccountId,
    pledge: u128,
) -> color_eyre::Result<StakingPoolInfo> {
    let fee = match json_rpc_client
        .call(unc_jsonrpc_client::methods::query::RpcQueryRequest {
            block_reference: unc_primitives::types::Finality::Final.into(),
            request: unc_primitives::views::QueryRequest::CallFunction {
                account_id: validator_account_id.clone(),
                method_name: "get_reward_fee_fraction".to_string(),
                args: unc_primitives::types::FunctionArgs::from(vec![]),
            },
        })
        .await
    {
        Ok(response) => Some(
            response
                .call_result()?
                .parse_result_from_json::<RewardFeeFraction>()
                .wrap_err(
                    "Failed to parse return value of view function call for RewardFeeFraction.",
                )?,
        ),
        Err(unc_jsonrpc_client::errors::JsonRpcError::ServerError(
            unc_jsonrpc_client::errors::JsonRpcServerError::HandlerError(
                unc_jsonrpc_client::methods::query::RpcQueryError::NoContractCode { .. }
                | unc_jsonrpc_client::methods::query::RpcQueryError::ContractExecutionError {
                    ..
                },
            ),
        )) => None,
        Err(err) => return Err(err.into()),
    };

    let delegators = match json_rpc_client
        .call(unc_jsonrpc_client::methods::query::RpcQueryRequest {
            block_reference: unc_primitives::types::Finality::Final.into(),
            request: unc_primitives::views::QueryRequest::CallFunction {
                account_id: validator_account_id.clone(),
                method_name: "get_number_of_accounts".to_string(),
                args: unc_primitives::types::FunctionArgs::from(vec![]),
            },
        })
        .await
    {
        Ok(response) => Some(
            response
                .call_result()?
                .parse_result_from_json::<u64>()
                .wrap_err("Failed to parse return value of view function call for u64.")?,
        ),
        Err(unc_jsonrpc_client::errors::JsonRpcError::ServerError(
            unc_jsonrpc_client::errors::JsonRpcServerError::HandlerError(
                unc_jsonrpc_client::methods::query::RpcQueryError::NoContractCode { .. }
                | unc_jsonrpc_client::methods::query::RpcQueryError::ContractExecutionError {
                    ..
                },
            ),
        )) => None,
        Err(err) => return Err(err.into()),
    };

    Ok(StakingPoolInfo {
        validator_id: validator_account_id.clone(),
        fee,
        delegators,
        pledge,
    })
}

pub fn display_account_info(
    viewed_at_block_hash: &CryptoHash,
    viewed_at_block_height: &unc_primitives::types::BlockHeight,
    account_id: &unc_primitives::types::AccountId,
    delegated_pledge: &std::collections::BTreeMap<
        unc_primitives::types::AccountId,
        unc_token::UncToken,
    >,
    account_view: &unc_primitives::views::AccountView,
    access_keys: &[unc_primitives::views::AccessKeyInfoView],
) {
    let mut table: Table = Table::new();
    table.set_format(*prettytable::format::consts::FORMAT_NO_COLSEP);

    profile_table(
        viewed_at_block_hash,
        viewed_at_block_height,
        account_id,
        &mut table,
    );

    table.add_row(prettytable::row![
        Fg->"Native account balance",
        Fy->unc_token::UncToken::from_yoctounc(account_view.amount)
    ]);
    table.add_row(prettytable::row![
        Fg->"Validator pledge",
        Fy->unc_token::UncToken::from_yoctounc(account_view.pledging)
    ]);

    for (validator_id, pledge) in delegated_pledge {
        table.add_row(prettytable::row![
            Fg->format!("Delegated pledge with <{validator_id}>"),
            Fy->pledge
        ]);
    }

    table.add_row(prettytable::row![
        Fg->"Storage used by the account",
        Fy->bytesize::ByteSize(account_view.storage_usage),
    ]);

    let contract_status = if account_view.code_hash == CryptoHash::default() {
        "No contract code".to_string()
    } else {
        hex::encode(account_view.code_hash.as_ref())
    };
    table.add_row(prettytable::row![
        Fg->"Contract (SHA-256 checksum hex)",
        Fy->contract_status
    ]);

    let access_keys_summary = if access_keys.is_empty() {
        "Account is locked (no access keys)".to_string()
    } else {
        let full_access_keys_count = access_keys
            .iter()
            .filter(|access_key| {
                matches!(
                    access_key.access_key.permission,
                    unc_primitives::views::AccessKeyPermissionView::FullAccess
                )
            })
            .count();
        format!(
            "{} full access keys and {} function-call-only access keys",
            full_access_keys_count,
            access_keys.len() - full_access_keys_count
        )
    };
    table.add_row(prettytable::row![
        Fg->"Access keys",
        Fy->access_keys_summary
    ]);
    table.printstd();
}

pub fn display_account_profile(
    viewed_at_block_hash: &CryptoHash,
    viewed_at_block_height: &unc_primitives::types::BlockHeight,
    account_id: &unc_primitives::types::AccountId,
) {
    let mut table = Table::new();
    table.set_format(*prettytable::format::consts::FORMAT_NO_COLSEP);
    profile_table(
        viewed_at_block_hash,
        viewed_at_block_height,
        account_id,
        &mut table,
    );
    table.printstd();
}

fn profile_table(
    viewed_at_block_hash: &CryptoHash,
    viewed_at_block_height: &unc_primitives::types::BlockHeight,
    account_id: &unc_primitives::types::AccountId,
    table: &mut Table,
) {
    table.add_row(prettytable::row![
        Fy->account_id,
        format!("At block #{}\n({})", viewed_at_block_height, viewed_at_block_hash)
    ]);
}

pub fn display_access_key_list(access_keys: &[unc_primitives::views::AccessKeyInfoView]) {
    let mut table = Table::new();
    table.set_titles(prettytable::row![Fg=>"#", "Public Key", "Nonce", "Permissions"]);

    for (index, access_key) in access_keys.iter().enumerate() {
        let permissions_message = match &access_key.access_key.permission {
            AccessKeyPermissionView::FullAccess => "full access".to_owned(),
            AccessKeyPermissionView::FunctionCall {
                allowance,
                receiver_id,
                method_names,
            } => {
                let allowance_message = match allowance {
                    Some(amount) => format!(
                        "with an allowance of {}",
                        unc_token::UncToken::from_yoctounc(*amount)
                    ),
                    None => "with no limit".to_string(),
                };
                if method_names.is_empty() {
                    format!(
                        "do any function calls on {} {}",
                        receiver_id, allowance_message
                    )
                } else {
                    format!(
                        "only do {:?} function calls on {} {}",
                        method_names, receiver_id, allowance_message
                    )
                }
            }
        };

        table.add_row(prettytable::row![
            Fg->index + 1,
            access_key.public_key,
            access_key.access_key.nonce,
            permissions_message
        ]);
    }

    table.set_format(*prettytable::format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.printstd();
}

/// Interactive prompt for network name.
///
/// If account_ids is provided, show the network connections that are more
/// relevant at the top of the list.
pub fn input_network_name(
    config: &crate::config::Config,
    account_ids: &[unc_primitives::types::AccountId],
) -> color_eyre::eyre::Result<Option<String>> {
    if config.network_connection.len() == 1 {
        return Ok(config.network_names().pop());
    }
    let variants = if !account_ids.is_empty() {
        let (mut matches, non_matches): (Vec<_>, Vec<_>) = config
            .network_connection
            .iter()
            .partition(|(_, network_config)| {
                // We use `linkdrop_account_id` as a heuristic to determine if
                // the accounts are on the same network. In the future, we
                // might consider to have a better way to do this.
                network_config
                    .linkdrop_account_id
                    .as_ref()
                    .map_or(false, |linkdrop_account_id| {
                        account_ids.iter().any(|account_id| {
                            account_id.as_str().ends_with(linkdrop_account_id.as_str())
                        })
                    })
            });
        let variants = if matches.is_empty() {
            non_matches
        } else {
            matches.extend(non_matches);
            matches
        };
        variants.into_iter().map(|(k, _)| k).collect()
    } else {
        config.network_connection.keys().collect()
    };

    let select_submit = Select::new("What is the name of the network?", variants).prompt();
    match select_submit {
        Ok(value) => Ok(Some(value.clone())),
        Err(
            inquire::error::InquireError::OperationCanceled
            | inquire::error::InquireError::OperationInterrupted,
        ) => Ok(None),
        Err(err) => Err(err.into()),
    }
}

#[easy_ext::ext(JsonRpcClientExt)]
pub impl unc_jsonrpc_client::JsonRpcClient {
    fn blocking_call<M>(
        &self,
        method: M,
    ) -> unc_jsonrpc_client::MethodCallResult<M::Response, M::Error>
    where
        M: unc_jsonrpc_client::methods::RpcMethod,
    {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(self.call(method))
    }

    /// A helper function to make a view-funcation call using JSON encoding for the function
    /// arguments and function return value.
    fn blocking_call_view_function(
        &self,
        account_id: &unc_primitives::types::AccountId,
        method_name: &str,
        args: Vec<u8>,
        block_reference: unc_primitives::types::BlockReference,
    ) -> Result<unc_primitives::views::CallResult, color_eyre::eyre::Error> {
        let query_view_method_response = self
            .blocking_call(unc_jsonrpc_client::methods::query::RpcQueryRequest {
                block_reference,
                request: unc_primitives::views::QueryRequest::CallFunction {
                    account_id: account_id.clone(),
                    method_name: method_name.to_owned(),
                    args: unc_primitives::types::FunctionArgs::from(args),
                },
            })
            .wrap_err("Failed to make a view-function call")?;
        query_view_method_response.call_result()
    }

    fn blocking_call_view_access_key(
        &self,
        account_id: &unc_primitives::types::AccountId,
        public_key: &unc_crypto::PublicKey,
        block_reference: unc_primitives::types::BlockReference,
    ) -> Result<
        unc_jsonrpc_primitives::types::query::RpcQueryResponse,
        unc_jsonrpc_client::errors::JsonRpcError<
            unc_jsonrpc_primitives::types::query::RpcQueryError,
        >,
    > {
        self.blocking_call(unc_jsonrpc_client::methods::query::RpcQueryRequest {
            block_reference,
            request: unc_primitives::views::QueryRequest::ViewAccessKey {
                account_id: account_id.clone(),
                public_key: public_key.clone(),
            },
        })
    }

    fn blocking_call_view_access_key_list(
        &self,
        account_id: &unc_primitives::types::AccountId,
        block_reference: unc_primitives::types::BlockReference,
    ) -> Result<
        unc_jsonrpc_primitives::types::query::RpcQueryResponse,
        unc_jsonrpc_client::errors::JsonRpcError<
            unc_jsonrpc_primitives::types::query::RpcQueryError,
        >,
    > {
        self.blocking_call(unc_jsonrpc_client::methods::query::RpcQueryRequest {
            block_reference,
            request: unc_primitives::views::QueryRequest::ViewAccessKeyList {
                account_id: account_id.clone(),
            },
        })
    }

    fn blocking_call_view_account(
        &self,
        account_id: &unc_primitives::types::AccountId,
        block_reference: unc_primitives::types::BlockReference,
    ) -> Result<
        unc_jsonrpc_primitives::types::query::RpcQueryResponse,
        unc_jsonrpc_client::errors::JsonRpcError<
            unc_jsonrpc_primitives::types::query::RpcQueryError,
        >,
    > {
        self.blocking_call(unc_jsonrpc_client::methods::query::RpcQueryRequest {
            block_reference,
            request: unc_primitives::views::QueryRequest::ViewAccount {
                account_id: account_id.clone(),
            },
        })
    }
}

#[easy_ext::ext(RpcQueryResponseExt)]
pub impl unc_jsonrpc_primitives::types::query::RpcQueryResponse {
    fn access_key_view(&self) -> color_eyre::eyre::Result<unc_primitives::views::AccessKeyView> {
        if let unc_jsonrpc_primitives::types::query::QueryResponseKind::AccessKey(
            access_key_view,
        ) = &self.kind
        {
            Ok(access_key_view.clone())
        } else {
            color_eyre::eyre::bail!(
                "Internal error: Received unexpected query kind in response to a View Access Key query call",
            );
        }
    }

    fn access_key_list_view(
        &self,
    ) -> color_eyre::eyre::Result<unc_primitives::views::AccessKeyList> {
        if let unc_jsonrpc_primitives::types::query::QueryResponseKind::AccessKeyList(
            access_key_list,
        ) = &self.kind
        {
            Ok(access_key_list.clone())
        } else {
            color_eyre::eyre::bail!(
                "Internal error: Received unexpected query kind in response to a View Access Key List query call",
            );
        }
    }

    fn account_view(&self) -> color_eyre::eyre::Result<unc_primitives::views::AccountView> {
        if let unc_jsonrpc_primitives::types::query::QueryResponseKind::ViewAccount(account_view) =
            &self.kind
        {
            Ok(account_view.clone())
        } else {
            color_eyre::eyre::bail!(
                "Internal error: Received unexpected query kind in response to a View Account query call",
            );
        }
    }

    fn call_result(&self) -> color_eyre::eyre::Result<unc_primitives::views::CallResult> {
        if let unc_jsonrpc_primitives::types::query::QueryResponseKind::CallResult(result) =
            &self.kind
        {
            Ok(result.clone())
        } else {
            color_eyre::eyre::bail!(
                "Internal error: Received unexpected query kind in response to a view-function query call",
            );
        }
    }
}

#[easy_ext::ext(CallResultExt)]
pub impl unc_primitives::views::CallResult {
    fn parse_result_from_json<T>(&self) -> Result<T, color_eyre::eyre::Error>
    where
        T: for<'de> serde::Deserialize<'de>,
    {
        serde_json::from_slice(&self.result).wrap_err_with(|| {
            format!(
                "Failed to parse view-function call return value: {}",
                String::from_utf8_lossy(&self.result)
            )
        })
    }

    fn print_logs(&self) {
        eprintln!("--------------");
        if self.logs.is_empty() {
            eprintln!("No logs")
        } else {
            eprintln!("Logs:");
            eprintln!("  {}", self.logs.join("\n  "));
        }
        eprintln!("--------------");
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct UsedAccount {
    pub account_id: unc_primitives::types::AccountId,
    pub used_as_signer: bool,
}

fn get_used_account_list_path(credentials_home_dir: &std::path::Path) -> std::path::PathBuf {
    credentials_home_dir.join("accounts.json")
}

pub fn create_used_account_list_from_keychain(
    credentials_home_dir: &std::path::Path,
) -> color_eyre::eyre::Result<()> {
    let mut used_account_list: std::collections::BTreeSet<unc_primitives::types::AccountId> =
        std::collections::BTreeSet::new();
    let read_dir =
        |dir: &std::path::Path| dir.read_dir().map(Iterator::flatten).into_iter().flatten();
    for network_connection_dir in read_dir(credentials_home_dir) {
        for entry in read_dir(&network_connection_dir.path()) {
            match (entry.path().file_stem(), entry.path().extension()) {
                (Some(file_stem), Some(extension)) if extension == "json" => {
                    if let Ok(account_id) = file_stem.to_string_lossy().parse() {
                        used_account_list.insert(account_id);
                    }
                }
                _ if entry.path().is_dir() => {
                    if let Ok(account_id) = entry.file_name().to_string_lossy().parse() {
                        used_account_list.insert(account_id);
                    }
                }
                _ => {}
            }
        }
    }

    if !used_account_list.is_empty() {
        let used_account_list_path = get_used_account_list_path(credentials_home_dir);
        let used_account_list_buf = serde_json::to_string(
            &used_account_list
                .into_iter()
                .map(|account_id| UsedAccount {
                    account_id,
                    used_as_signer: true,
                })
                .collect::<Vec<_>>(),
        )?;
        std::fs::write(&used_account_list_path, used_account_list_buf).wrap_err_with(|| {
            format!(
                "Failed to write to file: {}",
                used_account_list_path.display()
            )
        })?;
    }
    Ok(())
}

pub fn update_used_account_list_as_signer(
    credentials_home_dir: &std::path::Path,
    account_id: &unc_primitives::types::AccountId,
) {
    let account_is_signer = true;
    update_used_account_list(credentials_home_dir, account_id, account_is_signer);
}

pub fn update_used_account_list_as_non_signer(
    credentials_home_dir: &std::path::Path,
    account_id: &unc_primitives::types::AccountId,
) {
    let account_is_signer = false;
    update_used_account_list(credentials_home_dir, account_id, account_is_signer);
}

fn update_used_account_list(
    credentials_home_dir: &std::path::Path,
    account_id: &unc_primitives::types::AccountId,
    account_is_signer: bool,
) {
    let mut used_account_list = get_used_account_list(credentials_home_dir);

    let used_account = if let Some(mut used_account) = used_account_list
        .iter()
        .position(|used_account| &used_account.account_id == account_id)
        .and_then(|position| used_account_list.remove(position))
    {
        used_account.used_as_signer |= account_is_signer;
        used_account
    } else {
        UsedAccount {
            account_id: account_id.clone(),
            used_as_signer: account_is_signer,
        }
    };
    used_account_list.push_front(used_account);

    let used_account_list_path = get_used_account_list_path(credentials_home_dir);
    if let Ok(used_account_list_buf) = serde_json::to_string(&used_account_list) {
        let _ = std::fs::write(used_account_list_path, used_account_list_buf);
    }
}

pub fn get_used_account_list(credentials_home_dir: &std::path::Path) -> VecDeque<UsedAccount> {
    let used_account_list_path = get_used_account_list_path(credentials_home_dir);
    serde_json::from_str(
        std::fs::read_to_string(used_account_list_path)
            .as_deref()
            .unwrap_or("[]"),
    )
    .unwrap_or_default()
}

pub fn is_used_account_list_exist(credentials_home_dir: &std::path::Path) -> bool {
    get_used_account_list_path(credentials_home_dir).exists()
}

pub fn input_signer_account_id_from_used_account_list(
    credentials_home_dir: &std::path::Path,
    message: &str,
) -> color_eyre::eyre::Result<Option<crate::types::account_id::AccountId>> {
    let account_is_signer = true;
    input_account_id_from_used_account_list(credentials_home_dir, message, account_is_signer)
}

pub fn input_non_signer_account_id_from_used_account_list(
    credentials_home_dir: &std::path::Path,
    message: &str,
) -> color_eyre::eyre::Result<Option<crate::types::account_id::AccountId>> {
    let account_is_signer = false;
    input_account_id_from_used_account_list(credentials_home_dir, message, account_is_signer)
}

fn input_account_id_from_used_account_list(
    credentials_home_dir: &std::path::Path,
    message: &str,
    account_is_signer: bool,
) -> color_eyre::eyre::Result<Option<crate::types::account_id::AccountId>> {
    let used_account_list = get_used_account_list(credentials_home_dir)
        .into_iter()
        .filter(|account| !account_is_signer || account.used_as_signer)
        .map(|account| account.account_id.to_string())
        .collect::<Vec<_>>();
    let account_id_str = match Text::new(message)
        .with_autocomplete(move |val: &str| {
            Ok(used_account_list
                .iter()
                .filter(|s| s.contains(val))
                .cloned()
                .collect())
        })
        .with_validator(|account_id_str: &str| {
            match unc_primitives::types::AccountId::validate(account_id_str) {
                Ok(_) => Ok(inquire::validator::Validation::Valid),
                Err(err) => Ok(inquire::validator::Validation::Invalid(
                    inquire::validator::ErrorMessage::Custom(format!("Invalid account ID: {err}")),
                )),
            }
        })
        .prompt()
    {
        Ok(value) => value,
        Err(
            inquire::error::InquireError::OperationCanceled
            | inquire::error::InquireError::OperationInterrupted,
        ) => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    let account_id = crate::types::account_id::AccountId::from_str(&account_id_str)?;
    update_used_account_list(credentials_home_dir, account_id.as_ref(), account_is_signer);
    Ok(Some(account_id))
}
