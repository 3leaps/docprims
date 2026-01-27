---
status: accepted
date: 2026-01-27
deciders: ["@3leapsdave"]
roles: ["devlead"]
---

# SDR-0002: OOXML v0 Block Granularity

## Context

The v0 extraction contract (`schemas/v0/extract/*.json`) requires a stable sequence of blocks,
byte-range anchors into `document.text`, and provenance locators.

For OOXML formats, there is a natural tension between:

- fine-grained blocks (XLSX cell-level, PPTX shape-level) that provide very precise provenance
- coarse-grained blocks (row/paragraph-level) that reduce block counts, simplify joining rules,
  and are cheaper to compute under resource limits

Because docprims parses untrusted input, default behavior must be safe under adversarial inputs
and predictable for downstream consumers.

## Decision

For v0, docprims emits coarse-grained OOXML blocks:

- DOCX: `docx:paragraph`
- XLSX: `xlsx:row` (tab-joined cells per row)
- PPTX: `pptx:paragraph`

Locators remain schema-conformant and format-specific:

- `docx:locator` with `hints.paragraph_index`
- `xlsx:locator` with `hints.sheet_name`, `hints.sheet_index`, optional `hints.row_index`
- `pptx:locator` with `hints.slide_index`, `hints.paragraph_index`

## Rationale

- **Resource safety**: coarse blocks reduce `max_blocks` pressure and avoid creating a block per
  cell/shape in worst-case files.
- **Determinism**: row/paragraph ordering is well-defined and stable given relationship-driven
  part enumeration.
- **Consumer utility**: most downstream matching uses byte ranges over joined text; row/paragraph
  blocks provide sufficient provenance anchors for v0.

## Consequences

- XLSX and PPTX provenance is less precise than cell/shape-level.
- Future versions may add optional finer-grained block kinds (`xlsx:cell`, `pptx:shape_text`) or
  nested blocks via `children`, but v0 keeps a single primary granularity to preserve simplicity.

## Follow-ups

- If downstream consumers require cell/shape precision, propose an SDR/ADR to add optional finer
  blocks and document the performance/limit impacts.
