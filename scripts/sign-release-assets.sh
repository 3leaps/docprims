#!/usr/bin/env bash
# Sign release checksum manifests with minisign (and optionally PGP)
# Usage: sign-release-assets.sh <tag> [dir]
#
# Environment variables:
#   DOCPRIMS_MINISIGN_KEY  - Path to minisign secret key (required)
#   DOCPRIMS_PGP_KEY_ID    - PGP key ID for optional GPG signing (optional)
#   DOCPRIMS_GPG_HOMEDIR   - Custom GPG home directory (optional)
#
# Requires: minisign, optionally gpg
set -euo pipefail

TAG=${1:?"usage: sign-release-assets.sh <tag> [dir]"}
DIR=${2:-dist/release}

if [ ! -d "$DIR" ]; then
  echo "Error: Directory $DIR does not exist"
  exit 1
fi

if [ -z "${DOCPRIMS_MINISIGN_KEY:-}" ]; then
  echo "Error: DOCPRIMS_MINISIGN_KEY environment variable not set"
  echo "Set to path of your minisign secret key:"
  echo "  export DOCPRIMS_MINISIGN_KEY=/path/to/docprims.key"
  exit 1
fi

if [ ! -f "$DOCPRIMS_MINISIGN_KEY" ]; then
  echo "Error: Minisign key not found: $DOCPRIMS_MINISIGN_KEY"
  exit 1
fi

cd "$DIR"

echo "Signing release $TAG..."

for manifest in SHA256SUMS SHA512SUMS; do
  if [ -f "$manifest" ]; then
    echo "Signing $manifest with minisign..."
    minisign -S -s "$DOCPRIMS_MINISIGN_KEY" \
      -m "$manifest" \
      -t "docprims $TAG - $(date -u +%Y-%m-%dT%H:%M:%SZ)" \
      -x "${manifest}.minisig"
    echo "[ok] Created ${manifest}.minisig"
  fi
done

if [ -n "${DOCPRIMS_PGP_KEY_ID:-}" ]; then
  GPG_OPTS=()
  if [ -n "${DOCPRIMS_GPG_HOMEDIR:-}" ]; then
    GPG_OPTS+=("--homedir" "$DOCPRIMS_GPG_HOMEDIR")
  fi

  for manifest in SHA256SUMS SHA512SUMS; do
    if [ -f "$manifest" ]; then
      echo "Signing $manifest with PGP..."
      gpg "${GPG_OPTS[@]}" \
        --armor \
        --detach-sign \
        --local-user "$DOCPRIMS_PGP_KEY_ID" \
        --output "${manifest}.asc" \
        "$manifest"
      echo "[ok] Created ${manifest}.asc"
    fi
  done
else
  echo "[--] PGP signing skipped (DOCPRIMS_PGP_KEY_ID not set)"
fi

echo "[ok] Signing complete"
