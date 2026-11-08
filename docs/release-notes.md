# Release notes

## v0.7.2

A small release that lets metablock routes carry tags.

### New features

- Routes in a metablock configuration accept an optional `tags` list, sent
  to the metablock API with the block so that extensions can find the route
  by tag. Routes without tags are sent exactly as before.

[Full changelog](https://github.com/quantmind/rops/compare/v0.7.1...v0.7.2)

## v0.7.1

A bug-fix release for chart deploys, multi-arch Docker images and GitHub
release downloads.

### Improvements and fixes

- Deploying a chart no longer fails with "no cached repo found" when its Helm
  repository is already registered with a missing or stale index: rops now
  adds the repository with `--force-update`, which always downloads its index
  ([#9](https://github.com/quantmind/rops/pull/9)).
- Multi-arch Docker builds disable BuildKit's default attestations, so each
  pushed per-arch image is a plain image that `docker manifest create`
  accepts ([#9](https://github.com/quantmind/rops/pull/9)).
- A failed GitHub API request when downloading a tool release now reports the
  HTTP status and the API's message instead of an opaque "error decoding
  response body" ([#8](https://github.com/quantmind/rops/pull/8)).

[Full changelog](https://github.com/quantmind/rops/compare/v0.7.0...v0.7.1)

## v0.7.0

Kubernetes tooling and Helm 4 support. `kubectl` can now be installed as a
managed tool, and `rops charts update` installs the signed helm-secrets
plugins that Helm 4 requires.

### New features

- `kubectl` is available as a third-party tool, downloaded from the official
  `dl.k8s.io` release site at the latest Kubernetes version
  ([#5](https://github.com/quantmind/rops/pull/5)).
- `rops charts update` installs the latest `secrets`, `secrets-getter` and
  `secrets-post-renderer` plugins from the helm-secrets GitHub release,
  verifying their signatures with the helm-secrets author's public key, which
  is built into rops. Helm 4 only installs signed plugins
  ([#6](https://github.com/quantmind/rops/pull/6)).

[Full changelog](https://github.com/quantmind/rops/compare/v0.6.0...v0.7.0)

## v0.6.0

Readable listings and a sturdier installer.

- `rops charts ls`, `rops tools ls` and `rops settings` print a table, with a
  `--raw` flag for JSON output ([#4](https://github.com/quantmind/rops/pull/4)).
- The `install-rops` script retries transient GitHub API errors and reports
  why a request failed ([#3](https://github.com/quantmind/rops/pull/3)).

[Full changelog](https://github.com/quantmind/rops/compare/v0.5.6...v0.6.0)

## v0.5.6

A fix for the metablock block API. Deploying a chart with a `block` section
failed with `error decoding response body`, because the API now resolves the
organization a request acts within from a header rops did not send.

### Improvements and fixes

- Send the `x-metablock-org-id` header on every block request. The organization
  is taken from the `[blocks] org` setting or the `METABLOCK_ORG` variable and
  resolved to its id, since the header matches organizations by id only. It
  defaults to `metablock`, the organization owning the spaces, which is not
  named after them: the `quantmind` space belongs to the `metablock`
  organization ([#2](https://github.com/quantmind/rops/pull/2)).
- Report an error response from the metablock API by its status and body.
  `get_block` decoded the response without looking at the status, turning the
  `422` answered to a request missing the header into reqwest's opaque
  `error decoding response body`; `create_block` and `update_block` reported
  client errors but not server ones
  ([#2](https://github.com/quantmind/rops/pull/2)).

### Documentation and assets

- Release bodies are now published from this file, see the release
  instructions in `.github/instructions/release.instructions.md`.

[Full changelog](https://github.com/quantmind/rops/compare/v0.5.5...v0.5.6)
