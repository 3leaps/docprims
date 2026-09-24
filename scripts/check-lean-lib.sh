#!/usr/bin/env bash
# Verify that library builds of the `docprims` crate pull only what their
# features need: no CLI dependencies without `cli`, and no dependencies of
# formats that are not enabled.
set -euo pipefail

cd "$(dirname "$0")/.."

fail=0

# check <label> <forbidden crate>... -- <cargo tree feature args>...
check() {
  local label="$1"
  shift
  local forbidden=()
  while [ "$1" != "--" ]; do
    forbidden+=("$1")
    shift
  done
  shift
  local raw tree
  if ! raw="$(cargo tree -p docprims -e normal --locked --prefix none --format '{p}' "$@")"; then
    echo "[!!] $label: cargo tree failed"
    fail=1
    return
  fi
  tree="$(awk '{print $1}' <<<"$raw" | sort -u)"
  if ! grep -qx docprims-core <<<"$tree"; then
    echo "[!!] $label: dependency tree looks empty or malformed"
    fail=1
    return
  fi
  local bad=0
  for crate in "${forbidden[@]}"; do
    if grep -qx "$crate" <<<"$tree"; then
      echo "[!!] $label: '$crate' is in the dependency tree"
      bad=1
    fi
  done
  if [ "$bad" -eq 0 ]; then
    echo "[ok] $label"
  else
    fail=1
  fi
}

# serde_json is not listed: the extract/v0 types serialize with it in every build.
CLI_DEPS=(clap tracing tracing-subscriber rsfulmen)

check "default features: no CLI dependencies" "${CLI_DEPS[@]}" --
check "text only: no OOXML or CLI dependencies" docprims-ooxml zip "${CLI_DEPS[@]}" -- \
  --no-default-features --features text
check "ooxml only: no text-format or CLI dependencies" docprims-text pulldown-cmark scraper "${CLI_DEPS[@]}" -- \
  --no-default-features --features ooxml
check "markdown only: no HTML, XML, OOXML or CLI dependencies" \
  scraper quick-xml docprims-ooxml zip "${CLI_DEPS[@]}" -- --no-default-features --features markdown

exit "$fail"
