use color_eyre::eyre::Context;

use super::Miner;

pub mod constructor_mode;

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = crate::GlobalContext)]
#[interactive_clap(output_context = RegisterRsaKeysContext)]
pub struct RegisterRsaKeysCommand {
    #[interactive_clap(skip_default_input_arg)]
    /// What is the CA(root/treasury) account ID?
    account_id: crate::types::account_id::AccountId,
    #[interactive_clap(named_arg)]
    /// Specify a path to pem file
    use_file: PemFile,
}

#[derive(Debug, Clone)]
pub struct RegisterRsaKeysContext {
    global_context: crate::GlobalContext,
    receiver_account_id: unc_primitives::types::AccountId,
    signer_account_id: unc_primitives::types::AccountId,
}

impl RegisterRsaKeysContext {
    pub fn from_previous_context(
        previous_context: crate::GlobalContext,
        scope: &<RegisterRsaKeysCommand as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        Ok(Self {
            global_context: previous_context,
            receiver_account_id: scope.account_id.clone().into(),
            signer_account_id: scope.account_id.clone().into(),
        })
    }
}

impl RegisterRsaKeysCommand {
    pub fn input_account_id(
        context: &crate::GlobalContext,
    ) -> color_eyre::eyre::Result<Option<crate::types::account_id::AccountId>> {
        crate::common::input_signer_account_id_from_used_account_list(
            &context.config.credentials_home_dir,
            "What is the CA(root/treasury) account ID?",
        )
    }
}

#[derive(Debug, Clone, interactive_clap_derive::InteractiveClap)]
#[interactive_clap(input_context = RegisterRsaKeysContext)]
#[interactive_clap(output_context = RsaFileContext)]
pub struct PemFile {
    /// What is a file location of the pem?
    pub file_path: crate::types::path_buf::PathBuf,
    #[interactive_clap(subcommand)]
    constructor: self::constructor_mode::ConstructorMode,
}

#[derive(Debug, Clone)]
pub struct RsaFileContext {
    pub global_context: crate::GlobalContext,
    pub receiver_account_id: unc_primitives::types::AccountId,
    pub signer_account_id: unc_primitives::types::AccountId,
    pub miners: Vec<Miner>,
}

impl RsaFileContext {
    pub fn from_previous_context(
        previous_context: RegisterRsaKeysContext,
        scope: &<PemFile as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        let data = std::fs::read_to_string(&scope.file_path).wrap_err_with(|| {
            format!("Failed to open or read the file: {:?}.", &scope.file_path.0,)
        })?;
        let miner_json: Vec<super::Miner> = serde_json::from_str(&data).wrap_err_with(|| {
            format!(
                "Error json codec reading data from file: {:?}",
                &scope.file_path.0
            )
        })?;

        Ok(Self {
            global_context: previous_context.global_context,
            receiver_account_id: previous_context.receiver_account_id,
            signer_account_id: previous_context.signer_account_id,
            miners: miner_json,
        })
    }
}
