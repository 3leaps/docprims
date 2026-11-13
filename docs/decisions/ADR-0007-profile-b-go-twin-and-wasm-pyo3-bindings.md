# ADR-0007: Profile B Go Twin, Single Rust Crate, and WASM/PyO3 Bindings

> **Status**: Accepted
> **Date**: 2026-11-13
> **Authors**: devlead
> **Supervised by**: @3leapsdave
> **Supersedes**: [ADR-0002](ADR-0002-crate-structure.md) (crate structure)

## Context

docprims v0.1 is a Profile A prim: a Rust workspace of four crates
(`docprims-core`, `docprims-text`, `docprims-ooxml`, `docprims-cli`), a C ABI
crate (`ffi/docprims-ffi`), a cgo Go binding that vendors prebuilt static and
shared libraries in git (about 163 MB), and a napi-rs TypeScript addon.

That shape has three problems:

1. Go consumers must carry cgo. The README documents duplicate Rust runtime
   symbol failures (`_rust_eh_personality`) when docprims is linked next to
   another Rust `staticlib`.
2. Prebuilt binaries committed to git cannot be traced to a signed tag.
3. The four-crate split creates compatibility surfaces without a consumer
   that needs them separately. None of the four crates was ever published to
   crates.io, so changing the split breaks no one.

crucible ADR-0018 and the repository-classes standard define Profile B: locked
parallel pure-Go and Rust implementations under one contract, version, tag and
conformance suite. yamlprims (its ADR-0004) and canonprims are the reference
implementations.

The first priorities for v0.2 are PDF and a pure-Go tree that Go programs can
use with `CGO_ENABLED=0`.

## Decision

1. **Profile B.** docprims becomes a Profile B prim. The Rust tree and a
   pure-Go tree implement one contract (DDR-0001, `schemas/`) and pass one
   shared conformance suite. `.fulmen/repo.yaml` declares the class, profile,
   trees and extensions.
2. **Go tree operates first.** The repo bootstraps one tree at a time, as
   ADR-0018 §2 allows. It declares `profile_status: bootstrapping` with Go as
   the operating tree, and only the Go tree is graded against the shared
   vectors until the Rust tree catches up. Vector expectations are authored as
   data, never snapshotted from the Go tree. When both trees pass, the repo
   flips to `profile_status: locked`, the same-PR lockstep rule starts, and the
   flip is recorded in the release notes.
3. **Single Rust crate.** `docprims-core`, `docprims-text` and
   `docprims-ooxml` merge into one `docprims` crate with internal modules.
   `docprims-cli` stays a thin CLI over it. Both forbid unsafe code. Each
   format family is a Cargo feature (`pdf`, `ooxml`, `text`), all on by
   default. A consumer that disables a feature does not pull that family's
   dependencies (for example `zip` and `quick-xml` for `ooxml`, `scraper` and
   `pulldown-cmark` for `text`). A compiled-out format returns the same
   `Unsupported` refusal as an unbuilt one, so features change availability,
   never semantics. CI builds and tests each feature alone and all together.
4. **Pure-Go twin in the root module.** The Go tree is the package
   `github.com/3leaps/docprims/extract` in the root `go.mod`, mirroring
   yamlprims' `yaml/` package, so one `VERSION` and one `vX.Y.Z` tag resolve
   (repository-classes C6). It builds with `CGO_ENABLED=0` and depends only on
   the Go standard library and the canonprims Go package (ADR-0008), enforced
   by a dependency check script. Format families are importable subpackages
   (`extract/pdf`, `extract/ooxml`) with `extract` as a dispatcher over them,
   so a consumer importing one family links only that family. v0.2 scope in
   Go: PDF, DOCX, XLSX, PPTX and XML.
5. **HTML and Markdown are Rust-only in v0.2, as declared extensions.** Go has
   no standard-library HTML or Markdown parser. These two formats are declared
   in `.fulmen/repo.yaml` `extensions` with `scope: rust-only`. The shared
   vectors test them as an explicit `Unsupported` refusal on the Go tree, and
   those two formats alone are scoped out of the cross-tree byte-identical
   requirement. No sibling prim is involved, so this is not ADR-0018's
   sibling capability gap; it uses the same refusal-vector mechanics under the
   standard's rule that a language-only capability is a declared extension
   that does not change shared behaviour. Go support is planned for v0.3 with
   `golang.org/x/net/html` (BSD-3-Clause) and `github.com/yuin/goldmark`
   (MIT); adding them widens the Go dependency allow-list and is recorded when
   it happens.
6. **Retire cgo and the C ABI.** `bindings/go/docprims`, `ffi/docprims-ffi`,
   `cbindgen.toml` and the vendored libraries are removed from the tree in
   v0.2. The nested Go module was already tagged, so until then it is declared
   as a `legacy-go-module` extension. v0.1.5 is its final release (ADR-0009).
   Git history is not rewritten.
7. **TypeScript as WASM.** `@3leaps/docprims` 0.2.x is a zero-import
   `wasm32-unknown-unknown` module built from the Rust crate: bytes in, JSON
   out, no filesystem access. The napi-rs addon is retired and npm 0.1.x is
   deprecated with a pointer to 0.2.0.
8. **Python via PyO3.** A new `docprims` wheel (`abi3-py310`) built with
   maturin from the Rust crate.
9. **Isolation.** `bindings/typescript` and `bindings/python` are separate
   Cargo manifests outside the root workspace, as in yamlprims, so WASM and
   PyO3 dependencies stay out of the root lockfile and the crates.io
   verification closure.
10. **Library boundary.** The library takes bytes, never paths, and performs
    no filesystem, network or environment access in either tree. File I/O
    belongs to the CLI.

## Consequences

### Positive

- Go consumers get docprims with `CGO_ENABLED=0` and no linker conflicts.
- Every shipped artifact is built by CI from a signed tag (ADR-0009).
- One crate is one compatibility surface.
- PDF and the Go tree, the two things v0.2 is for, are built first.

### Negative

- Two implementations of OOXML, XML and PDF must be kept in lockstep once the
  repo is locked.
- TypeScript and Python come from the Rust tree, so they follow the Rust tree
  rather than the Go tree during bootstrap.
- TypeScript 0.2.x is a breaking change from the 0.1.x napi addon (paths no
  longer accepted; bytes only).
- HTML and Markdown are unavailable to pure-Go consumers in v0.2.

### Neutral

- The shared conformance suite and `make crosscheck` become the definition of
  "one product".

## Alternatives Considered

### Keep Profile A and fix the cgo link model

Does not remove cgo, which is the requirement for Go consumers.

### Separate Go repository

ADR-0018 requires one repo, one version and one tag for Profile B.

### Keep the four-crate split, or one crate per format family

Limits each consumer's dependency closure, but multiplies crates to version,
publish and verify. Cargo features give the same closure control with one
semver line, one publish and one tag.

### Treat Rust-only HTML and Markdown as an ADR-0018 sibling gap

The sibling-gap clause is written for a Profile A sibling that cannot serve the
Go tree. Stretching it to formats a tree has not built would need an
amendment to ADR-0018. The declared-extension reading needs none.

### Rust tree operates first

The v0.1 Rust code exists, but most of it is reorganised by this record and
PDF is new in both trees. Starting with Go delivers the pure-Go capability
that motivates v0.2 and puts the hardest new work (PDF) where it is needed
first.

## References

- crucible ADR-0018 and `docs/standards/repository-classes.md` (Profile B,
  C6, P3, P7, §4)
- yamlprims ADR-0004 (Profile B pure-Go twin and isolated WASM/PyO3 bindings)
- [ADR-0008](ADR-0008-canonprims-sibling-dependency.md),
  [ADR-0009](ADR-0009-signed-releases-and-v0.1-cgo-deprecation.md),
  [DDR-0001](DDR-0001-extract-v1-receipts-refusals-and-limits.md),
  [SDR-0003](SDR-0003-pdf-scope-limits-and-refusals.md)
