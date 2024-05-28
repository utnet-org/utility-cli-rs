use unc_integration_tests::{from_git, generate_abi_fn_with, generate_abi_with};
use function_name::named;
use git2::build::CheckoutBuilder;
use git2::Repository;
use std::collections::HashMap;
use tempfile::TempDir;

use crate::util::AsJsonSchema;

fn clone_git_repo() -> color_eyre::eyre::Result<TempDir> {
    let temp_dir = tempfile::tempdir()?;
    let repo_dir = temp_dir.path();
    let repo = Repository::clone(from_git::SDK_REPO, repo_dir)?;
    let commit = repo.revparse_single(from_git::SDK_REVISION)?;
    repo.checkout_tree(&commit, Some(&mut CheckoutBuilder::new()))?;

    Ok(temp_dir)
}

#[test]
#[named]
fn test_dependency_local_path() -> unc::CliResult {
    let unc_sdk_dir = clone_git_repo()?;
    let unc_sdk_dep_path = unc_sdk_dir.path().join("unc-sdk");

    // unc-sdk = { path = "::path::", features = ["abi"] }
    let abi_root = generate_abi_fn_with! {
        Cargo: "/templates/sdk-dependency/_Cargo_local_path.toml";
        Vars: HashMap::from([("path", unc_sdk_dep_path.to_str().unwrap())]);
        Code:
        pub fn foo(&self, a: bool, b: u32) {}
    };

    assert_eq!(abi_root.body.functions.len(), 2);
    let function = &abi_root.body.functions[1];
    let params = function.params.json_schemas()?;
    assert_eq!(params.len(), 2);

    Ok(())
}

#[test]
#[named]
fn test_dependency_local_path_with_version() -> unc::CliResult {
    let unc_sdk_dir = clone_git_repo()?;
    let unc_sdk_dep_path = unc_sdk_dir.path().join("unc-sdk");

    let abi_root = generate_abi_fn_with! {
        Cargo: "/templates/sdk-dependency/_Cargo_local_path_with_version.toml";
        Vars: HashMap::from([("path", unc_sdk_dep_path.to_str().unwrap())]);
        Code:
        pub fn foo(&self, a: bool, b: u32) {}
    };

    assert_eq!(abi_root.body.functions.len(), 2);
    let function = &abi_root.body.functions[1];
    let params = function.params.json_schemas()?;
    assert_eq!(params.len(), 2);

    Ok(())
}

#[test]
#[named]
fn test_dependency_default_features() -> unc::CliResult {
    let abi_root = generate_abi_fn_with! {
        Cargo: "/templates/_Cargo.toml";
        Code:
        pub fn foo(&self, a: bool, b: u32) {}
    };

    assert_eq!(abi_root.body.functions.len(), 2);
    let function = &abi_root.body.functions[1];
    let params = function.params.json_schemas()?;
    assert_eq!(params.len(), 2);

    Ok(())
}

#[test]
#[named]
fn test_dependency_explicit() -> unc::CliResult {
    let abi_root = generate_abi_fn_with! {
        Cargo: "/templates/sdk-dependency/_Cargo_explicit.toml";
        Code:
        pub fn foo(&self, a: bool, b: u32) {}
    };

    assert_eq!(abi_root.body.functions.len(), 2);
    let function = &abi_root.body.functions[1];
    let params = function.params.json_schemas()?;
    assert_eq!(params.len(), 2);

    Ok(())
}

#[test]
#[named]
fn test_dependency_no_default_features() -> unc::CliResult {
    let abi_root = generate_abi_fn_with! {
        Cargo: "/templates/sdk-dependency/_Cargo_no_default_features.toml";
        Code:
        pub fn foo(&self, a: bool, b: u32) {}
    };

    assert_eq!(abi_root.body.functions.len(), 2);
    let function = &abi_root.body.functions[1];
    let params = function.params.json_schemas()?;
    assert_eq!(params.len(), 2);

    Ok(())
}

#[test]
#[named]
fn test_dependency_multiple_features() -> unc::CliResult {
    let abi_root = generate_abi_fn_with! {
        Cargo: "/templates/sdk-dependency/_Cargo_multiple_features.toml";
        Code:
        pub fn foo(&self, a: bool, b: u32) {}
    };

    assert_eq!(abi_root.body.functions.len(), 2);
    let function = &abi_root.body.functions[1];
    let params = function.params.json_schemas()?;
    assert_eq!(params.len(), 2);

    Ok(())
}

#[test]
#[named]
fn test_dependency_platform_specific() -> unc::CliResult {
    let abi_root = generate_abi_fn_with! {
        Cargo: "/templates/sdk-dependency/_Cargo_platform_specific.toml";
        Code:
        pub fn foo(&self, a: bool, b: u32) {}
    };

    assert_eq!(abi_root.body.functions.len(), 2);
    let function = &abi_root.body.functions[1];
    let params = function.params.json_schemas()?;
    assert_eq!(params.len(), 2);

    Ok(())
}

// Does not work because of UNC SDK (generates code that depends on `unc-sdk` being the package name).
#[ignore]
#[test]
#[named]
fn test_dependency_renamed() -> unc::CliResult {
    let abi_root = generate_abi_with! {
        Cargo: "/templates/sdk-dependency/_Cargo_renamed.toml";
        Code:
        use unc_sdk::borsh::{BorshDeserialize, BorshSerialize};
        use unc::unc_bindgen;

        #[unc_bindgen]
        #[derive(Default, BorshDeserialize, BorshSerialize)]
        #[borsh(crate = "unc_sdk::borsh")]
        pub struct Contract {}

        #[unc_bindgen]
        impl Contract {
            pub fn foo(&self, a: bool, b: u32) {}
        }
    };

    assert_eq!(abi_root.body.functions.len(), 1);
    let function = &abi_root.body.functions[0];
    let params = function.params.json_schemas()?;
    assert_eq!(params.len(), 2);

    Ok(())
}

#[test]
#[named]
fn test_dependency_patch() -> unc::CliResult {
    // [dependencies]
    // unc-sdk = "2.0.3"
    //
    // [patch.crates-io]
    // unc-sdk = { git = "https://github.com/utnet-org/utility-sdk-rs.git", rev = "10b0dea3b1a214d789cc90314aa814a4181610ad" }
    let abi_root = generate_abi_fn_with! {
        Cargo: "/templates/sdk-dependency/_Cargo_patch.toml";
        Code:
        pub fn foo(&self, a: bool, b: u32) {}
    };

    assert_eq!(abi_root.body.functions.len(), 2);
    let function = &abi_root.body.functions[1];
    let params = function.params.json_schemas()?;
    assert_eq!(params.len(), 2);

    Ok(())
}

/// this is a test of Cargo.toml format
/// TODO: un-ignore when `2.x.x` unc-sdk is published
/// and `unc_integration_tests::SDK_VERSION` is changed 1.x.x -> 2.x.x
#[test]
#[ignore]
#[named]
fn test_abi_not_a_table() -> unc::CliResult {
    let abi_root = generate_abi_fn_with! {
        Cargo: "/templates/sdk-dependency/_Cargo_not_a_table.toml";
        Code:
        pub fn foo(&self, a: u32, b: u32) {}
    };

    assert_eq!(abi_root.body.functions.len(), 1);
    let function = &abi_root.body.functions[0];
    let params = function.params.json_schemas()?;
    assert_eq!(params.len(), 2);

    Ok(())
}
