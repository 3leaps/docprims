# CI/CD

## Local gates

| Target | Runs |
|--------|------|
| `make precommit` | `fmt-check`, `lint` |
| `make check` | format, lint, tests, lean-library check, docs build, `cargo-deny`, `cargo-audit` |
| `make prepush` | `check`, `version-check`, `release-tooling-test` |
| `make release-check` | `version-check`, then packages and verify-builds every publishable crate without publishing |
| `make release-preflight` | pre-tag gate: anchors, clean tree, `prepush`, `release-check`, release notes, `HEAD` == `origin/main` |

`make release-tooling-test` runs the release guard, signing, asset, crate,
version and release-notes tests. The Decernor test uses `DOCPRIMS_DECERNOR_BIN`
when set, otherwise `decernor` on `PATH`.

## Workflows

| Workflow | Trigger | Purpose |
|----------|---------|---------|
| `ci` | push to `main`, PR | quality gates, header drift, MSRV, `publish-check` (package check and release tooling tests with a checksum-pinned Decernor), Go vendored-lib tests |
| `go-bindings` | PR, manual | Go bindings against locally built libraries |
| `typescript-bindings` | push to `main`, PR | TypeScript build and tests on glibc and musl |
| `typescript-napi-prebuilds` | manual | cross-platform `.node` prebuilds and the npm package directory; publishes nothing |
| `go-bindings-prep` | manual | builds the Go prebuilt libraries and opens a PR with them |
| `release` | `v*` tag push, manual | builds and validates the exact release asset set; on a tag push only, verifies the signed tag and opens an unsigned draft release |
| `validate-release` | manual | validates a pushed tag from Go and TypeScript |
| `typescript-npm-publish` | manual | publishes the npm packages with OIDC trusted publishing |

All workflows default to read-only `contents` permission and pin every action
to a commit SHA. Jobs that write (the release draft, the Go prep PR, npm OIDC)
declare their own permissions.

## Release workflow

A manual run of `release` builds every CLI archive, the C header and the SBOM,
then validates the assembled set against `config/release/cli-platforms.txt`.
It creates no release, so the build matrix can be proven before a tag exists.

On a tag push, `validate` binds the annotated tag to `VERSION` and
`origin/main`, and `verify-signature` checks the tag signature against the
committed public pin and GitHub verification. The draft job runs only after
both pass.

### Credentials

The strict release guard fetches the release tag and `origin/main`. The
read-only `validate` job keeps its read-scoped checkout credential for that
guard. The draft job has a write-scoped token, so it provides an authenticated
Git header only while running the guard and removes it before the steps that
create the draft.

After a tag workflow failure, confirm whether the draft exists and its unsigned
asset inventory is complete before running `make release`. Repair the workflow
and recreate the annotated tag only when the failed workflow did not create a
draft.
