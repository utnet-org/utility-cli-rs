use serde::{Deserialize, Serialize};

/// The contract source metadata is a standard interface that allows auditing and viewing source code for a deployed smart contract.
/// (https://github.com/unc/utility-sdk-rs/blob/master/unc-contract-standards/src/contract_metadata.rs)
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct ContractSourceMetadata {
    pub version: Option<String>,
    pub link: Option<String>,
    pub standards: Vec<Standard>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct Standard {
    pub standard: String,
    pub version: String,
}
