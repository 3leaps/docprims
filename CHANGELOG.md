# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

> **Note:** This file maintains the latest 10 releases in reverse chronological order.
> Older releases are archived in `docs/releases/`.

## [Unreleased]

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
