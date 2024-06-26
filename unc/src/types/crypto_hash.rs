#[derive(Debug, Copy, Clone)]
pub struct CryptoHash(pub unc_primitives::hash::CryptoHash);

impl From<CryptoHash> for unc_primitives::hash::CryptoHash {
    fn from(item: CryptoHash) -> Self {
        item.0
    }
}

impl std::fmt::Display for CryptoHash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for CryptoHash {
    type Err = color_eyre::eyre::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let crypto_hash =
            unc_primitives::hash::CryptoHash::from_str(s).map_err(color_eyre::eyre::Report::msg)?;
        Ok(Self(crypto_hash))
    }
}

impl interactive_clap::ToCli for CryptoHash {
    type CliVariant = CryptoHash;
}
