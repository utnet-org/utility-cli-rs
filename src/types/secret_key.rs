#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretKey(pub unc_crypto::SecretKey);

impl std::fmt::Display for SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for SecretKey {
    type Err = unc_crypto::ParseKeyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let private_key = unc_crypto::SecretKey::from_str(s)?;
        Ok(Self(private_key))
    }
}

impl From<SecretKey> for unc_crypto::SecretKey {
    fn from(item: SecretKey) -> Self {
        item.0
    }
}

impl interactive_clap::ToCli for SecretKey {
    type CliVariant = SecretKey;
}
