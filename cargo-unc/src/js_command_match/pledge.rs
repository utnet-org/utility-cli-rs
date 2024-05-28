#[derive(Debug, Clone, clap::Parser)]
/// This is a legacy `pledge` command. Once you run it with the specified arguments, new syntax command will be suggested.
pub struct PledgeArgs {
    account_id: String,
    pledging_key: String,
    amount: String,
    #[clap(allow_hyphen_values = true, num_args = 0..)]
    _unknown_args: Vec<String>,
}

impl PledgeArgs {
    pub fn to_cli_args(&self, network_config: String) -> Vec<String> {
        vec![
            "validator".to_owned(),
            "pledging".to_owned(),
            "pledge-proposal".to_owned(),
            self.account_id.to_owned(),
            self.pledging_key.to_owned(),
            format!("{} unc", self.amount),
            "network-config".to_owned(),
            network_config,
            "sign-with-keychain".to_owned(),
            "send".to_owned(),
        ]
    }
}
