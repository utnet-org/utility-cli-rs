#[derive(Debug, Clone, PartialEq, PartialOrd, Ord, Eq)]
pub struct PublicKey(pub unc_crypto::PublicKey);

impl std::fmt::Display for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for PublicKey {
    type Err = unc_crypto::ParseKeyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let public_key = unc_crypto::PublicKey::from_str(s)?;
        Ok(Self(public_key))
    }
}

impl From<PublicKey> for unc_crypto::PublicKey {
    fn from(item: PublicKey) -> Self {
        item.0
    }
}

impl From<unc_crypto::PublicKey> for PublicKey {
    fn from(item: unc_crypto::PublicKey) -> Self {
        Self(item)
    }
}

impl interactive_clap::ToCli for PublicKey {
    type CliVariant = PublicKey;
}
