---
title: "Go Bindings Distribution"
status: active
date: 2026-01-27
---

# Go Bindings Distribution

This standard defines how docprims Go bindings are distributed and how prebuilt artifacts are managed.

It is not a roadmap.

## Policy

- The Go module lives in a subdirectory: `bindings/go/docprims`.
- Go consumers must be able to `go get` the module without requiring a Rust toolchain.
- Therefore, prebuilt static libs **must** be present in-repo under:
  - `bindings/go/docprims/lib/<platform>/libdocprims_ffi.a`
- The generated C header is part of the binding surface and **must** be present under:
  - `bindings/go/docprims/include/docprims.h`

## Platform Matrix (Go)

Supported for Go (cgo):

- macOS arm64: `darwin-arm64`
- Linux x64: `linux-amd64`
- Linux x64 musl: `linux-amd64-musl` (selected via `-tags musl`)
- Linux arm64: `linux-arm64`
- Linux arm64 musl: `linux-arm64-musl` (selected via `-tags musl`)
- Windows x64: `windows-amd64` built for `x86_64-pc-windows-gnu` (MinGW)

Not supported:

- Windows arm64 for Go (MinGW/cgo toolchain limitation)

## Versioning and Tags

Go requires a path-prefixed tag for modules in subdirectories. For each release `vX.Y.Z`, create:

- `vX.Y.Z` (repo tag)
- `bindings/go/docprims/vX.Y.Z` (Go module tag)

Both tags must point at the same commit.

## Prebuilt Lib Update Process

1. Update `VERSION` in the repo.
2. Run the Go bindings prep workflow:
   - `.github/workflows/go-bindings-prep.yml`
3. Merge the resulting PR that updates `bindings/go/docprims/lib/<platform>/` and headers.
4. Only then create the release tags.

## References

- `docs/architecture/bindings.md`
