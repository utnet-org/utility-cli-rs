# unc CLI extensions system

`unc CLI` is built to scale. The number of possible features is endless. Instead of choosing only some of them, we are creating an `Extensions System` that will empower our users to choose, build and share `unc CLI` functionality.

## How it works

Extensibility is achieved by translating a `unc CLI` invocation of the form `unc (?<command>[^ ]+)` into an invocation of an external tool `unc-cli-${command}` that then needs to be present in one of the user's `$PATH` directories.
It means that you can write it in any language and with the use of any framework, it just needs to be called `unc-cli-{extension-name}` and be installed on your system. This approach is inspired by [Cargo](https://github.com/rust-lang/cargo).

## How to build an extension

As mentioned above, any binary can become an extension, but we are encouraging developers to use [Rust](https://www.rust-lang.org/), [Clap](https://docs.rs/clap/2.33.0/clap/), and a set of libraries developed by unc. Here is some of them:

- `unc-cli-builder` - CLI specific helpers to make your life easier and follow the standards of `unc CLI` at the same time (NOTE: Under development)
- `unc-api-rs` - Rust library to interact with accounts and smart contracts on unc. (NOTE: Under development)
- [unc-jsonrpc-client-rs](https://github.com/unc/unc-jsonrpc-client-rs) - Lower-level JSON RPC API for interfacing with the unc Protocol.

## Example

Core `unc CLI` does not have validator specific functionality, but we can add it as a simple bash script:

`unc-cli-staking-pool-info`

```bash
#!/bin/sh
POOL_ID=$1
unc execute view-method network mainnet contract "name.unc" call "get_fields_by_pool" '{"pool_id": "'"$POOL_ID"'"}' at-final-block
```

Make sure that this script is in your `$PATH` and has proper permissions to be executed. Then call it like this:

```bash
$ unc staking-pool-info aurora.unc
{
  "country": "Gibraltar",
  "country_code": "gi",
  "github": "auroraisunc",
  "twitter": "auroraisunc",
  "telegram": "auroraisunc",
  "url": "https://aurora.dev/",
  "city": "Gibraltar",
  "description": "Aurora validator fees are spent on supporting the Rainbow Bridge infrastructure, keeping the bridge free and accessible to everyone (except for the gas fees).",
  "logo": "https://aurora.dev/static/favicon-32x32.png",
  "name": "Aurora"
}
```
<!-- TODO: add example written in Rust when it will be available -->