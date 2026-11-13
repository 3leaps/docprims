# SDR-0003: PDF Scope, Resource Limits, and Refusals

> **Status**: Accepted
> **Date**: 2026-11-13
> **Authors**: devlead
> **Supervised by**: @3leapsdave

## Threat Model

### Assets

- Availability of the host process (memory, CPU, stack).
- Integrity of extracted text handed to downstream pipelines.
- The permissive licence posture of every shipped dependency closure.

### Threat Actors

Anyone who can supply a document: uploads, crawled content, email
attachments, adversarially crafted files.

### Attack Vectors

- Decompression bombs in Flate, LZW and other stream filters.
- Object graphs with cycles, deep nesting, or huge object counts.
- Cross-reference loops and broken xref tables that force repeated scans.
- Recursive Form XObjects and content streams with enormous operator counts.
- Malformed fonts, CMaps and encodings that produce unbounded output.
- Encrypted files designed to make the extractor guess passwords.

## Decision

### Scope (v0.2)

PDF support is **text extraction only**, in both trees. The Go tree builds it
first, as the operating tree during bootstrap (ADR-0007).

In scope:

- Classic xref tables, xref streams, object streams, incremental updates, and
  a bounded recovery scan when the xref is broken.
- Filters: FlateDecode, LZWDecode (own implementation; Go's `compress/lzw`
  lacks PDF's early-change mode), ASCII85Decode, ASCIIHexDecode,
  RunLengthDecode. Image filters (DCT, JBIG2, JPX, CCITT) are skipped, never
  decoded.
- Standard security handler (RC4, AES-128, AES-256) with an **empty user
  password only**.
- Text operators `BT`/`ET`, `Tj`, `TJ`, `'`, `"`, `Td`, `TD`, `Tm`, `T*`, `Tc`,
  `Tw`, `Tz`, `TL`, `Ts`, and Form XObjects.
- Standard, WinAnsi, MacRoman and PDFDoc encodings, `Differences`,
  `ToUnicode` CMaps, Identity-H/V, Type 3 fonts, and the Adobe Glyph List
  (licence confirmed in the dependency audit before use).
- Reading order v0: per page, glyphs grouped into lines by baseline and
  sorted left to right. Uncertain ordering sets result quality to `partial`.

Out of scope: OCR, rendering, tables, forms, annotations, digital signature
validation, and Adobe predefined CJK CMaps beyond Identity.

### Limits

These extend ADR-0003 and live in the versioned limits profile (DDR-0001).
Defaults are proposals until measured against the doccorpus stress tier.

| Limit                      | Applies to                                    |
| -------------------------- | --------------------------------------------- |
| `max_input_bytes`          | Raw document size                             |
| `max_objects`              | Indirect objects registered                   |
| `max_decoded_stream_bytes` | Bytes produced by one stream's filter chain   |
| `max_total_decoded_bytes`  | Bytes produced across all streams             |
| `max_decompression_ratio`  | Decoded to encoded bytes, per stream          |
| `max_content_operators`    | Operators executed across all content streams |
| `max_xobject_depth`        | Form XObject nesting                          |
| `max_object_depth`         | Nested arrays and dictionaries in one object  |
| `max_pages`                | Pages walked                                  |
| `max_output_bytes`         | Extracted text, checked while writing         |
| `max_recovery_scan_bytes`  | Bytes scanned when rebuilding a broken xref   |

Every limit is checked before the work it guards, with checked arithmetic.
Exceeding a limit returns `LimitExceeded` naming the limit; it never panics,
truncates silently, or loops.

### Refusals

Refusals use the closed vocabulary in DDR-0001: `Unsupported`, `Encrypted`
(any handler other than standard with empty user password), `LimitExceeded`,
and `Malformed`. Neither tree guesses passwords.

### Qualification

Fixtures come from the doccorpus repository: synthetic files whose expected
text is known from the generator, plus a public-domain supplement with a
licence recorded per case. Copyleft tools such as poppler `pdftotext` may be
used as non-normative comparison data in qualification scripts and never
enter a shipped dependency closure.

### Release gate

PDF ships in a release only when the operating tree passes the doccorpus
conformance and hostile tiers for PDF. A release never ships PDF on partial
qualification; if the gate is not met, that release ships without PDF.

## Consequences

- Hostile inputs fail closed with typed errors in both trees.
- Some real-world PDFs (scanned, CJK with predefined CMaps, password
  protected) produce refusals or `partial` quality in v0.2.
