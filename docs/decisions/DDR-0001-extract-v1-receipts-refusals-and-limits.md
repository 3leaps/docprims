# DDR-0001: Extract v1, Receipts, Refusals, and Limits Profiles

> **Status**: Proposed
> **Date**: 2026-11-09
> **Authors**: 3leaps Architecture Council

## Context

v0.2 adds a second implementation (the pure-Go twin, ADR-0007) and PDF
(SDR-0003). Both trees and all bindings must produce the same result for the
same bytes, and pipelines need to know not only what text came out but what
was skipped or refused.

crucible has no document-extraction contract today. It does have
`coverage-attestation/v0` (completeness claims with confirmed or inferred
basis, volume, and stable gap codes) and `data-artifact/v0` (descriptors for
produced artifacts).

The v0 goldens embed `source.uri` and `generator.version`, so they cannot be
compared across trees or checkouts.

## Decision

docprims defines four machine contracts in its own `schemas/` directory, with
`https://schemas.3leaps.dev/docprims/...` ids. They move to crucible as a
`document-extract` family once a second producer or consumer exists.

1. **`docprims-extract` v1** — the result. Promoted from v0:
   - adds `pdf:page` and `pdf:line` block kinds and a PDF locator (page index,
     object reference);
   - keeps ordered blocks with `doc_text_range` spans and `quality`;
   - conformance compares the `document` object only; `source` and
     `generator` are excluded from comparison;
   - output is canonical JSON (RFC 8785), via canonprims in both trees, so
     the trees compare byte for byte;
   - classified `experimental` under crucible's schema-stability standard
     until v1.0.0 of docprims.
2. **Refusal vocabulary** — a closed set of error kinds shared by both trees
   and all bindings: `Unsupported`, `Encrypted`, `LimitExceeded` (with the
   limit name), `Malformed` (with a closed sub-code list). New kinds require a
   schema revision.
3. **Extraction receipt** — one per call: input SHA-256, detected format,
   tree (`rust` or `go`) and version, limits profile id and revision, result
   quality, warnings, and the refusal if any. `sensitivity` is present and
   left at `unknown`, as crucible's data-sensitivity standard requires at
   ingestion boundaries; docprims does not classify content.
4. **Limits profile** — a named, versioned set of resource limits
   (SDR-0003, ADR-0003). The receipt names the profile applied, and doccorpus
   cases name the profile and the limit they expect to trip.

Per-page completeness reuses `coverage-attestation/v0` rather than a new
shape: one scope per page, `basis: confirmed` for pages read, and stable gap
codes `image_only`, `undecodable_font`, `limit_truncated` and
`encrypted_part`, with character counts as volume. A blank page and an
unreadable page are therefore distinguishable.

A short mapping note in `docs/architecture/` describes an extraction result
as a `data-artifact/v0` artifact (blocks as the grain, receipt and coverage
as provenance) for indexing and RAG pipelines. No new crucible contract is
proposed in v0.2.

## Consequences

- Cross-tree comparison is byte-exact and independent of paths.
- docprims depends on canonprims (Profile B), so `leaf: false`.
- Promotion to crucible later means a schema id change, handled with a
  documented alias period.

## Alternatives Considered

### Define the contracts in crucible now

Adds a third review surface before any second producer exists.

### Each tree writes its own deterministic JSON

Avoids the sibling dependency but duplicates canonicalization rules the
canonprims conformance suite already pins.
