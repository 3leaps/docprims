# ADR-0001: License Policy

> **Status**: Accepted
> **Date**: 2025-01-25
> **Authors**: 3leaps Architecture Council

## Context

docprims is a document text extraction library. Many existing libraries in this space (poppler, mupdf, pdf.js internals) are GPL or AGPL licensed, which creates license contamination for commercial software that statically links or embeds them.

We need a clear license policy that ensures docprims can be safely used in commercial and proprietary software.

## Decision

1. **Project License**: MIT OR Apache-2.0 dual license (user's choice)

2. **Dependency Policy**: Only allow dependencies with permissive licenses:
   - MIT
   - Apache-2.0 (with or without LLVM exception)
   - BSD-2-Clause, BSD-3-Clause
   - ISC
   - Zlib
   - CC0-1.0
   - Unicode-DFS-2016
   - MPL-2.0 (weak copyleft, acceptable for linking)

3. **Explicitly Forbidden**:
   - GPL (any version)
   - LGPL (any version)
   - AGPL (any version)
   - Any other copyleft license

4. **Enforcement**: Use `cargo-deny` in CI to automatically reject forbidden licenses.

## Consequences

### Positive

- Safe for commercial embedding and static linking
- Clear value proposition vs GPL alternatives
- Automated enforcement prevents accidental violations

### Negative

- Cannot use some well-tested libraries (poppler, mupdf)
- May need to implement some functionality from scratch
- Smaller ecosystem of compatible dependencies

### Neutral

- Need to maintain `deny.toml` configuration
- Must review each new dependency for license compliance

## Alternatives Considered

### Alternative 1: LGPL with static linking exception

Could use LGPL libraries if we only dynamically link. Rejected because:
- Complicates distribution
- Some platforms don't support dynamic linking well
- Users expect simple embedding

### Alternative 2: MIT only

Could require MIT-only dependencies. Rejected because:
- Unnecessarily restrictive
- Excludes many good Apache-2.0 libraries
- No practical benefit over dual-license approach

## References

- [cargo-deny documentation](https://embarkstudios.github.io/cargo-deny/)
- [SPDX License List](https://spdx.org/licenses/)
- [sysprims ADR-0001](https://github.com/3leaps/sysprims/blob/main/docs/architecture/adr/0001-license-policy.md)
