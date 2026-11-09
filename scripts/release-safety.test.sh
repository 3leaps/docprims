#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/docprims-release-safety.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT

expect_fail() {
  if "$@" >/dev/null 2>&1; then
    echo "expected failure: $*" >&2
    exit 1
  fi
}

root="$fixture/repo"
git init -q -b main "$root"
(
  cd "$root"
  expect_fail "$SCRIPT_DIR/release-clean.sh" /
  mkdir -p dist
  ln -s "$fixture/outside" dist/release
  expect_fail "$SCRIPT_DIR/release-clean.sh"
  [[ -L dist/release ]]
)

echo "[ok] release cleanup safety tests passed"
