# docprims

GPL-free document text extraction primitives.

[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)
[![Rust](https://img.shields.io/badge/rust-1.81%2B-orange.svg)](https://www.rust-lang.org/)

## Overview

**docprims** extracts text from documents without GPL license contamination. Use it to build content analysis tools, search indexers, or document processors that can be safely embedded in commercial software.

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

### Why docprims?

Most document extraction libraries are GPL-licensed (poppler, mupdf). If you need to:

- Build commercial software with document extraction
- Statically link a document parser
- Avoid GPL license obligations

...then you need a permissively-licensed alternative. That's docprims.

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

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

Subject to [3 Leaps OSS policies](https://github.com/3leaps/oss-policies).

## Related Projects

- [sysprims](https://github.com/3leaps/sysprims) - GPL-free process utilities (sibling project)
- [Gentry](https://github.com/fulmenhq/gentry) - Content protection scanner (primary consumer)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
