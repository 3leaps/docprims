# ADR-0008: canonprims as a Sibling Prim Dependency

> **Status**: Accepted
> **Date**: 2026-11-13
> **Authors**: devlead
> **Supervised by**: @3leapsdave

## Context

The two docprims trees must produce byte-identical results for the same input
(ADR-0007, DDR-0001). That needs one canonical JSON serialisation. Each tree
could carry its own deterministic writer with a key-order rule, or both could
use canonprims, the estate's Profile B prim for RFC 8785 (JSON
Canonicalization Scheme) output, whose conformance suite already pins the
canonicalisation rules.

crucible ADR-0018 §3 allows a prim to depend on a sibling prim under four
rules, checked by repository-classes P7. docprims would be the first prim to
use that allowance.

## Decision

1. **Depend on canonprims now**, in both trees, for canonical output of every
   machine contract in DDR-0001: the Rust crate `canonprims` and its pure-Go
   package. canonprims is Profile B, so the Go tree stays cgo-free.
2. **Not a leaf.** `.fulmen/repo.yaml` declares `leaf: false` and allow-lists
   canonprims per tree. The dependency check fails on any other prim import.
3. **P7 obligations.**
   - **Pin.** canonprims is pinned at a tagged release, and is public, by
     docprims' first public v0.2 release. An exact-commit pin is allowed
     before then.
   - **No leakage.** No canonprims type or package path appears in docprims'
     public API, contract or conformance vectors. Vectors may hold byte-level
     expected output, which is contract, not a type. CI runs an exported-API
     scan per tree: `go doc -all` or `apidiff` for Go, `cargo public-api` for
     Rust.
   - **cgo-free Go.** The Go tree depends only on Go modules that need no cgo.
   - **No cycle.** canonprims does not depend on docprims.
4. **No reimplementation.** docprims does not reimplement RFC 8785 in either
   tree to avoid the dependency.

## Consequences

### Positive

- Cross-tree byte identity rests on rules a separate conformance suite already
  pins.
- docprims is a worked example of prim-on-prim composition for the
  repository-classes standard.

### Negative

- A canonprims release can gate a docprims release.
- docprims inherits canonprims' dependency closure and its release cadence.

### Neutral

- The Go tree's dependency allow-list is the standard library plus canonprims
  until v0.3 adds HTML and Markdown parsers (ADR-0007).

## Alternatives Considered

### Each tree writes its own deterministic JSON

Avoids the sibling dependency, but duplicates canonicalisation rules in two
languages and tests them a second time.

### Defer the dependency to a later release

Leaves two writers to retire later and delays the first live run of the P7
checks.

## References

- crucible ADR-0018 §3 and `docs/standards/repository-classes.md` P7, §4
- RFC 8785, JSON Canonicalization Scheme
- [ADR-0007](ADR-0007-profile-b-go-twin-and-wasm-pyo3-bindings.md),
  [DDR-0001](DDR-0001-extract-v1-receipts-refusals-and-limits.md)
