# rOps

Rust operations tool for managing Docker images and Helm charts deployment.

## Installation

`Rops` can self-update, but you can also install it manually by running the following command:

```bash
curl -L https://raw.githubusercontent.com/quantmind/rops/main/dev/install-rops | bash
```

## Configuration

`rops` reads your local `.env` file if it exists, so that you can configure these environment variables:

* `ROPS_CONFIG`: path to the `rops.toml` configuration file (default: `rops.toml`)
* `RUST_LOG`: set the log level (default: `info`)

To get started, you can create a `rops.toml` file in the root of your project with the following content:

```toml
[git]
default_branch = "main"
```

1. Add a new file named `rops.toml` at the root of your project
