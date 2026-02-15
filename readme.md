# rOps

Rust operations tool for managing Docker images and Helm charts deployment.

## Installation

`Rops` can self-update, but you can also install it manually by running the following command:

```bash
curl -L https://raw.githubusercontent.com/quantmind/rops/main/dev/install-rops | bash
```

It will download the latest release from GitHub and place the executable in `$HOME/bin/rops`. Make sure that this directory is in your `PATH` to use `rops` from anywhere. if you want to specify a different installation directory, you can set the `ROPS_INSTALL_DIR` environment variable before running the installer:

```bash
export ROPS_INSTALL_DIR=/your/custom/path
```

If `GITHUB_TOKEN` is set in your environment, the installer will use it to authenticate with the GitHub API and avoid rate limits. You can set it with:

```bash
export GITHUB_TOKEN=your_token
```

## Configuration

`rops` reads your local `.env` file if it exists, so that you can configure these environment variables:

* `ROPS_CONFIG`: path to the `rops.toml` configuration file (default: `rops.toml`)
* `RUST_LOG`: set the log level (default: `info`)
* `GITHUB_TOKEN`: GitHub token for authenticated API requests (optional)

To get started, you can create a `rops.toml` file in the root of your project with the following content:

```toml
[git]
default_branch = "main"
```
