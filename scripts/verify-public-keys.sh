#!/usr/bin/env bash
# Verify that exported keys contain only public material (no secrets)
# Usage: verify-public-keys.sh [dir]
set -euo pipefail

DIR=${1:-dist/release}

if [ ! -d "$DIR" ]; then
  echo "Error: Directory $DIR does not exist"
  exit 1
fi

cd "$DIR"

ERRORS=0

if [ -f "docprims-minisign.pub" ]; then
  if grep -qi "secret" "docprims-minisign.pub"; then
    echo "[!!] DANGER: docprims-minisign.pub may contain secret key material!"
    ERRORS=$((ERRORS + 1))
  elif grep -q "^untrusted comment:" "docprims-minisign.pub"; then
    echo "[ok] docprims-minisign.pub appears to be a valid public key"
  else
    echo "[!!] docprims-minisign.pub has unexpected format"
    ERRORS=$((ERRORS + 1))
  fi
else
  echo "[--] docprims-minisign.pub not found"
fi

if [ -f "docprims-release-signing-key.asc" ]; then
  if grep -q "PRIVATE KEY BLOCK" "docprims-release-signing-key.asc"; then
    echo "[!!] DANGER: docprims-release-signing-key.asc contains PRIVATE KEY!"
    ERRORS=$((ERRORS + 1))
  elif grep -q "PUBLIC KEY BLOCK" "docprims-release-signing-key.asc"; then
    echo "[ok] docprims-release-signing-key.asc is a public key"
  else
    echo "[!!] docprims-release-signing-key.asc has unexpected format"
    ERRORS=$((ERRORS + 1))
  fi
fi

if [ $ERRORS -eq 0 ]; then
  echo "[ok] Public key verification passed"
  exit 0
fi

echo "[!!] CRITICAL: Found $ERRORS potential secret key exposures!"
exit 1
