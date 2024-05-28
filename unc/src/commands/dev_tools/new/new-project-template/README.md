# unc-new-project-name

unc-new-project-description

## How to Build Locally?

Install [`unc`](https://github.com/utnet-org/utility-cli-rs) and run:

```bash
unc dev-tool build
```

## How to Test Locally?

```bash
cargo nextest run --nocapture
```

## How to Deploy?

Deployment is automated with GitHub Actions CI/CD pipeline.
To deploy manually, install [`unc`](https://github.com/utnet-org/utility-cli-rs) and run:

```bash
unc dev-tool deploy <account-id>
```
