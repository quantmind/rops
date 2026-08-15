# Release notes

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
