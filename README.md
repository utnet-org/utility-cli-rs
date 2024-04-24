# Utility CLI

Utility CLI is your **human-friendly** companion that helps to interact with [Utility](https://utility.org) from command line.

Just run `unc` and let it guide you through!

<p>
  <img src="docs/media/create-account.svg" alt="" width="1200">
</p>

## Install

Visit [Releases page](https://github.com/unc/utility-cli-rs/releases/) to see the latest updates.

<details>
  <summary>Install prebuilt binaries via shell script (macOS, Linux, WSL)</summary>

```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/unc/utility-cli-rs/releases/latest/download/utility-cli-rs-installer.sh | sh
```

</details>

<details>
  <summary>Install prebuilt binaries via powershell script (Windows)</summary>

```sh
irm https://github.com/unc/utility-cli-rs/releases/latest/download/utility-cli-rs-installer.ps1 | iex
```

</details>

<details>
  <summary>Run prebuilt binaries with npx (Node.js)</summary>

```sh
npx utility-cli-rs
```

</details>

<details>
  <summary>Install prebuilt binaries into your npm project (Node.js)</summary>

```sh
npm install utility-cli-rs
```

</details>

<details>
  <summary>Compile and install from the source code (Cargo)</summary>

Install it with `cargo`, just make sure you have [Rust](https://rustup.rs) installed on your computer.

```bash
cargo install utility-cli-rs
```

or, install the most recent version from git repository:

```bash
cargo install --git https://github.com/utnet-org/utility-cli-rs
```

</details>

<details>
  <summary>Install on CI (GitHub Actions)</summary>

It is often desirable to use `unc` cli from CI to automate some actions, so here is an example of how you can make a function call during CI:

```yml
name: Release
on:
  push:
    branches: [main]

jobs:
  deploy-widgets:
    runs-on: ubuntu-latest
    name: Make a function call on mainnet
    env:
      UNC_NETWORK_CONNECTION: mainnet
      UNC_CONTRACT_ACCOUNT_ID: ${{ vars.UNC_CONTRACT_ACCOUNT_ID }}
      UNC_SIGNER_ACCOUNT_ID: ${{ vars.UNC_SIGNER_ACCOUNT_ID }}
      UNC_SIGNER_ACCOUNT_PUBLIC_KEY: ${{ vars.UNC_SIGNER_ACCOUNT_PUBLIC_KEY }}
      UNC_SIGNER_ACCOUNT_PRIVATE_KEY: ${{ secrets.UNC_SIGNER_ACCOUNT_PRIVATE_KEY }}

    steps:
    - name: Checkout repository
      uses: actions/checkout@v3

    - name: Install unc cli
      run: |
        curl --proto '=https' --tlsv1.2 -LsSf https://github.com/utility/utility-cli-rs/releases/download/v0.2.0/utility-cli-rs-installer.sh | sh

    - name: Call some function
      run: |
        unc contract call-function as-transaction "$UNC_CONTRACT_ACCOUNT_ID" 'function_name_here' json-args '{}' prepaid-gas '100 TeraGas' attached-deposit '0 unc' sign-as "$UNC_SIGNER_ACCOUNT_ID" network-config "$UNC_NETWORK_CONNECTION" sign-with-plaintext-private-key --signer-public-key "$UNC_SIGNER_ACCOUNT_PUBLIC_KEY" --signer-private-key "$UNC_SIGNER_ACCOUNT_PRIVATE_KEY" send
```

You will need to configure GitHub Actions Secrets and Variables and once it is ready, this CI will only take a couple of _seconds_ to complete!

See how it is used by [DevHub]([https://github.com/unc/devgigsboard](https://github.com/unc-DevHub/uncdevhub-contract/blob/05fb66ac307d84347f29e8e3ab9f429a78cb6513/.github/workflows/release.yml#L30-L41)).
</details>

## Run

Once installed, you just run it with `unc` command:

```bash
$ unc

? What are you up to? (select one of the options with the up-down arrows on your keyboard and press Enter)
> account     - Manage accounts
  tokens      - Manage token assets such as UNC, FT, NFT
  pledging     - Manage pledging: view, add and withdraw pledge
  contract    - Manage smart-contracts: deploy code, call functions
  transaction - Operate transactions
  config      - Manage connections in a configuration file (config.toml)
  extension   - Manage unc CLI and extensions
[↑↓ to move, enter to select, type to filter]
```

The CLI interactively guides you through some pretty complex topics, helping you make informed decisions along the way.

## [Read more](docs/README.en.md)  

- [Usage](docs/README.en.md#usage)
- [Installation](docs/README.en.md#installation)
- [User Guide](docs/README.en.md#user-guide)
- [Config](docs/README.en.md#config)
- [Building](docs/README.en.md#building)
