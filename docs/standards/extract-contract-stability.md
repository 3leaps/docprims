# extract/v0 Contract: Stability and Output Guarantees

This page states what consumers of docprims output can rely on: the stability
level of the `extract/v0` schema, the guarantees every result meets, and how
to treat the sensitivity of extracted text. It applies to the `docprims` crate,
the CLI's JSON output, and the C, Go and TypeScript bindings, which all return
the same structure.

## Schema stability: Evolving

`extract/v0` (`schemas/v0/extract/`) is classified **Evolving** under the
3leaps [Schema Stability Classification](https://github.com/3leaps/crucible/blob/main/docs/standards/schema-stability-classification.md)
standard.

| Aspect | What it means for `extract/v0` |
|--------|--------------------------------|
| Compatibility | Additive changes (new optional fields, new block kinds) keep existing consumers working |
| Breaking changes | Only with notice: a `CHANGELOG.md` entry and a new docprims minor version while docprims is 0.x |
| Versioning | `schema_version` in every result follows the bump rules in `docs/architecture/overview.md` |
| Consumer advice | Safe for development; track the changelog |

Changes to *what text is extracted* from a given document (for example, which
characters are kept) are contract changes and are announced the same way.

Stable status (full compatibility within a major version) is planned with
docprims 1.0. The Rust types behind the schema follow
[PDR-0001](../decisions/PDR-0001-crates-io-publication.md).

## Output guarantees

Every result meets these properties. Each is covered by tests across all
supported formats.

- **Character set.** Extracted text contains only XML 1.0 characters, excluding DEL and C1 controls (U+007F–U+009F); other characters are dropped. Markdown and HTML keep their parsers' U+FFFD replacement for NUL.
- **Parser choice.** The parser is chosen from the path or source URI
  extension, or from an explicit format. Document content is never inspected
  to choose a parser.
- **Byte ranges.** Each block's `doc_text_range` indexes the emitted
  `document.text`, and the text is the blocks joined in order by one
  separator each.
- **Limits.** `document.text` never exceeds `max_output_bytes`. When a limit
  truncates output, `quality.status` is `partial`, and no block is empty.

The character-set guarantee is not a display-safety claim. Bidirectional and
other format controls (for example U+202E) are legitimate content in
right-to-left text and are kept; presenting untrusted text safely is the
renderer's responsibility.

## Data sensitivity

docprims extracts text from documents it does not classify. Under the 3leaps
[Data Sensitivity Classification](https://github.com/3leaps/crucible/blob/main/docs/standards/data-sensitivity-classification.md)
standard:

- Extracted text and metadata carry the **same sensitivity as the input
  document**. Extraction never lowers it.
- Output from an unclassified document is **UNKNOWN** and is handled as such
  until the consumer classifies it.

`extract/v0` has no sensitivity field; consumers that store or forward results
carry the classification alongside them.
