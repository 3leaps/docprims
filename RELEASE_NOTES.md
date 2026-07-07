# Release Notes

> **Note:** This file aggregates the latest 3 releases in reverse chronological order.
> For the complete release history, see `CHANGELOG.md`.
> For detailed release documentation, see `docs/releases/`.

---

## v0.1.5 - 2026-07-07

**Status:** Patch Release (runtime floors + repository guidance)

Refreshes package runtime floors and repository guidance before the next coordinated dependency modernization pass.

### Highlights

- **Rust MSRV**: Raised the workspace MSRV to Rust 1.88.0.
- **TypeScript Node floor**: `@3leaps/docprims` now requires Node.js 22 or newer.
- **Trusted publishing guard**: npm publishing now hard-checks Node >=22.14.0 and npm >=11.5.1.
- **TypeScript workflow alignment**: Binding CI, release validation, and N-API prebuild workflows validate on Node 22.
- **Repository guidance cleanup**: Agent guidance, local-only planning conventions, and YAML linting configuration were refreshed.

### Runtime Floors

v0.1.5 aligns the project with currently supported runtime floors:

| Component | Floor |
|-----------|-------|
| Rust | 1.88.0 |
| TypeScript package | Node.js 22+ |
| npm trusted publishing | Node.js >=22.14.0 and npm >=11.5.1 |

The TypeScript package remains on TypeScript 5.x and napi-rs 2.x for this release. TypeScript 6.x and napi-rs 3.x are reserved for a later coordinated modernization pass.

### Release Workflow Hardening

The npm publish workflow now uses a dedicated runtime guard that fails fast if the publish runner is below the trusted-publishing floor. The same guard is available locally through:

```bash
make npm-publish-prereqs-check
```

`make lint` also runs this guard so workflow drift is caught during normal quality checks.

### Documentation Updates

- README CLI install guidance now uses `cargo install --path crates/docprims-cli` from a repo checkout.
- Repository guidance now treats planning artifacts as local-only and non-canonical.
- YAML linting configuration is explicit at repo root.

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
