#!/usr/bin/env bash
# Export public signing keys to release directory
# Usage: export-release-keys.sh [dir]
#
# Environment variables:
#   DOCPRIMS_MINISIGN_PUB  - Path to minisign public key (or derives from DOCPRIMS_MINISIGN_KEY)
#   DOCPRIMS_MINISIGN_KEY  - Path to minisign secret key (used to derive public key location)
#   DOCPRIMS_PGP_KEY_ID    - PGP key ID for optional export (optional)
#   DOCPRIMS_GPG_HOMEDIR   - Custom GPG home directory (optional)
set -euo pipefail

DIR=${1:-dist/release}

if [ ! -d "$DIR" ]; then
  echo "Error: Directory $DIR does not exist"
  exit 1
fi

echo "Exporting public keys to $DIR..."

MINISIGN_PUB="${DOCPRIMS_MINISIGN_PUB:-}"
if [ -z "$MINISIGN_PUB" ] && [ -n "${DOCPRIMS_MINISIGN_KEY:-}" ]; then
  MINISIGN_PUB="${DOCPRIMS_MINISIGN_KEY%.key}.pub"
fi

if [ -n "$MINISIGN_PUB" ] && [ -f "$MINISIGN_PUB" ]; then
  cp "$MINISIGN_PUB" "$DIR/docprims-minisign.pub"
  echo "[ok] Exported $DIR/docprims-minisign.pub"
else
  echo "[!!] Minisign public key not found"
  echo "Set DOCPRIMS_MINISIGN_PUB or ensure .pub file exists alongside .key"
fi

if [ -n "${DOCPRIMS_PGP_KEY_ID:-}" ]; then
  GPG_OPTS=()
  if [ -n "${DOCPRIMS_GPG_HOMEDIR:-}" ]; then
    GPG_OPTS+=("--homedir" "$DOCPRIMS_GPG_HOMEDIR")
  fi

  gpg "${GPG_OPTS[@]}" --armor --export "$DOCPRIMS_PGP_KEY_ID" >"$DIR/docprims-release-signing-key.asc"
  echo "[ok] Exported $DIR/docprims-release-signing-key.asc"
else
  echo "[--] PGP key export skipped (DOCPRIMS_PGP_KEY_ID not set)"
fi
