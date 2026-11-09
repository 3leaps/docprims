# docprims v0.2 build specification

Status: accepted plan (2026-11-09). Nothing in this document is implemented
yet. Decision records: [ADR-0007](decisions/ADR-0007-profile-b-go-twin-and-wasm-pyo3-bindings.md)
(architecture), [SDR-0003](decisions/SDR-0003-pdf-scope-limits-and-refusals.md)
(PDF scope and limits), [DDR-0001](decisions/DDR-0001-extract-v1-receipts-refusals-and-limits.md)
(contracts, proposed).

docprims v0.2.0 relaunches docprims as a Profile B prim: one Rust crate and a
pure-Go twin under one contract, PDF text extraction landing Go-first, and the
yamlprims release pipeline replacing the v0.1 one.

## Starting point (v0.1.x)

| Area       | State on 2026-11-09                                                              |
| ---------- | -------------------------------------------------------------------------------- |
| Formats    | DOCX, XLSX, PPTX, Markdown, HTML, XML. No PDF.                                   |
| Rust       | Four workspace crates, about 4,500 lines; never published to crates.io.          |
| Go         | cgo binding with about 163 MB of vendored prebuilt libraries in git.             |
| TypeScript | napi-rs addon; `@3leaps/docprims` 0.1.4 on npm.                                  |
| Python     | None; `docprims` unregistered on PyPI.                                           |
| Releases   | Tags v0.1.0 to v0.1.4, annotated but unsigned. VERSION says 0.1.5, never tagged. |

## Packages

| Surface    | Path                        | Package                               | v0.2 formats                                            |
| ---------- | --------------------------- | ------------------------------------- | ------------------------------------------------------- |
| Rust core  | `crates/docprims`           | `docprims` (crates.io)                | DOCX, XLSX, PPTX, Markdown, HTML, XML; PDF if qualified |
| CLI        | `crates/docprims-cli`       | `docprims-cli`                        | Same as Rust core                                       |
| Go twin    | `extract/` in root `go.mod` | `github.com/3leaps/docprims/extract`  | DOCX, XLSX, PPTX, XML, PDF                              |
| TypeScript | `bindings/typescript`       | `@3leaps/docprims` (zero-import WASM) | Same as Rust core                                       |
| Python     | `bindings/python`           | `docprims` (PyO3, abi3-py310)         | Same as Rust core                                       |

Rules (ADR-0007):

- The library takes bytes and performs no filesystem, network or environment
  access. The CLI does file I/O.
- Rust: `#![forbid(unsafe_code)]`, MSRV 1.88, workspace lockfile committed.
- Go: `CGO_ENABLED=0`, standard library only, plus the canonprims Go package
  (DDR-0001). Enforced by `scripts/deps-check.sh`.
- WASM: zero imports. Bindings live outside the root Cargo workspace.
- Formats a tree does not support refuse with `Unsupported`; each refusal is a
  conformance vector.

## Contracts

See DDR-0001: `docprims-extract` v1 result, closed refusal vocabulary,
extraction receipt, limits profile, and per-page completeness through
crucible `coverage-attestation/v0`. Contracts start in docprims `schemas/`
and move to crucible when a second producer or consumer exists.

## Conformance and corpus

- `conformance/vectors/cases.jsonl`: data-authored vectors. The six v0
  goldens become vectors that compare the `document` object only.
- `make crosscheck`: both trees over every vector; byte-identical canonical
  JSON for formats both support.
- **doccorpus** (separate repository, `3leaps/doccorpus`): public, fully
  synthetic document corpus following structcorpus rules (public-only
  admission, byte-exact fixtures, stdlib Python tooling, recipes with output
  digests, hostile cases generated outside git and run under limits).
  Expected text is known by construction from the generator.

| doccorpus tier  | Contents                                                                                                                                                                                                                                  | v0.2  |
| --------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| Conformance     | One feature per case: every filter; WinAnsi; Identity-H with and without ToUnicode; multi-column and rotated text; incremental updates; xref and object streams; empty-password encryption; malformed xref; image-only pages; OOXML parts | Yes   |
| Hostile         | Bombs and malformed structures, each expecting a named refusal                                                                                                                                                                            | Yes   |
| Stress          | Ladders by size, pages, objects and depth, to measure limit defaults                                                                                                                                                                      | Small |
| Producer matrix | Same synthetic text exported by several real producers in a pinned container                                                                                                                                                              | v0.3  |
| Public domain   | US government works and Apache-licensed pdf.js test files, licence per case                                                                                                                                                               | v0.3  |

docprims pins a doccorpus release by commit and content digest, never `main`.

## Release provenance

The v0.2 pipeline ports yamlprims' release tooling (`scripts/release-*`,
`release.yml`, `registry-publish.yml`, `crates-verify.yml`, workflow pin and
permission checks, `keys/` anchors).

| Control         | v0.1                | v0.2                                                                 |
| --------------- | ------------------- | -------------------------------------------------------------------- |
| Tags            | Annotated, unsigned | OpenPGP-signed by `3 Leaps Infosec Team` with a pinned subkey        |
| Checksums       | Minisign            | SHA256SUMS and SHA512SUMS, each signed with Minisign and OpenPGP     |
| Binaries in git | Vendored libraries  | None; CI builds every artifact from the signed tag                   |
| SBOM            | None                | CycloneDX                                                            |
| Registries      | npm only            | crates.io, npm, PyPI by OIDC trusted publishing; Go via module proxy |
| Verification    | Manual              | Post-publish byte comparison against the signed bundle               |

### Maintainer (local-seat) steps

Agents prepare everything up to these steps and hand them over:

- [ ] Sign the specification tag and each `v0.2.x` tag with the infosec
      hardware-backed signing subkey (`make release-tag`).
- [ ] Sign checksum manifests with the org Minisign key and OpenPGP, then
      upload the bundle.
- [ ] First publish of `docprims` and `docprims-cli` to crates.io by token,
      if trusted publishing still requires an existing crate.
- [ ] Configure trusted publishers: npm `@3leaps/docprims`, PyPI pending
      publisher `docprims`, crates.io after first publish.
- [ ] Configure GitHub release environments and protections to match
      yamlprims.
- [ ] Decide whether v0.1.5 is tagged as the last cgo release.
- [ ] Deprecate npm 0.1.x and the old Go module path.

## Work packages and gates

| WP  | Work                                                                                         | Needs    | Ends at                         |
| --- | -------------------------------------------------------------------------------------------- | -------- | ------------------------------- |
| 0   | This build spec, ADR-0007, SDR-0003, DDR-0001, `.fulmen/repo.yaml`                           | —        | **Stop 1: spec accepted**       |
| 1   | doccorpus foundation (conformance, hostile, small stress tiers); v0 goldens as vectors; pins | WP0      | Vectors reviewed                |
| 2   | Merge Rust crates into `docprims`; canonical output; refusals; receipt                       | WP0      | Rust passes vectors             |
| 3   | Go twin: DOCX, XLSX, PPTX, XML                                                               | WP1      | `make crosscheck` green         |
| 4   | Go PDF engine                                                                                | WP1      | PDF go/no-go (SDR-0003)         |
| 5   | Rust PDF                                                                                     | WP4      | Passes vectors, or declared gap |
| 6   | WASM TypeScript and PyO3 Python bindings                                                     | WP2      | **Stop 2: API freeze**          |
| 7   | Port release pipeline; remove cgo, FFI and napi workflows                                    | WP0      | Release tooling tests pass      |
| 8   | Deprecations, README, migration notes                                                        | WP3, WP6 | **Stop 3: publish v0.2.0**      |

Web agent threads open pull requests only. Tagging, signing and publishing
stay with the maintainer and release agents. Each PR gets a crucible
fierce-collaboration panel (`entarch`, `secrev`, `releng`, `qa`; at least one
approving seat off the implementer's reasoner family) before maintainer
review.
