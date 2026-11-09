#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
root="$(git rev-parse --show-toplevel)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/docprims-release-assets.XXXXXX")"
payload="$(mktemp -d "${TMPDIR:-/tmp}/docprims-release-payload.XXXXXX")"
trap 'rm -rf "$fixture" "$payload"' EXIT

version="$(cat "$root/VERSION")"
expected_platforms="$(cut -d' ' -f1 "$root/config/release/cli-platforms.txt" | LC_ALL=C sort)"
workflow_platforms="$(awk '$1 == "platform:" { print $2 }' "$root/.github/workflows/release.yml" | LC_ALL=C sort)"
[[ "$expected_platforms" == "$workflow_platforms" ]] || {
  echo 'error: CLI release matrix differs from platform inventory' >&2
  exit 1
}
[[ "$version" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]] || {
  echo "error: fixture requires a stable VERSION" >&2
  exit 1
}
export DOCPRIMS_RELEASE_TAG="v${version}"

expect_fail() {
  if "$@" >/dev/null 2>&1; then
    echo "expected failure: $*" >&2
    exit 1
  fi
}

printf 'license\n' >"$fixture/LICENSE-APACHE"
printf 'license\n' >"$fixture/LICENSE-MIT"
printf '{}\n' >"$fixture/sbom-${version}.cdx.json"
printf 'header\n' >"$fixture/docprims.h"

# make_archive PLATFORM FORMAT BINARY [MEMBER...]: build an archive from
# regular files named MEMBER (default: the binary and both licenses).
make_archive() {
  local platform="$1" format="$2" binary="$3"
  shift 3
  local members=("$@")
  [[ ${#members[@]} -gt 0 ]] || members=("$binary" LICENSE-APACHE LICENSE-MIT)
  local archive="$fixture/docprims-${version}-${platform}.${format}"
  rm -rf "$payload" "$archive"
  mkdir -p "$payload"
  local member
  for member in "${members[@]}"; do
    mkdir -p "$payload/$(dirname "$member")"
    printf 'fixture\n' >"$payload/$member"
  done
  python3 - "$archive" "$format" "$payload" "${members[@]}" <<'PY'
import sys
import tarfile
import zipfile

archive, archive_format, payload, *members = sys.argv[1:]
if archive_format == "zip":
    with zipfile.ZipFile(archive, "w") as out:
        for m in members:
            out.write(f"{payload}/{m}", m)
else:
    with tarfile.open(archive, "w:gz") as out:
        for m in members:
            out.add(f"{payload}/{m}", m)
PY
}

good_archives() {
  local platform format binary
  while read -r platform format binary; do
    make_archive "$platform" "$format" "$binary"
  done <"$root/config/release/cli-platforms.txt"
}

good_archives
"$SCRIPT_DIR/validate-release-assets.sh" "$fixture" base >/dev/null
wrong_tag="v${version%.*}.$((${version##*.} + 1))"
expect_fail env DOCPRIMS_RELEASE_TAG="$wrong_tag" \
  "$SCRIPT_DIR/validate-release-assets.sh" "$fixture" base

printf 'stale\n' >"$fixture/foreign.txt"
expect_fail "$SCRIPT_DIR/validate-release-assets.sh" "$fixture" base
rm "$fixture/foreign.txt"
rm "$fixture/docprims.h"
expect_fail "$SCRIPT_DIR/validate-release-assets.sh" "$fixture" base
printf 'header\n' >"$fixture/docprims.h"

# Each malformed archive fails on its own against an otherwise good set.
reject_archive() {
  good_archives
  "$@"
  expect_fail "$SCRIPT_DIR/validate-release-assets.sh" "$fixture" base
}
reject_archive make_archive linux-amd64 tar.gz docprims docprims LICENSE-MIT
reject_archive make_archive linux-amd64 tar.gz docprims docprims LICENSE-APACHE LICENSE-MIT extra
reject_archive make_archive linux-amd64 tar.gz docprims docprims.exe LICENSE-APACHE LICENSE-MIT
reject_archive make_archive linux-amd64 tar.gz docprims bin/docprims LICENSE-APACHE LICENSE-MIT
reject_archive make_archive windows-amd64 zip docprims.exe docprims LICENSE-APACHE LICENSE-MIT
reject_archive make_archive windows-amd64 zip docprims.exe sub/docprims.exe LICENSE-APACHE LICENSE-MIT

# The layout `tar -C dir .` produces: a ./ directory and ./-prefixed names.
good_archives
rm -rf "$payload" && mkdir -p "$payload"
printf 'fixture\n' | tee "$payload/docprims" "$payload/LICENSE-APACHE" "$payload/LICENSE-MIT" >/dev/null
tar -czf "$fixture/docprims-${version}-darwin-arm64.tar.gz" -C "$payload" .
expect_fail "$SCRIPT_DIR/validate-release-assets.sh" "$fixture" base

python3 - "$fixture" "$version" <<'PY'
import io
import sys
import tarfile
import zipfile

fixture, version = sys.argv[1:]
def tar_with(path, special):
    with tarfile.open(path, "w:gz") as out:
        for name in ("docprims", "LICENSE-APACHE", "LICENSE-MIT"):
            info = tarfile.TarInfo(name)
            if name == special[0]:
                special[1](info)
            data = b"fixture\n" if info.isreg() else b""
            info.size = len(data)
            out.addfile(info, io.BytesIO(data))
def symlink(info):
    info.type = tarfile.SYMTYPE
    info.linkname = "/etc/passwd"
def escape(info):
    info.name = "../docprims"
tar_with(f"{fixture}/symlink.tar.gz", ("docprims", symlink))
tar_with(f"{fixture}/escape.tar.gz", ("docprims", escape))
with zipfile.ZipFile(f"{fixture}/symlink.zip", "w") as out:
    link = zipfile.ZipInfo("docprims.exe")
    link.external_attr = (0o120777 << 16)
    out.writestr(link, "target")
    out.writestr("LICENSE-APACHE", "fixture\n")
    out.writestr("LICENSE-MIT", "fixture\n")
with zipfile.ZipFile(f"{fixture}/dir.zip", "w") as out:
    out.writestr("docprims.exe", "fixture\n")
    out.writestr("LICENSE-APACHE", "fixture\n")
    out.writestr("LICENSE-MIT", "fixture\n")
    out.writestr(zipfile.ZipInfo("docs/"), "")
PY
for bad in symlink.tar.gz:linux-arm64.tar.gz escape.tar.gz:linux-arm64-musl.tar.gz symlink.zip:windows-amd64.zip dir.zip:windows-amd64.zip; do
  good_archives
  cp "$fixture/${bad%%:*}" "$fixture/docprims-${version}-${bad#*:}"
  mkdir -p "$fixture.hold" && mv "$fixture"/symlink.* "$fixture"/escape.* "$fixture"/dir.* "$fixture.hold/"
  expect_fail "$SCRIPT_DIR/validate-release-assets.sh" "$fixture" base
  mv "$fixture.hold"/* "$fixture/" && rmdir "$fixture.hold"
done
rm -f "$fixture"/symlink.* "$fixture"/escape.* "$fixture"/dir.*

good_archives
printf 'notes\n' >"$fixture/release-notes-${DOCPRIMS_RELEASE_TAG}.md"
printf 'gpg fixture\nminisign fixture\n' >"$fixture/expected-fingerprints.txt"
printf '{"fixture":true}\n' >"$fixture/expected-fingerprints.ndjson"
# shellcheck source=release-common.sh
# shellcheck disable=SC1091
source "$SCRIPT_DIR/release-common.sh"
signable_assets="$(release_signable_assets)"
(
  cd "$fixture"
  printf '%s\n' "$signable_assets" | LC_ALL=C sort | xargs shasum -a 256 >SHA256SUMS
  printf '%s\n' "$signable_assets" | LC_ALL=C sort | xargs shasum -a 512 >SHA512SUMS
)
"$SCRIPT_DIR/verify-checksums.sh" "$fixture" >/dev/null
cp "$fixture/SHA256SUMS" "$fixture/SHA256SUMS.good"
printf '%s\n' "$(head -n 1 "$fixture/SHA256SUMS")" >>"$fixture/SHA256SUMS"
expect_fail "$SCRIPT_DIR/verify-checksums.sh" "$fixture"
mv "$fixture/SHA256SUMS.good" "$fixture/SHA256SUMS"
sed '1s#  #  nested/#' "$fixture/SHA256SUMS" >"$fixture/SHA256SUMS.bad"
mv "$fixture/SHA256SUMS.bad" "$fixture/SHA256SUMS"
expect_fail "$SCRIPT_DIR/verify-checksums.sh" "$fixture"

echo "[ok] exact release asset and archive tests passed"
