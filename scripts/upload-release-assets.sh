#!/usr/bin/env bash
# Upload signed release assets to GitHub
# Usage: upload-release-assets.sh <tag> [dir]
#
# Uploads checksum files, signatures, public keys, and release notes.
# Requires: gh CLI authenticated with write permissions
set -euo pipefail

TAG=${1:?"usage: upload-release-assets.sh <tag> [dir]"}
DIR=${2:-dist/release}

if [ ! -d "$DIR" ]; then
  echo "Error: Directory $DIR does not exist"
  exit 1
fi

cd "$DIR"

REQUIRED_FILES=(
  "SHA256SUMS"
  "SHA256SUMS.minisig"
  "SHA512SUMS"
  "SHA512SUMS.minisig"
  "docprims-minisign.pub"
)

for file in "${REQUIRED_FILES[@]}"; do
  if [ ! -f "$file" ]; then
    echo "Error: Required file missing: $file"
    exit 1
  fi
done

UPLOAD_FILES=(
  "SHA256SUMS"
  "SHA256SUMS.minisig"
  "SHA512SUMS"
  "SHA512SUMS.minisig"
  "docprims-minisign.pub"
)

for optional in "SHA256SUMS.asc" "SHA512SUMS.asc" "docprims-release-signing-key.asc"; do
  if [ -f "$optional" ]; then
    UPLOAD_FILES+=("$optional")
  fi
done

RELEASE_NOTES="release-notes-${TAG}.md"
if [ -f "$RELEASE_NOTES" ]; then
  UPLOAD_FILES+=("$RELEASE_NOTES")
fi

gh release upload "$TAG" "${UPLOAD_FILES[@]}" --clobber

if [ -f "$RELEASE_NOTES" ]; then
  gh release edit "$TAG" --notes-file "$RELEASE_NOTES"
fi

gh release edit "$TAG" --draft=false

echo "[ok] Release $TAG published"
