#!/usr/bin/env bash
set -euo pipefail

# Build and optionally vendor the Go/cgo static library for the current host.
#
# This script is intentionally simple and host-oriented. For cross-platform vendoring,
# prefer CI workflows that run on the target runner.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

cd "$ROOT_DIR"

UNAME_S="$(uname -s)"
UNAME_M="$(uname -m)"

case "$UNAME_S/$UNAME_M" in
Darwin/arm64)
  PLATFORM="darwin-arm64"
  ;;
Darwin/x86_64)
  PLATFORM="darwin-amd64"
  ;;
Linux/x86_64)
  PLATFORM="linux-amd64"
  ;;
Linux/aarch64)
  PLATFORM="linux-arm64"
  ;;
*)
  echo "unsupported host platform: $UNAME_S/$UNAME_M" >&2
  exit 2
  ;;
esac

echo "[docprims] building FFI static lib for $PLATFORM"

make cbindgen
cargo build --release -p docprims-ffi

mkdir -p "bindings/go/docprims/include"
cp "ffi/docprims-ffi/docprims.h" "bindings/go/docprims/include/docprims.h"

mkdir -p "bindings/go/docprims/lib/$PLATFORM"
cp "target/release/libdocprims_ffi.a" "bindings/go/docprims/lib/$PLATFORM/libdocprims_ffi.a"

echo "[ok] wrote bindings/go/docprims/lib/$PLATFORM/libdocprims_ffi.a"
