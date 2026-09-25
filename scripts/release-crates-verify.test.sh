#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=release-crates-registry.sh
# shellcheck disable=SC1091
source "$SCRIPT_DIR/release-crates-registry.sh"
cargo() { [[ "$*" == 'info --registry crates-io docprims@0.2.0' ]]; }
curl() { printf '%s\n' '{"version":{"num":"0.2.0","crate":"docprims","yanked":false}}'; }
registry_wait docprims 0.2.0
registry_api_check docprims 0.2.0
if registry_api_check docprims 0.2.1 >/dev/null 2>&1; then
  echo 'error: mismatched version passed API check' >&2
  exit 1
fi
if registry_api_check docprims-core 0.2.0 >/dev/null 2>&1; then
  echo 'error: mismatched crate passed API check' >&2
  exit 1
fi
curl() { printf '%s\n' '{"version":{"num":"0.2.0","crate":"docprims","yanked":true}}'; }
if registry_api_check docprims 0.2.0 >/dev/null 2>&1; then
  echo 'error: yanked version passed API check' >&2
  exit 1
fi
cargo() { return 1; }
sleep() { :; }
if registry_wait docprims 0.2.0 >/dev/null 2>&1; then
  echo 'error: missing registry version passed index wait' >&2
  exit 1
fi
echo '[ok] registry index and API checks reject mismatched releases'
