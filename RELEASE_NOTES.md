# Release Notes

> **Note:** This file aggregates the latest 3 releases in reverse chronological order.
> For the complete release history, see `CHANGELOG.md`.
> For detailed release documentation, see `docs/releases/`.

---

## v0.2.0 — 2026-09-25

**Status:** Minor Release (library entry point, extraction fixes, dependency refresh)

Adds the `docprims` crate as the library's entry point, fixes extraction edge cases, and refreshes dependencies and toolchains. Breaking for Rust users of the subcrates; bindings keep their results and error codes.

### Highlights

- **`docprims` crate**: One dependency for all formats, a feature per format; the parser is chosen by extension or an explicit `Format`, never by content.
- **CLI**: `cargo install docprims --features cli`; binary name, commands and output are unchanged.
- **Extracted text character set**: XML 1.0 characters only, excluding DEL and C1 controls, on every format; numeric character references resolve in DOCX, XLSX and PPTX.
- **OOXML decompression limits**: Enforced on bytes actually read, per part (100 MiB) and per archive (400 MiB).
- **Output limits**: No empty block or trailing separator when `max_output_bytes` truncates.
- **Toolchains**: MSRV 1.88.0; Node.js 22+; the TypeScript binding builds with napi-rs 3 and TypeScript 7.

### Breaking Changes

- `docprims-cli` crate removed (use the `docprims` crate's `cli` feature).
- `--timeout-ms` / `ExtractOptions::timeout_ms` and `ExtractOptions` removed.
- `docprims_ooxml::common` is private.
- `DocprimsError`, `DocprimsQualityStatus` and `DocprimsContainerKind` are `#[non_exhaustive]`.

See `docs/releases/v0.2.0.md` for usage and migration.

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
