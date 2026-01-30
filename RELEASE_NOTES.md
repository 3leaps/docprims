# Release Notes

> **Note:** This file aggregates the latest 3 releases in reverse chronological order.
> For the complete release history, see `CHANGELOG.md`.
> For detailed release documentation, see `docs/releases/`.

---

## v0.1.1 - 2026-01-29

**Status:** Patch Release (TypeScript bindings preview)

Adds first-cut TypeScript/Node.js bindings (Node-API via napi-rs) intended for validation from a git checkout. npm publishing will follow in v0.1.2 once the trusted OIDC publishing workflow lands.

### Highlights

- **TypeScript bindings**: `@3leaps/docprims` wrapper for `extractFile` / `extractBytes` (+ `*Json` variants)
- **Defensive limits**: `max_input_bytes` enforced for `extractBytes` inputs
- **CI coverage**: TypeScript tests on linux/macos/windows + Alpine/musl; release validation from a tag
- **Stable goldens**: CLI golden fixtures ignore `generator.version` across patch bumps
- **Go bindings (shared lib opt-in)**: `docprims_shared` build tag + vendored shared libraries to avoid Rust `staticlib` collisions

### TypeScript (from git checkout)

```bash
cd bindings/typescript/docprims
npm install
npm run test:ci
```

## v0.1.0 - 2026-01-28

**Status:** Initial Release

GPL-free document text extraction primitives for Rust with Go bindings. Extract text from DOCX, XLSX, PPTX, Markdown, HTML, and XML with provenance tracking and schema-validated output.

### Highlights

- **Schema-Validated Output**: All extractors emit `DocprimsExtract` JSON conforming to versioned schemas
- **Provenance Tracking**: Every text block includes locators back to source (paragraph index, cell ref, etc.)
- **Defensive Parsing**: Built for untrusted input with configurable resource limits
- **GPL-Free**: All dependencies permissively licensed (MIT/Apache-2.0)

### Supported Formats

| Format | Crate | Block Types |
|--------|-------|-------------|
| Markdown | `docprims-text` | heading, paragraph, code, list_item |
| HTML | `docprims-text` | block (stripped tags) |
| XML | `docprims-text` | text (element context) |
| DOCX | `docprims-ooxml` | paragraph, table_cell |
| XLSX | `docprims-ooxml` | cell |
| PPTX | `docprims-ooxml` | shape_text |

### CLI Quick Start

```bash
# Install
cargo install --path crates/docprims-cli

# Extract plain text
docprims extract document.docx

# Extract with structured blocks (JSON)
docprims extract document.md --format json --include-blocks

# Multi-file NDJSON
docprims extract *.docx --format json
```

### Go Bindings

```go
import "github.com/3leaps/docprims/bindings/go/docprims"

result, err := docprims.ExtractMarkdownV0("README.md", docprims.ExtractLimits{})
if err != nil {
    log.Fatal(err)
}
fmt.Println(result.Document.Text)
```

Prebuilt libraries available for:
- darwin-arm64 (macOS Apple Silicon)
- linux-amd64, linux-arm64 (glibc)
- linux-amd64-musl, linux-arm64-musl (Alpine)
- windows-amd64 (GNU toolchain)

### Output Schema

```json
{
  "schema_id": "https://schemas.3leaps.dev/docprims/extract/v0/docprims-extract.schema.json",
  "schema_version": "1.0.0",
  "generator": { "name": "docprims", "version": "0.1.0" },
  "source": {
    "uri": "./document.md",
    "format": { "family": "text", "kind": "markdown" },
    "sha256": "..."
  },
  "document": {
    "quality": { "status": "complete" },
    "text": "Full extracted text...",
    "blocks": [
      {
        "id": "markdown:heading:0",
        "kind": "markdown:heading",
        "text": "Title",
        "doc_text_range": { "start_byte": 0, "end_byte": 5 },
        "loc": { "kind": "markdown:locator", "hints": { "block_index": 0 } }
      }
    ]
  }
}
```

### Known Limitations

- PDF extraction not yet supported (planned for v0.2)
- OOXML table structure (rows/columns) not yet exposed in blocks
- Span annotations (bold/italic) planned for future release

### Dependencies

Core parsing libraries (all MIT/Apache-2.0):
- `quick-xml` - XML parsing
- `zip` - ZIP archive handling
- `pulldown-cmark` - Markdown parsing
- `scraper` - HTML parsing

### Upgrade Notes

This is the initial release. No upgrade path required.

---

*For older releases, see `docs/releases/`.*
