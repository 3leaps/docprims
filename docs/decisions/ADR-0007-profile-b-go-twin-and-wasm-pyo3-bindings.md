# ADR-0007: Profile B Go Twin, Single Rust Crate, and WASM/PyO3 Bindings

> **Status**: Accepted
> **Date**: 2026-11-09
> **Authors**: 3leaps Architecture Council
> **Supersedes**: ADR-0002 (crate structure)

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

crucible ADR-0018 defines Profile B (locked parallel pure-Go and Rust
implementations under one contract, version, tag and conformance suite), and
yamlprims ADR-0004 and canonprims are the reference implementations.

## Decision

1. **Profile B.** docprims becomes a Profile B prim. The Rust tree and a
   pure-Go tree implement one contract (`docs/build-spec.md`, DDR-0001) and
   pass one shared conformance suite.
2. **Bootstrapping by capability.** Each tree declares the formats it supports
   in `.fulmen/repo.yaml` (`capabilities`). A format a tree does not support
   returns a typed `Unsupported` refusal, and each such refusal is a
   conformance vector. Cross-tree byte identity is required only for formats
   both trees support. This applies the ADR-0018 per-tree capability gap to
   formats not yet built; see Open Question 1.
3. **Single Rust crate.** `docprims-core`, `docprims-text` and
   `docprims-ooxml` merge into one `docprims` crate with internal modules.
   `docprims-cli` stays a thin CLI over it. Rust forbids unsafe code in both.
4. **Pure-Go twin.** A new package `github.com/3leaps/docprims/extract` in the
   root `go.mod`, `CGO_ENABLED=0`, standard library only, enforced by a
   dependency check script. v0.2 scope: DOCX, XLSX, PPTX, XML and PDF.
5. **HTML and Markdown stay Rust-only in v0.2.** Go has no standard-library
   HTML or Markdown parser. A dependency decision for the Go tree is deferred
   to v0.3.
6. **Retire cgo and the C ABI.** `bindings/go/docprims`, `ffi/docprims-ffi`,
   `cbindgen.toml` and the vendored libraries are removed from the tree. The
   old Go module path gets one final deprecation release. Git history is not
   rewritten.
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
- Every shipped artifact is built by CI from a signed tag.
- One crate is one compatibility surface.

### Negative

- Two implementations of OOXML, XML and PDF must be kept in lockstep.
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

### Keep the four-crate split

No consumer needs the formats as separate crates; the split multiplies
publishing and semver work.

## Open Questions

1. Does ADR-0018's per-tree capability gap, written for sibling-dependency
   gaps, also cover formats a tree has not built yet? This record assumes yes.
   If crucible disagrees, it needs a short amendment.
2. Go package name: `extract` is the working name.
