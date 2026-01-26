# ADR-0002: Crate Structure

> **Status**: Accepted
> **Date**: 2025-01-25
> **Authors**: 3leaps Architecture Council

## Context

docprims needs to support multiple document formats (OOXML, PDF, HTML, Markdown, XML) with:
- A unified API for common operations
- Format-specific APIs where needed
- FFI bindings for Go, TypeScript, Python
- A CLI for reference and testing

We need to decide how to organize the crates in the workspace.

## Decision

Organize as a Cargo workspace with the following structure:

```
crates/
├── docprims-core/     # Shared types, traits, errors
├── docprims-text/     # Text-based formats (Markdown, HTML, XML)
├── docprims-ooxml/    # Office Open XML (DOCX, XLSX, PPTX)
├── docprims-cli/      # CLI binary
ffi/
└── docprims-ffi/      # Unified FFI for all formats
```

### Crate Responsibilities

**docprims-core**:
- `ExtractedText` struct and related types
- `Extractor` trait for unified API
- Error types (`DocprimsError`)
- Metadata types (`DocumentMetadata`)
- No format-specific code

**docprims-text**:
- Markdown text extraction (strip syntax or pass through)
- HTML text extraction (strip tags, decode entities)
- XML text extraction (extract text nodes)
- Feature flags for each format

**docprims-ooxml**:
- DOCX paragraph/table extraction
- XLSX cell/sheet extraction
- PPTX slide/notes extraction
- ZIP archive handling
- XML namespace handling

**docprims-cli**:
- Unified `docprims extract` command
- Format auto-detection
- Output formats (plain, JSON, markdown)

**docprims-ffi**:
- C-ABI exports for all formats
- Single shared library
- Memory management helpers

### Future: docprims-pdf

When PDF support is added:
```
crates/
└── docprims-pdf/      # PDF text extraction
```

## Consequences

### Positive

- Clear separation of concerns
- Users can depend on only what they need
- Text formats grouped together (similar complexity)
- Single FFI crate simplifies bindings

### Negative

- More crates to maintain
- Cross-crate API changes require coordination
- FFI crate depends on all format crates

### Neutral

- Follows sysprims workspace pattern
- Standard Rust workspace conventions

## Alternatives Considered

### Alternative 1: Monolithic crate

Single `docprims` crate with feature flags for each format. Rejected because:
- Large compile times even for partial usage
- Less clear API boundaries
- Harder to test in isolation

### Alternative 2: Separate FFI per format

`docprims-ooxml-ffi`, `docprims-pdf-ffi`, etc. Rejected because:
- More build complexity
- Bindings need to link multiple libraries
- Unified API is a key feature

### Alternative 3: Separate crate per format

`docprims-docx`, `docprims-xlsx`, `docprims-pptx` instead of unified `docprims-ooxml`. Rejected because:
- OOXML formats share significant code (ZIP, XML namespaces)
- Unnecessary granularity
- More crates to maintain

## References

- [sysprims crate structure](https://github.com/3leaps/sysprims/blob/main/docs/architecture/adr/0002-crate-structure.md)
- [Cargo workspaces documentation](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html)
