#[derive(Debug, Clone)]
pub struct Signature(pub unc_crypto::Signature);

impl std::fmt::Display for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::str::FromStr for Signature {
    type Err = unc_crypto::ParseSignatureError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let signature = unc_crypto::Signature::from_str(s)?;
        Ok(Self(signature))
    }
}

impl From<Signature> for unc_crypto::Signature {
    fn from(item: Signature) -> Self {
        item.0
    }
}
