#!/usr/bin/env bash
# Verify signatures on checksum manifests
# Usage: verify-signatures.sh [dir]
set -euo pipefail

DIR=${1:-dist/release}

if [ ! -d "$DIR" ]; then
  echo "Error: Directory $DIR does not exist"
  exit 1
fi

cd "$DIR"

ERRORS=0

if [ -f "docprims-minisign.pub" ]; then
  for manifest in SHA256SUMS SHA512SUMS; do
    if [ -f "$manifest" ] && [ -f "${manifest}.minisig" ]; then
      if minisign -Vm "$manifest" -p docprims-minisign.pub; then
        echo "[ok] $manifest minisign signature valid"
      else
        echo "[!!] $manifest minisign signature INVALID"
        ERRORS=$((ERRORS + 1))
      fi
    elif [ -f "$manifest" ]; then
      echo "[!!] Missing signature: ${manifest}.minisig"
      ERRORS=$((ERRORS + 1))
    fi
  done
else
  echo "[!!] docprims-minisign.pub not found - cannot verify minisign signatures"
  ERRORS=$((ERRORS + 1))
fi

if [ -f "docprims-release-signing-key.asc" ]; then
  GNUPGHOME=$(mktemp -d)
  export GNUPGHOME
  trap 'rm -rf "$GNUPGHOME"' EXIT

  gpg --import docprims-release-signing-key.asc 2>/dev/null
  for manifest in SHA256SUMS SHA512SUMS; do
    if [ -f "$manifest" ] && [ -f "${manifest}.asc" ]; then
      if gpg --verify "${manifest}.asc" "$manifest" 2>/dev/null; then
        echo "[ok] $manifest PGP signature valid"
      else
        echo "[!!] $manifest PGP signature INVALID"
        ERRORS=$((ERRORS + 1))
      fi
    fi
  done
fi

if [ $ERRORS -eq 0 ]; then
  echo "[ok] All signatures verified"
  exit 0
fi

echo "[!!] $ERRORS signature verification errors"
exit 1
