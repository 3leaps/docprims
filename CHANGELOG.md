# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **Note:** This file maintains the latest 10 releases in reverse chronological order.
> Older releases are archived in `docs/releases/`.

## [Unreleased]

## [0.1.4] - 2026-01-31

### Added

- **ADR-0006**: TypeScript npm publishing standard documenting OIDC trusted publishing configuration

### Changed

- **Go shared library rpath**: Embedded `-Wl,-rpath` entries in cgo LDFLAGS for darwin-arm64, linux-amd64, linux-arm64; local builds no longer require `LD_LIBRARY_PATH`/`DYLD_LIBRARY_PATH`
- **TypeScript npm publish workflow**: Applied OIDC fixes from sysprims learnings
  - npm CLI upgrade to 11.5.1 (required for OIDC)
  - Retry logic for artifact download (transient network errors)
  - Force OIDC mode pattern (unset tokens, isolated npmrc)
  - Removed `fetch-tags` (causes git conflict on tag ref)
- **Release checklist**: Added TypeScript N-API prebuilds step and npm publishing section

## [0.1.3] - 2026-01-31

### Added

- **TypeScript CI/CD workflows**: Cross-platform prebuild and npm publish automation
  - `typescript-napi-prebuilds.yml`: Builds native addons for linux-x64-gnu, linux-x64-musl, linux-arm64-gnu, linux-arm64-musl, darwin-arm64, win32-x64-msvc
  - `typescript-npm-publish.yml`: OIDC trusted publishing workflow with tag validation
- **TypeScript platform packages**: Optional dependencies for cross-platform npm distribution (`@3leaps/docprims-<platform>`)
- Root `package.json` for monorepo tooling compatibility

### Changed

- TypeScript native loader now falls back to platform packages when local build not present

## [0.1.2] - 2026-01-30

### Fixed

- **Darwin shared library install_name**: macOS dylib now uses `@rpath/libdocprims_ffi.dylib` instead of hardcoded CI build path, enabling proper runtime linking for Go `docprims_shared` consumers
- **CI: musl shared lib handling**: musl targets correctly skip shared library builds (static-only)
- **TypeScript Bun support**: Removed overly conservative runtime guard; Bun now works alongside Node.js

### Changed

- Release checklist: added local testing step for Go static/shared modes
- TypeScript: npm publish dry-run validated; cross-platform prebuild workflow planned for v0.1.3
- TypeScript: validated with both Node.js (v22) and Bun (v1.3) runtimes

## [0.1.1] - 2026-01-29

### Added

- **TypeScript bindings (first cut)** (`bindings/typescript/docprims`)
  - Node-API native addon (napi-rs) + TypeScript wrapper
  - `extractFile` / `extractBytes` plus `*Json` variants
  - Basic tests and CI coverage (linux/macos/windows + Alpine/musl)

- **Go bindings shared-library mode** (`bindings/go/docprims`)
  - Opt-in build tag: `docprims_shared`
  - Vendored shared libraries under `bindings/go/docprims/lib-shared/<platform>/`

### Changed

- Golden tests no longer assert `generator.version` to avoid patch bumps breaking fixtures
- Go bindings docs now mention potential Rust `staticlib` symbol collisions in some cgo applications

## [0.1.0] - 2026-01-28

Initial release of docprims - GPL-free document text extraction primitives.

### Added

- **Core Types** (`docprims-core`)
  - `DocprimsExtract` - top-level extraction result with schema versioning
  - `DocprimsBlock` - structured text blocks with provenance
  - `DocprimsLocation` - format-specific locators for source attribution
  - `ExtractLimits` - configurable resource limits for defensive parsing
  - `ExtractionQuality` - complete vs partial extraction status

- **Text Format Extraction** (`docprims-text`)
  - Markdown extraction with block-level structure (headings, paragraphs, code, lists)
  - HTML extraction with tag stripping and entity decoding
  - XML extraction with text node preservation
  - Schema-conformant v0 output for all text formats

- **OOXML Extraction** (`docprims-ooxml`)
  - DOCX paragraph and table cell extraction
  - XLSX cell extraction with shared string resolution
  - PPTX slide text and speaker notes extraction
  - ZIP archive handling with security controls

- **CLI** (`docprims-cli`)
  - `docprims extract <files>` - unified extraction command
  - Plain text and JSON output modes (`--format plain|json`)
  - NDJSON for multi-file extraction
  - Structured v0 output with `--include-blocks`
  - Logging with `--log-format` and `--log-level` controls
  - rsfulmen exit codes integration

- **FFI** (`docprims-ffi`)
  - C-ABI exports for cross-language bindings
  - `docprims_extract_markdown_v0()` with schema-conformant JSON output
  - Memory-safe string handling

- **Go Bindings** (`bindings/go/docprims`)
  - Prebuilt static libraries for darwin-arm64
  - CGo wrapper with platform-specific build tags
  - Musl support for Alpine containers

- **CI/CD**
  - GitHub Actions workflows for Rust quality gates
  - Go bindings test matrix (linux-amd64, darwin-arm64, musl)
  - Multi-platform FFI build workflow with cargo-zigbuild
  - Windows cross-check validation (no SDK required)

- **Documentation**
  - Architecture overview with Mermaid diagrams
  - ADRs for license policy, crate structure, input validation, stdout purity
  - JSON schema layout and versioning strategy

### Security

- Defensive parsing for untrusted input (zip bombs, XML bombs, path traversal)
- Configurable resource limits (max input/output bytes, max blocks)
- GPL-free dependency policy enforced via cargo-deny
