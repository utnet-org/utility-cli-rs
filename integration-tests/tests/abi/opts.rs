use unc_integration_tests::generate_abi_fn_with;
use function_name::named;
use std::fs;

#[test]
#[named]
fn test_abi_no_doc() -> unc::CliResult {
    let abi_root = generate_abi_fn_with! {
        Opts: "--no-doc";
        Code:
        /// Adds `a` and `b`.
        pub fn add(&self, a: u32, b: u32) -> u32 {
            a + b
        }
    };

    assert_eq!(abi_root.body.functions.len(), 2);
    let function = &abi_root.body.functions[0];
    assert!(function.doc.is_none());

    Ok(())
}

#[test]
#[named]
fn test_abi_opt_compact_abi() -> unc::CliResult {
    generate_abi_fn_with! {
        Opts: "--compact-abi";
        Code:
        pub fn add(&self, a: u32, b: u32) -> u32 {
            a + b
        }
    };

    let manifest_dir: std::path::PathBuf = env!("CARGO_MANIFEST_DIR").into();
    let workspace_dir = manifest_dir
        .parent()
        .unwrap()
        .join("target")
        .join("_abi-integration-tests");
    let abi_json = fs::read_to_string(
        workspace_dir
            .join("target")
            .join("unc")
            .join(format!("{}_abi.json", function_name!())),
    )?;

    assert_eq!(minifier::json::minify(&abi_json).to_string(), abi_json);

    Ok(())
}

#[test]
#[named]
fn test_abi_opt_out_dir() -> unc::CliResult {
    let out_dir = tempfile::tempdir()?;
    let abi_root = generate_abi_fn_with! {
        Opts: format!("--out-dir {}", out_dir.path().display());
        Code:
        pub fn add(&self, a: u32, b: u32) -> u32 {
            a + b
        }
    };

    let abi_json = fs::read(
        out_dir
            .path()
            .join(format!("{}_abi.json", function_name!())),
    )?;
    assert_eq!(abi_root, serde_json::from_slice(&abi_json)?);

    Ok(())
}
