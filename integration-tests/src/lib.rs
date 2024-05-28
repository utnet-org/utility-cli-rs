/// NOTE: `unc-sdk` version, published on crates.io
pub mod from_crates_io {
    use const_format::formatcp;

    pub const SDK_VERSION: &str = "2.0.2";
    pub const SDK_VERSION_TOML: &str = formatcp!(r#"version = "{SDK_VERSION}""#);
}

/// NOTE: this version is version of unc-sdk in arbitrary revision from N.x.x development cycle
pub mod from_git {
    use const_format::formatcp;

    pub const SDK_VERSION: &str = "2.0.3";
    pub const SDK_REVISION: &str = "3ca93ea104519b944823d564169eb1b24903a67f";
    pub const SDK_SHORT_VERSION_TOML: &str = formatcp!(r#"version = "{SDK_VERSION}""#);
    pub const SDK_REPO: &str = "https://github.com/utnet-org/utility-sdk-rs.git";
    pub const SDK_VERSION_TOML: &str =
        formatcp!(r#"version = "{SDK_VERSION}", git = "{SDK_REPO}", rev = "{SDK_REVISION}""#);
    pub const SDK_VERSION_TOML_TABLE: &str = formatcp!(
        r#"
        version = "{SDK_VERSION}"
        git = "https://github.com/utnet-org/utility-sdk-rs.git"
        rev = "{SDK_REVISION}"
        "#
    );
}

#[macro_export]
macro_rules! invoke_unc {
    ($(Cargo: $cargo_path:expr;)? $(Vars: $cargo_vars:expr;)? Opts: $cli_opts:expr; Code: $($code:tt)*) => {{
        let manifest_dir: camino::Utf8PathBuf = env!("CARGO_MANIFEST_DIR").into();
        let workspace_dir = manifest_dir.parent().unwrap().join("target").join("_abi-integration-tests");
        let crate_dir = workspace_dir.join(function_name!());
        let src_dir = crate_dir.join("src");
        std::fs::create_dir_all(&src_dir)?;

        let mut cargo_toml = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/templates/_Cargo.toml")).to_string();
        $(cargo_toml = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), $cargo_path)).to_string())?;
        let mut cargo_vars = std::collections::HashMap::new();
        $(cargo_vars = $cargo_vars)?;
        cargo_vars.insert("sdk-cratesio-version", $crate::from_crates_io::SDK_VERSION);
        cargo_vars.insert("sdk-cratesio-version-toml", $crate::from_crates_io::SDK_VERSION_TOML);
        cargo_vars.insert("sdk-git-version", $crate::from_git::SDK_VERSION);
        cargo_vars.insert("sdk-git-short-version-toml", $crate::from_git::SDK_SHORT_VERSION_TOML);
        cargo_vars.insert("sdk-git-version-toml", $crate::from_git::SDK_VERSION_TOML);
        cargo_vars.insert("sdk-git-version-toml-table", $crate::from_git::SDK_VERSION_TOML_TABLE);
        cargo_vars.insert("name", function_name!());
        for (k, v) in cargo_vars {
            cargo_toml = cargo_toml.replace(&format!("::{}::", k), v);
        }
        let cargo_path = crate_dir.join("Cargo.toml");
        std::fs::write(&cargo_path, cargo_toml)?;

        let lib_rs_file = syn::parse_file(&quote::quote! { $($code)* }.to_string()).unwrap();
        let lib_rs = prettyplease::unparse(&lib_rs_file);
        let lib_rs_path = src_dir.join("lib.rs");
        std::fs::write(lib_rs_path, lib_rs)?;

        std::env::set_var("CARGO_TARGET_DIR", workspace_dir.join("target"));

        let cmd = unc::commands::dev_tools::DevCommandsType::try_parse_from($cli_opts);

        match cmd {
            Ok(unc::commands::dev_tools::CliDevCommandsType::Abi(cmd)) => {
                let args = unc::commands::dev_tools::abi_command::AbiCommand {
                    no_doc: cmd.no_doc,
                    compact_abi: cmd.compact_abi,
                    out_dir: cmd.out_dir,
                    manifest_path: Some(cargo_path.into()),
                    color: cmd.color,
                };
                unc::commands::dev_tools::abi_command::abi::run(args)?;
            },
            Ok(unc::commands::dev_tools::CliDevCommandsType::Build(cmd)) => {
                let args = unc::commands::dev_tools::build_command::BuildCommand {
                    no_release: cmd.no_release,
                    no_abi: cmd.no_abi,
                    no_embed_abi: cmd.no_embed_abi,
                    no_doc: cmd.no_doc,
                    out_dir: cmd.out_dir,
                    manifest_path: Some(cargo_path.into()),
                    features: cmd.features,
                    no_default_features: cmd.no_default_features,
                    color: cmd.color,
                };
                unc::commands::dev_tools::build_command::build::run(args)?;
            },
            Ok(_) => todo!(),
            Err(_) => ()
        }

        workspace_dir.join("target").join("unc")
    }};
}

#[macro_export]
macro_rules! generate_abi_with {
    ($(Cargo: $cargo_path:expr;)? $(Vars: $cargo_vars:expr;)? $(Opts: $cli_opts:expr;)? Code: $($code:tt)*) => {{
        let opts = "unc dev-tool abi";
        $(let opts = format!("unc dev-tool abi {}", $cli_opts);)?;
        let result_dir = $crate::invoke_unc! {
            $(Cargo: $cargo_path;)? $(Vars: $cargo_vars;)?
            Opts: &opts;
            Code:
            $($code)*
        };

        let abi_root: unc_abi::AbiRoot =
            serde_json::from_slice(&std::fs::read(result_dir.join(format!("{}_abi.json", function_name!())))?)?;
        abi_root
    }};
}

#[macro_export]
macro_rules! generate_abi {
    ($($code:tt)*) => {{
        $crate::generate_abi_with! {
            Code:
            $($code)*
        }
    }};
}

/// Generate ABI for one function
#[macro_export]
macro_rules! generate_abi_fn_with {
    ($(Cargo: $cargo_path:expr;)? $(Vars: $cargo_vars:expr;)? $(Opts: $cli_opts:expr;)? Code: $($code:tt)*) => {{
        $crate::generate_abi_with! {
            $(Cargo: $cargo_path;)? $(Vars: $cargo_vars;)? $(Opts: $cli_opts;)?
            Code:
            use unc_sdk::borsh::{BorshDeserialize, BorshSerialize};
            use unc_sdk::unc_bindgen;

            #[unc_bindgen]
            #[derive(Default, BorshDeserialize, BorshSerialize)]
            #[borsh(crate = "unc_sdk::borsh")]
            pub struct Contract {}

            #[unc_bindgen]
            impl Contract {
                $($code)*
            }
        }
    }};
}

/// Generate ABI for one function
#[macro_export]
macro_rules! generate_abi_fn {
    ($($code:tt)*) => {{
        $crate::generate_abi_fn_with! {
            Code:
            $($code)*
        }
    }};
}

pub struct BuildResult {
    pub wasm: Vec<u8>,
    pub abi_root: Option<unc_abi::AbiRoot>,
    pub abi_compressed: Option<Vec<u8>>,
}

// TODO: make unc agnostic of stdin/stdout and capture the resulting paths from Writer
#[macro_export]
macro_rules! build_with {
    ($(Cargo: $cargo_path:expr;)? $(Vars: $cargo_vars:expr;)? $(Opts: $cli_opts:expr;)? Code: $($code:tt)*) => {{
        let opts = "unc dev-tool build";
        $(let opts = format!("unc dev-tool build {}", $cli_opts);)?;
        let result_dir = $crate::invoke_unc! {
            $(Cargo: $cargo_path;)? $(Vars: $cargo_vars;)?
            Opts: &opts;
            Code:
            $($code)*
        };

        let manifest_dir: std::path::PathBuf = env!("CARGO_MANIFEST_DIR").into();
        let workspace_dir = manifest_dir.parent().unwrap().join("target").join("_abi-integration-tests");
        let wasm_debug_path = workspace_dir.join("target")
            .join("wasm32-unknown-unknown")
            .join("debug")
            .join(format!("{}.wasm", function_name!()));
        let wasm_release_path = workspace_dir.join("target")
            .join("wasm32-unknown-unknown")
            .join("release")
            .join(format!("{}.wasm", function_name!()));
        let wasm: Vec<u8> = if wasm_release_path.exists() {
            std::fs::read(wasm_release_path)?
        } else {
            std::fs::read(wasm_debug_path)?
        };

        let abi_path = result_dir.join(format!("{}_abi.json", function_name!()));
        let abi_root: Option<unc_abi::AbiRoot> = if abi_path.exists() {
            Some(serde_json::from_slice(&std::fs::read(abi_path)?)?)
        } else {
            None
        };

        let abi_compressed_path = result_dir.join(format!("{}_abi.zst", function_name!()));
        let abi_compressed: Option<Vec<u8>> = if abi_compressed_path.exists() {
            Some(std::fs::read(abi_compressed_path)?)
        } else {
            None
        };

        $crate::BuildResult { wasm, abi_root, abi_compressed }
    }};
}

#[macro_export]
macro_rules! build {
    ($($code:tt)*) => {{
        $crate::build_with! {
            Code:
            $($code)*
        }
    }};
}

#[macro_export]
macro_rules! build_fn_with {
    ($(Cargo: $cargo_path:expr;)? $(Vars: $cargo_vars:expr;)? $(Opts: $cli_opts:expr;)? Code: $($code:tt)*) => {{
        $crate::build_with! {
            $(Cargo: $cargo_path;)? $(Vars: $cargo_vars;)? $(Opts: $cli_opts;)?
            Code:
            use unc_sdk::borsh::{BorshDeserialize, BorshSerialize};
            use unc_sdk::{unc_bindgen, UncSchema};

            #[unc_bindgen]
            #[derive(Default, BorshDeserialize, BorshSerialize)]
            #[borsh(crate = "unc_sdk::borsh")]
            pub struct Contract {}

            #[unc_bindgen]
            impl Contract {
                $($code)*
            }
        }
    }};
}

#[macro_export]
macro_rules! build_fn {
    ($($code:tt)*) => {{
        $crate::build_fn_with! {
            Code:
            $($code)*
        }
    }};
}
