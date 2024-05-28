use utility_cli_rs_integration_tests::generate_abi_fn;
use function_name::named;
use unc_abi::{AbiFunctionKind, AbiFunctionModifier};

#[test]
#[named]
fn test_view_function() -> utility_cli_rs::CliResult {
    let abi_root = generate_abi_fn! {
        pub fn add(&self, a: u32, b: u32) -> u32 {
            a + b
        }
    };

    assert_eq!(abi_root.body.functions.len(), 2);
    let function = &abi_root.body.functions[0];
    assert_eq!(function.kind, AbiFunctionKind::View);
    assert_eq!(function.modifiers, vec![]);

    Ok(())
}

#[test]
#[named]
fn test_call_function() -> utility_cli_rs::CliResult {
    let abi_root = generate_abi_fn! {
        pub fn add(&mut self, a: u32, b: u32) -> u32 {
            a + b
        }
    };

    assert_eq!(abi_root.body.functions.len(), 2);
    let function = &abi_root.body.functions[0];
    assert_eq!(function.kind, AbiFunctionKind::Call);
    assert_eq!(function.modifiers, vec![]);

    Ok(())
}

#[test]
#[named]
fn test_init_function() -> utility_cli_rs::CliResult {
    let abi_root = generate_abi_fn! {
        #[init]
        pub fn add(a: u32, b: u32) -> Self {
            Contract {}
        }
    };

    assert_eq!(abi_root.body.functions.len(), 2);
    let function = &abi_root.body.functions[0];
    assert_eq!(function.kind, AbiFunctionKind::Call);
    assert_eq!(function.modifiers, vec![AbiFunctionModifier::Init]);
    assert!(function.result.is_none());

    Ok(())
}

#[test]
#[named]
fn test_payable_function() -> utility_cli_rs::CliResult {
    let abi_root = generate_abi_fn! {
        #[payable]
        pub fn add(&mut self, a: u32, b: u32) -> u32 {
            a + b
        }
    };

    assert_eq!(abi_root.body.functions.len(), 2);
    let function = &abi_root.body.functions[0];
    assert_eq!(function.kind, AbiFunctionKind::Call);
    assert_eq!(function.modifiers, vec![AbiFunctionModifier::Payable]);

    Ok(())
}

#[test]
#[named]
fn test_private_function() -> utility_cli_rs::CliResult {
    let abi_root = generate_abi_fn! {
        #[private]
        pub fn add(&mut self, a: u32, b: u32) -> u32 {
            a + b
        }
    };

    assert_eq!(abi_root.body.functions.len(), 2);
    let function = &abi_root.body.functions[0];
    assert_eq!(function.kind, AbiFunctionKind::Call);
    assert_eq!(function.modifiers, vec![AbiFunctionModifier::Private]);

    Ok(())
}
