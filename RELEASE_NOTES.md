# Release Notes

> **Note:** This file aggregates the latest 3 releases in reverse chronological order.
> For the complete release history, see `CHANGELOG.md`.
> For detailed release documentation, see `docs/releases/`.

---

## v0.2.0 — 2026-09-25

**Status:** Minor Release (library entry point, extraction fixes, dependency refresh)

### Summary

v0.2.0 adds the `docprims` crate as the library's entry point, fixes several
extraction edge cases, and updates dependencies and toolchains across the
Rust crates and the C, Go and TypeScript bindings. It contains breaking
changes for Rust users of the subcrates; bindings keep their results and
error codes.

### Highlights

- **`docprims` crate**: One dependency for all formats, with a feature per format.
- **Extracted text character set**: Every format emits only XML 1.0 characters, excluding DEL and C1 controls.
- **Numeric character references** resolve in DOCX, XLSX and PPTX.
- **OOXML decompression limits** are enforced on bytes actually read, per part and per archive.
- **Output limits** never produce an empty block or a trailing separator.
- **CLI** is installed with `cargo install docprims --features cli`.

### Using the `docprims` crate

```toml
[dependencies]
docprims = "0.2"
```

```rust
use docprims::{extract_file, ExtractLimits};

let result = extract_file("report.docx", ExtractLimits::default())?;
println!("{}", result.document.text);
```

The parser is chosen from the file extension, or explicitly with
`extract_file_as` / `extract_bytes_as` and a `Format`. Content is never
inspected to choose a parser.

Formats are features: `markdown`, `html`, `xml`, `docx`, `xlsx`, `pptx`,
grouped as `text` and `ooxml`, all on by default. A Markdown-only build:

```toml
docprims = { version = "0.2", default-features = false, features = ["markdown"] }
```

`docprims-core`, `docprims-text` and `docprims-ooxml` are published as
components of `docprims` and carry no stability promise beyond what
`docprims` re-exports.

### Breaking Changes

| Change | Migration |
|--------|-----------|
| `docprims-cli` crate removed | `cargo install docprims --features cli` (same binary name, commands and output) |
| `--timeout-ms` / `ExtractOptions::timeout_ms` removed (never applied) | Remove the flag or field |
| `ExtractOptions` removed from `docprims-core` | Use `ExtractLimits` |
| `docprims_ooxml::common` no longer public | Use `docprims::extract_*` |
| `DocprimsError`, `DocprimsQualityStatus`, `DocprimsContainerKind` are `#[non_exhaustive]` | Add a wildcard arm to matches |

### Extraction Changes

Extracted text may differ from v0.1.x for documents that contain:

- control characters, DEL or C1 controls (now dropped);
- numeric character references in DOCX, XLSX or PPTX (now resolved);
- output truncated by `max_output_bytes` (no empty final block).

Block byte ranges are computed after characters are dropped, so they always
index the emitted text.

### Runtime Floors

| Component | Floor |
|-----------|-------|
| Rust (MSRV) | 1.88.0 |
| TypeScript package | Node.js 22+ |
| npm trusted publishing | Node.js >=22.14.0 and npm >=11.5.1 |

### Changelog

See `CHANGELOG.md` for the full list of changes.

---

## v0.1.4 - 2026-01-31

**Status:** Patch Release (Go rpath + TypeScript OIDC fixes)

Improves Go shared library developer experience and applies npm OIDC publishing fixes from sysprims learnings.

### Highlights

- **Go shared library rpath**: Local builds no longer require `LD_LIBRARY_PATH`/`DYLD_LIBRARY_PATH`
- **TypeScript npm OIDC fixes**: Workflow now correctly uses OIDC trusted publishing
- **ADR-0006**: Documents TypeScript npm publishing standard

### Go Shared Library Improvement

v0.1.4 embeds `-Wl,-rpath` entries in cgo LDFLAGS. Local development now works without environment variables:

```bash
# Before v0.1.4
DYLD_LIBRARY_PATH=./lib-shared/darwin-arm64 go run .

# v0.1.4+
go run .  # Just works
```

For distribution, bundle the shared library and use platform-appropriate rpath:
- macOS: `@executable_path`
- Linux: `$ORIGIN`

### TypeScript npm Publishing

The npm publish workflow now correctly uses OIDC trusted publishing:
- npm CLI upgrade to 11.5.1 (required for OIDC)
- Force OIDC mode pattern (isolated npmrc, unset tokens)
- Retry logic for artifact download

---

## v0.1.3 - 2026-01-31

**Status:** Patch Release (TypeScript CI/CD)

Release CI/CD process improvements for TypeScript bindings.

### Highlights

- **Cross-platform prebuilds**: New workflow builds native addons for all supported platforms
- **npm trusted publishing**: OIDC-based workflow for secure, automated npm releases
- **Platform packages**: Optional dependencies enable `npm install` without local Rust toolchain

### Supported Platforms

| Platform | Package |
|----------|---------|
| Linux x64 (glibc) | `@3leaps/docprims-linux-x64-gnu` |
| Linux x64 (musl) | `@3leaps/docprims-linux-x64-musl` |
| Linux arm64 (glibc) | `@3leaps/docprims-linux-arm64-gnu` |
| Linux arm64 (musl) | `@3leaps/docprims-linux-arm64-musl` |
| macOS arm64 | `@3leaps/docprims-darwin-arm64` |
| Windows x64 | `@3leaps/docprims-win32-x64-msvc` |

### Installation

```bash
npm install @3leaps/docprims
```

Platform-specific binaries install automatically as optional dependencies.

---

*For older releases, see `docs/releases/`.*
