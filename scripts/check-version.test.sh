#!/usr/bin/env bash
# Prove check-version.sh rejects drift at every version site it owns.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(dirname "$SCRIPT_DIR")"
work="$(mktemp -d "${TMPDIR:-/tmp}/docprims-version-test.XXXXXX")"
trap 'rm -rf "$work"' EXIT

fresh_copy() {
  rm -rf "$work/repo"
  mkdir "$work/repo"
  (cd "$root" && git ls-files -z --cached --others --exclude-standard) |
    (cd "$root" && xargs -0 tar -cf - --) | tar -xf - -C "$work/repo"
}

fresh_copy
"$work/repo/scripts/check-version.sh" >/dev/null

npm="bindings/typescript/docprims"
for mutation in \
  "VERSION|s/^.*$/9.9.9/" \
  "Cargo.toml|s/^docprims-text = { version = \"[^\"]*\"/docprims-text = { version = \"9.9.9\"/" \
  "Cargo.toml|s/^docprims = { version = \"[^\"]*\"/docprims = { version = \"9.9.9\"/" \
  "crates/docprims-ooxml/Cargo.toml|s/^version.workspace = true/version = \"9.9.9\"/" \
  "$npm/native/Cargo.toml|s/^version.workspace = true/version = \"9.9.9\"/" \
  "$npm/package.json|1,/\"version\": \"[^\"]*\"/s/\"version\": \"[^\"]*\"/\"version\": \"9.9.9\"/" \
  "$npm/package-lock.json|1,/\"version\": \"[^\"]*\"/s/\"version\": \"[^\"]*\"/\"version\": \"9.9.9\"/" \
  "$npm/package.json|s/\"devDependencies\": {/\"optionalDependencies\": {\"@3leaps\/docprims-linux-x64-gnu\": \"0.0.0\"}, \"devDependencies\": {/"; do
  file="${mutation%%|*}"
  expr="${mutation#*|}"
  fresh_copy
  before="$(cat "$work/repo/$file")"
  sed -i.bak -e "$expr" "$work/repo/$file"
  rm -f "$work/repo/$file.bak"
  if [[ "$before" == "$(cat "$work/repo/$file")" ]]; then
    echo "error: mutation did not apply: $mutation" >&2
    exit 1
  fi
  if "$work/repo/scripts/check-version.sh" >/dev/null 2>&1; then
    echo "error: version drift passed the check: $mutation" >&2
    exit 1
  fi
done

# version-sync.py writes every site the check reads.
fresh_copy
printf '9.9.9\n' >"$work/repo/VERSION"
if "$work/repo/scripts/check-version.sh" >/dev/null 2>&1; then
  echo 'error: VERSION-only bump passed the check' >&2
  exit 1
fi
"$work/repo/scripts/version-sync.py" >/dev/null
"$work/repo/scripts/check-version.sh" >/dev/null
grep -q '^version = "9.9.9"' "$work/repo/Cargo.lock" || {
  echo 'error: version-sync did not update Cargo.lock' >&2
  exit 1
}
echo '[ok] version check rejects drift at every version site; version-sync fixes it'
