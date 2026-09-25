---
id: "PDR-0001"
title: "Publish the docprims crates to crates.io after the release tag"
status: "accepted"
date: "2026-09-25"
deciders:
  - "@3leapsdave"
scope: "docprims crate publication, public API and release process"
tags:
  - "process"
  - "release"
  - "crates-io"
  - "semver"
relates-to:
  - "ADR-0002-crate-structure.md"
  - "ADR-0004-stdout-purity.md"
  - "RELEASE_CHECKLIST.md"
---

# PDR-0001: Publish the docprims crates to crates.io after the release tag

## Decision

### One front door, four published crates

`docprims` is the library's only documented entry point. It re-exports the
public types and dispatches to a parser per format, with each format behind a
feature. The command-line tool is the same crate built with the `cli` feature
(`cargo install docprims --features cli`); the binary name is `docprims`.

A published crate cannot depend on unpublished path-only crates, so
`docprims-core`, `docprims-text` and `docprims-ooxml` are published too. They
are components of `docprims`: their READMEs and docs point at it, and their
APIs carry no stability promise beyond what `docprims` re-exports. Changing
them to `publish = false` would break `docprims`.

`docprims-ffi` and `docprims-ts-napi` are not published. The C library and the
Go and npm packages ship through their own channels.

### Why this shape

Other 3leaps primitive libraries, such as sysprims, publish only independent
subsystem crates and keep their CLI unpublished. docprims differs on purpose:
its crates are format variants of one capability rather than independent
subsystems, and its CLI is a supported artifact with a versioned JSON output
contract (ADR-0004 and `schemas/v0/extract/`). A single name therefore serves
both `docprims = "0.2"` and `cargo install`. This is not a default for other
repositories.

Merging all formats into one crate was not chosen: the workspace split keeps
the FFI and Node addon crates small, and a later format can arrive as a
subcrate plus a `docprims` feature without a new top-level name.

### crates.io after the tag

Publish manually from a clean, guarded checkout of the pushed release tag, in
the order of `config/release/publishable-crates.txt`: `docprims-core`,
`docprims-text`, `docprims-ooxml`, then `docprims`. Wait for the crates.io
index and API after each upload, including the last. The first registry
version is v0.2.0; earlier tags are not backfilled.

The maintainer alone holds a short-lived, crate-scoped token outside the
repository and authorizes each irreversible upload. CI has no registry token
and no publish step. `make release-crates-dry-run` exercises each package with
local path patches for earlier unpublished crates; the real upload has no
patches.

### npm platform packages

The committed npm manifest lists no platform packages, so its lock file
installs with `npm ci`. The publish workflow adds `optionalDependencies` for
exactly the platform packages it publishes, at the same version.

## Public API and semver

- **`serde_json` is a public dependency.** Location hints and metadata expose
  `serde_json::Value` and `serde_json::Map`. A `serde_json` major version bump
  is a breaking change for docprims. This is deliberate: those fields are open
  JSON by design, and `serde_json` has kept one major version.
- **Output structs are not `#[non_exhaustive]`.** The subcrates construct them
  across crate boundaries with struct literals. While docprims is 0.x, an
  additive schema field is released as a new 0.x minor version. How structs
  are constructed is decided before 1.0.
- **Public enums are `#[non_exhaustive]`** (`DocprimsError`,
  `DocprimsQualityStatus`, `DocprimsContainerKind`), so matches on them need a
  wildcard arm and new variants are not breaking for matching code. `serde`
  deserialization still rejects an unknown variant: a consumer that parses
  output from a newer docprims with older types gets an error for a new value.
  That is acceptable while 0.x minors may break; whether 1.0 adds a catch-all
  variant is part of the same pre-1.0 decision.

## Positioning

`extract` is a one-shot operation: one document in, one `extract/v0` result
out. docprims does not define a persisted bundle format or a batch, watch or
service mode. A consumer that stores results wraps `extract/v0` in its own
envelope. If docprims later grows a persisted export or a long-running mode,
it adopts the shared contracts for those shapes rather than extending
`extract/v0`.

## Consequences

- Package checks on the first registry cut may need a rerun once earlier
  crates reach the index. The GitHub draft is not signed until package checks
  and the release workflow are green.
- A new library crate needs an entry in the ordered list and an explicit
  `publish = true`. The list check fails closed on a mismatch and on a
  published FFI or Node addon crate.
- The index and API checks confirm availability, name, version and unyanked
  status; they do not prove byte-for-byte identity with a local archive.
