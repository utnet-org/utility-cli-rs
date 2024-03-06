const ONE_UNC: u128 = 10u128.pow(24);

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    serde::Serialize,
    serde::Deserialize,
    derive_more::AsRef,
    derive_more::From,
    derive_more::Into,
    derive_more::FromStr,
)]
#[as_ref(forward)]
pub struct UncToken(pub unc_token::UncToken);

impl std::fmt::Display for UncToken {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.as_yoctounc() == 0 {
            write!(f, "0 unc")
        } else if self.as_yoctounc() <= 1_000 {
            write!(f, "{} yoctounc", self.as_yoctounc())
        } else if self.as_yoctounc() % ONE_UNC == 0 {
            write!(f, "{} unc", self.as_yoctounc() / ONE_UNC,)
        } else {
            write!(
                f,
                "{}.{} unc",
                self.as_yoctounc() / ONE_UNC,
                format!("{:0>24}", (self.as_yoctounc() % ONE_UNC)).trim_end_matches('0')
            )
        }
    }
}

impl UncToken {
    pub fn as_yoctounc(&self) -> u128 {
        self.0.as_yoctounc()
    }

    pub fn from_yoctounc(inner: u128) -> Self {
        Self(unc_token::UncToken::from_yoctounc(inner))
    }
}

impl interactive_clap::ToCli for UncToken {
    type CliVariant = UncToken;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn unc_token_to_string_0_unc() {
        assert_eq!(
            UncToken(unc_token::UncToken::from_unc(0)).to_string(),
            "0 unc".to_string()
        )
    }
    #[test]
    fn unc_token_to_string_0_milliunc() {
        assert_eq!(
            UncToken(unc_token::UncToken::from_milliunc(0)).to_string(),
            "0 unc".to_string()
        )
    }
    #[test]
    fn unc_token_to_string_0_yoctounc() {
        assert_eq!(
            UncToken(unc_token::UncToken::from_yoctounc(0)).to_string(),
            "0 unc".to_string()
        )
    }

    #[test]
    fn unc_token_to_string_0dot02_unc() {
        assert_eq!(
            UncToken(unc_token::UncToken::from_yoctounc(
                20_000_000_000_000_000_000_000
            ))
            .to_string(),
            "0.02 unc".to_string()
        )
    }
    #[test]
    fn unc_token_to_string_0dot00001230045600789_unc() {
        assert_eq!(
            UncToken(
                unc_token::UncToken::from_str("0.000012300456007890000000 unc")
                    .unwrap_or_default()
            )
            .to_string(),
            "0.00001230045600789 unc".to_string()
        )
    }
    #[test]
    fn unc_token_to_string_10_unc() {
        assert_eq!(
            UncToken(unc_token::UncToken::from_yoctounc(
                10_000_000_000_000_000_000_000_000
            ))
            .to_string(),
            "10 unc".to_string()
        )
    }
    #[test]
    fn unc_token_to_string_10dot02_000_01unc() {
        assert_eq!(
            UncToken(unc_token::UncToken::from_yoctounc(
                10_020_000_000_000_000_000_000_001
            ))
            .to_string(),
            "10.020000000000000000000001 unc".to_string()
        )
    }
    #[test]
    fn unc_token_to_string_1_yocto_unc() {
        assert_eq!(
            UncToken(unc_token::UncToken::from_yoctounc(1)).to_string(),
            "1 yoctounc".to_string()
        )
    }
    #[test]
    fn unc_token_to_string_100_yocto_unc() {
        assert_eq!(
            UncToken(unc_token::UncToken::from_yoctounc(100)).to_string(),
            "100 yoctounc".to_string()
        )
    }
}
