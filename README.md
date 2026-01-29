# docprims

Content extraction from documents in a permissive licensing toolkit.

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)
[![Rust](https://img.shields.io/badge/rust-1.81%2B-orange.svg)](https://www.rust-lang.org/)

## Overview

**docprims** extracts text from documents with permissive licensing throughout. Use it to build content analysis tools, search indexers, or document processors that can be statically linked and embedded in commercial software.

### Supported Formats

| Format   | Crate            | Status  |
| -------- | ---------------- | ------- |
| DOCX     | `docprims-ooxml` | Planned |
| XLSX     | `docprims-ooxml` | Planned |
| PPTX     | `docprims-ooxml` | Planned |
| PDF      | `docprims-pdf`   | Future  |
| Markdown | `docprims-text`  | Planned |
| HTML     | `docprims-text`  | Planned |
| XML      | `docprims-text`  | Planned |

### The Problem

You need to extract text from PDFs, Word documents, or spreadsheets. Your options:

1. **Use established libraries** — Tools like poppler or mupdf work well, but their copyleft licenses restrict how you can distribute your software
2. **Roll your own** — Format specs are complex; PDF alone has decades of edge cases
3. **Pay for commercial solutions** — Often expensive and still come with licensing constraints

docprims offers a fourth path: MIT/Apache-2.0 licensed extraction that you can statically link, embed in commercial software, or ship without copyleft obligations.

### Who Should Use This

**AI/ML Engineers**: Building pipelines that ingest documents for training, RAG, or content analysis. You need extraction without license overhead in your data stack.

**Platform Teams**: Your legal department has opinions about copyleft licenses in your software supply chain. docprims is designed for environments where license hygiene matters.

**Document Processing SaaS**: Building products that handle customer documents. Embedding a permissively-licensed extractor simplifies your licensing story.

**Open Source Projects**: Avoiding license compatibility debates. MIT/Apache-2.0 is unambiguous.

## Installation

### Rust

```toml
[dependencies]
docprims-ooxml = "0.1"
docprims-text = "0.1"
```

### Go

```bash
go get github.com/3leaps/docprims/bindings/go/docprims
```

### CLI

```bash
cargo install docprims-cli
```

## Usage

### Rust

```rust
use docprims_ooxml::extract_docx;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let text = extract_docx("document.docx")?;
    println!("{}", text.content);
    Ok(())
}
```

### CLI

```bash
# Extract text from a document
docprims extract document.docx

# Output as JSON with metadata
docprims extract document.docx --format json --include-metadata

# Extract from multiple formats
docprims extract report.pdf slides.pptx data.xlsx
```

## Architecture

```
docprims/
├── crates/
│   ├── docprims-core/    # Shared types, traits, errors
│   ├── docprims-text/    # Markdown, HTML, XML
│   ├── docprims-ooxml/   # DOCX, XLSX, PPTX
│   └── docprims-cli/     # CLI binary
├── ffi/
│   └── docprims-ffi/     # C-ABI for language bindings
└── bindings/
    ├── go/               # Go binding
    ├── typescript/       # TypeScript/Node binding
    └── python/           # Python binding
```

## Development

Project documentation conventions (what is canonical vs planning notes):
`docs/orientation/sources-of-truth.md`

### Prerequisites

- Rust 1.81+
- curl (for bootstrap)

### Setup

```bash
make bootstrap    # Install development tools
make check        # Run all quality checks
make test         # Run tests
```

### Quality Gates

```bash
make fmt          # Format code
make lint         # Run clippy
make deny         # Check licenses (GPL-free enforcement)
make audit        # Security vulnerability scan
```

## Supply Chain

docprims is designed for environments where dependency hygiene matters:

- **License-clean**: All dependencies use MIT, Apache-2.0, or compatible licenses
- **Auditable**: Run `cargo tree` to inspect the full dependency graph
- **SBOM-ready**: Compatible with `cargo sbom`
- **No runtime network calls**: All functionality is local

```bash
# Check dependencies
cargo deny check licenses

# Audit for vulnerabilities
cargo audit
```

## Prior Art

docprims builds on ideas from others in this space:

- **[poppler](https://poppler.freedesktop.org/)** — Excellent PDF rendering library (GPL). If copyleft works for your use case, it's battle-tested.
- **[mupdf](https://mupdf.com/)** — High-quality document toolkit (AGPL). Commercial licenses available.
- **[calamine](https://github.com/tafia/calamine)** — MIT-licensed Rust XLSX/ODS reader. Good reference for spreadsheet parsing.
- **[pdf-rs](https://github.com/nicowilliams/pdf-rs)** — MIT-licensed Rust PDF parsing.

We're not claiming to replace these projects. docprims fills a specific niche: embeddable, license-clean document extraction with first-class bindings.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

Subject to [3 Leaps OSS policies](https://github.com/3leaps/oss-policies).

## Related Projects

- **[sysprims](https://github.com/3leaps/sysprims)** — Process control primitives with the same licensing philosophy (sibling project)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
