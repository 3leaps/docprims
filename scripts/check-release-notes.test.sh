#!/usr/bin/env bash
# Prove the per-cut notes check accepts only an exact RELEASE_NOTES section.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/docprims-notes-test.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/docs/releases"

section=$'## v1.2.0 — 2026-01-02\n\nBody line.\n\n### Detail\n\n- item'
write_fixture() {
  printf '# Release Notes\n\n---\n\n%s\n\n---\n\n## v1.1.0 — 2026-01-01\n\nOld.\n' "$1" >"$fixture/RELEASE_NOTES.md"
  printf '%s\n' "$2" >"$fixture/docs/releases/v1.2.0.md"
}
check() { (cd "$fixture" && "$SCRIPT_DIR/check-release-notes.sh" "$@") >/dev/null 2>&1; }
expect_fail() {
  if check "$@"; then
    echo "error: notes check passed unexpectedly: $*" >&2
    exit 1
  fi
}

write_fixture "$section" "$section"
check v1.2.0 || {
  echo "error: matching section was rejected" >&2
  exit 1
}
write_fixture "$section" "${section} changed"
expect_fail v1.2.0
write_fixture "$section" "${section%$'\n- item'}"
expect_fail v1.2.0
write_fixture "${section/2026-01-02/2026-01-03}" "$section"
expect_fail v1.2.0
write_fixture "$section" "$section"
expect_fail v1.1.0
expect_fail v1.3.0
expect_fail 1.2.0
rm "$fixture/docs/releases/v1.2.0.md"
expect_fail v1.2.0
echo '[ok] per-cut notes check accepts only the exact section'
