use color_eyre::eyre::Context;

pub mod constructor_mode;

#[derive(Debug, Clone, interactive_clap::InteractiveClap)]
#[interactive_clap(input_context = crate::GlobalContext)]
#[interactive_clap(output_context = CreateChallengeRsaContext)]
pub struct CreateChallengeRsaCommand {
    #[interactive_clap(skip_default_input_arg)]
    /// What is the miner account ID?
    account_id: crate::types::account_id::AccountId,
    #[interactive_clap(named_arg)]
    /// Specify a path to pem file
    use_file: PemFile,
}

#[derive(Debug, Clone)]
pub struct CreateChallengeRsaContext {
    global_context: crate::GlobalContext,
    receiver_account_id: near_primitives::types::AccountId,
    signer_account_id: near_primitives::types::AccountId,
}

impl CreateChallengeRsaContext {
    pub fn from_previous_context(
        previous_context: crate::GlobalContext,
        scope: &<CreateChallengeRsaCommand as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        Ok(Self {
            global_context: previous_context,
            receiver_account_id: scope.account_id.clone().into(),
            signer_account_id: scope.account_id.clone().into(),
        })
    }
}

impl CreateChallengeRsaCommand {
    pub fn input_account_id(
        context: &crate::GlobalContext,
    ) -> color_eyre::eyre::Result<Option<crate::types::account_id::AccountId>> {
        crate::common::input_signer_account_id_from_used_account_list(
            &context.config.credentials_home_dir,
            "What is the miner account ID?",
        )
    }
}

#[derive(Debug, Clone, interactive_clap_derive::InteractiveClap)]
#[interactive_clap(input_context = CreateChallengeRsaContext)]
#[interactive_clap(output_context = PemFileContext)]
pub struct PemFile {
    /// What is a file location of the pem?
    pub file_path: crate::types::path_buf::PathBuf,
    #[interactive_clap(subcommand)]
    initialize: self::constructor_mode::InitializeMode,
}

#[derive(Debug, Clone)]
pub struct PemFileContext {
    pub global_context: crate::GlobalContext,
    pub receiver_account_id: near_primitives::types::AccountId,
    pub signer_account_id: near_primitives::types::AccountId,
    pub public_key: near_crypto::PublicKey,
    pub challenge_key: near_crypto::PublicKey,
    pub private_key: String, // aes encrypted only read from system keychain
}

impl PemFileContext {
    pub fn from_previous_context(
        previous_context: CreateChallengeRsaContext,
        scope: &<PemFile as interactive_clap::ToInteractiveClapContextScope>::InteractiveClapContextScope,
    ) -> color_eyre::eyre::Result<Self> {
        let data = std::fs::read_to_string(&scope.file_path).wrap_err_with(|| {
            format!("Failed to open or read the file: {:?}.", &scope.file_path.0,)
        })?;
        let rsa_json: super::Rsa2048KeyPair = serde_json::from_str(&data)
        .wrap_err_with(|| format!("Error json codec reading data from file: {:?}", &scope.file_path.0))?;

        Ok(Self {
            global_context: previous_context.global_context,
            receiver_account_id: previous_context.receiver_account_id,
            signer_account_id: previous_context.signer_account_id,
            challenge_key: rsa_json.challenge_key,
            public_key: rsa_json.public_key,
            private_key: rsa_json.secret_key,
        })
    }
}
