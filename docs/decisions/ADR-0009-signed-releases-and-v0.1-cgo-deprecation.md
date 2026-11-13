# ADR-0009: Signed Releases, Registry Publishing, and v0.1 cgo Deprecation

> **Status**: Accepted
> **Date**: 2026-11-13
> **Authors**: devlead
> **Supervised by**: @3leapsdave

## Context

docprims v0.1 shipped prebuilt libraries committed to git and published only
to npm. The v0.2 artifacts (ADR-0007) are a Rust crate and CLI, a pure-Go
package, a WASM npm package and a Python wheel. Each must be traceable to a
signed tag, in the same way recent 3leaps prims (for example yamlprims) are.

The nested cgo Go module `github.com/3leaps/docprims/bindings/go/docprims` has
tags through v0.1.4, and the repository has VERSION 0.1.5 prepared but not
tagged. Its users are, as far as we know, inside the 3leaps estate.

## Decision

1. **Signed tags.** Each release tag `vX.Y.Z` is annotated and signed under the
   3leaps release identity. One tag versions every tree and binding
   (repository-classes C6, C7).
2. **Signed checksums.** Each release carries `SHA256SUMS` and `SHA512SUMS`,
   each signed with Minisign and OpenPGP, a CycloneDX SBOM, and the public
   keys needed to verify them.
3. **Estate signing key.** docprims uses the same Minisign and OpenPGP keys as
   recent 3leaps repositories rather than a docprims-specific key. Their
   expected fingerprints are pinned in `keys/expected-fingerprints.txt` and
   `keys/expected-fingerprints.ndjson`. Release verification checks the
   published public keys against these pins before trusting a signature.
4. **Registries.** Rust (`docprims`, `docprims-cli`) on crates.io,
   `@3leaps/docprims` on npm, `docprims` on PyPI, and the Go package through
   the Go module proxy. v0.2.0 publishes to crates.io, npm and PyPI with a
   maintainer token, because trusted publishing for a new crates.io crate can
   only be configured once the crate exists. From v0.2.1, publishing uses OIDC
   trusted publishing with no long-lived tokens, as ADR-0006 already does for
   npm.
5. **v0.1 cgo deprecation, kept light.** v0.1.5 is tagged by the maintainer
   as the final release of the cgo Go module and the napi-rs npm package. Its
   release notes point to v0.2 and the pure-Go `extract` package. v0.2 removes
   the nested module (ADR-0007). There is no compatibility shim or migration
   tool.

## Consequences

### Positive

- Every v0.2 artifact can be verified against a signed tag and pinned keys.
- One estate key means one set of fingerprints for consumers to trust.

### Negative

- v0.2.0 relies on a maintainer token once.
- A compromise of the estate key affects every repository that uses it.

### Neutral

- ADR-0006's OIDC configuration for npm carries forward. Its napi-rs prebuild
  packaging is retired with the addon (ADR-0007).

## Alternatives Considered

### A docprims-specific Minisign key

Isolates the blast radius of a key compromise, but adds another key for
consumers to pin while the estate is small.

### Skip v0.1.5

Leaves Go users of the cgo module without a final release that points to v0.2.

### OIDC from v0.2.0

Blocked by crates.io, which needs the crate to exist before a trusted
publisher can be configured.

## References

- crucible `docs/standards/repository-classes.md` C6, C7
- [ADR-0006](ADR-0006-typescript-npm-publishing.md),
  [ADR-0007](ADR-0007-profile-b-go-twin-and-wasm-pyo3-bindings.md)
