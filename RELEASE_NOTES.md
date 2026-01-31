# Release Notes

> **Note:** This file aggregates the latest 3 releases in reverse chronological order.
> For the complete release history, see `CHANGELOG.md`.
> For detailed release documentation, see `docs/releases/`.

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

## v0.1.2 - 2026-01-30

**Status:** Patch Release (Go dynamic libs + TypeScript publish prep)

Fixes Go shared library distribution on macOS and captures learnings from TypeScript npm publish dry-run.

### Highlights

- **Darwin dylib fix**: `install_name_tool -id "@rpath/libdocprims_ffi.dylib"` fixes runtime linking for Go `docprims_shared`
- **CI refinements**: musl targets skip shared lib builds (static-only by design)
- **TypeScript npm prep**: dry-run validated current setup publishes local platform only; cross-platform prebuild workflow planned

### Go Shared Library (darwin)

```bash
DYLD_LIBRARY_PATH=./lib-shared/darwin-arm64 go run .  # development
go build -ldflags="-r /path/to/lib-shared/darwin-arm64" .  # production
```

### TypeScript (from git checkout)

npm publishing requires cross-platform prebuilds. Until then, use from git:

```bash
cd bindings/typescript/docprims && npm install && npm run build:native
```

---

*For older releases, see `docs/releases/`.*
