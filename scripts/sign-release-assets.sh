#!/usr/bin/env bash
# Sign exact checksum manifests with minisign and optional PGP.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=release-common.sh
# shellcheck disable=SC1091
source "$SCRIPT_DIR/release-common.sh"

directory="${1:-dist/release}"
require_release_guard 1 >/dev/null
tag="$(release_tag)"
"$SCRIPT_DIR/verify-checksums.sh" "$directory" >/dev/null
"$SCRIPT_DIR/validate-release-assets.sh" "$directory" checksummed >/dev/null

: "${DOCPRIMS_MINISIGN_KEY:?load the approved minisign secret key}"
: "${DOCPRIMS_MINISIGN_PUB:?load the approved minisign public key}"
for key_file in "$DOCPRIMS_MINISIGN_KEY" "$DOCPRIMS_MINISIGN_PUB"; do
  [[ -f "$key_file" && ! -L "$key_file" ]] || {
    echo "error: configured minisign material is not a regular file" >&2
    exit 1
  }
done

root="$(release_repo_root)"
secret_parent="$(cd "$(dirname "$DOCPRIMS_MINISIGN_KEY")" && pwd -P)"
secret_real="$secret_parent/$(basename "$DOCPRIMS_MINISIGN_KEY")"
case "$secret_real" in
"$root"/*)
  echo "error: minisign secret key must be outside the repository" >&2
  exit 1
  ;;
esac

if [[ -n "${DOCPRIMS_PGP_KEY_ID:-}" || -n "${DOCPRIMS_GPG_HOMEDIR:-}" ]]; then
  require_complete_pgp_config
  [[ -d "$DOCPRIMS_GPG_HOMEDIR" ]] || {
    echo "error: configured GPG home is unavailable" >&2
    exit 1
  }
fi

for manifest in SHA256SUMS SHA512SUMS; do
  minisign -S -s "$DOCPRIMS_MINISIGN_KEY" \
    -m "$directory/$manifest" \
    -t "docprims $tag" \
    -x "$directory/${manifest}.minisig"
  if [[ -n "${DOCPRIMS_PGP_KEY_ID:-}" ]]; then
    gpg --homedir "$DOCPRIMS_GPG_HOMEDIR" \
      --batch --armor --detach-sign \
      --local-user "$DOCPRIMS_PGP_KEY_ID" \
      --output "$directory/${manifest}.asc" \
      "$directory/$manifest"
  fi
done
"$SCRIPT_DIR/validate-release-assets.sh" \
  "$directory" signed-without-keys >/dev/null
echo "[ok] checksum manifests signed"
