---
title: "Release Prep SOP"
status: draft
date: 2026-01-27
---

# Release Prep SOP

This SOP describes the minimum steps to prepare a docprims release so that bindings and artifacts are coherent.

## Steps

1. Ensure `make check` passes.
2. Ensure golden tests pass (`cargo test --workspace`).
3. Run Go bindings prep (updates vendored libs and headers):
   - `.github/workflows/go-bindings-prep.yml`
4. Merge the Go bindings PR.
5. Tag the release:
   - `vX.Y.Z`
   - `bindings/go/docprims/vX.Y.Z`
6. Run validate-release (when present) against the draft release artifacts.

## Notes

- Do not publish a release tag that does not include the vendored Go libs in the tag commit.
- Treat Windows arm64 support as TypeScript/CLI-first; Go is Windows x64 only.
