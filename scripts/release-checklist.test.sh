#!/usr/bin/env bash
# Every make target the release checklist names must exist in the Makefile.
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
missing=0
while IFS= read -r target; do
  grep -Eq "^${target}:" "$root/Makefile" || {
    echo "error: RELEASE_CHECKLIST.md names missing target: make $target" >&2
    missing=1
  }
done < <(grep -oE 'make [a-z][a-z0-9-]*' "$root/RELEASE_CHECKLIST.md" | awk '{ print $2 }' | sort -u)
[[ "$missing" -eq 0 ]]
echo '[ok] release checklist names only existing make targets'
