#!/usr/bin/env bash
# Shared release asset and repository invariants.

set -euo pipefail

DOCPRIMS_REPOSITORY="3leaps/docprims"

release_repo_root() {
  git rev-parse --show-toplevel
}

release_version() {
  local root
  root="$(release_repo_root)"
  local version
  version="$(cat "$root/VERSION")"
  if [[ ! "$version" =~ ^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]; then
    echo "error: VERSION must contain one stable semantic version" >&2
    return 1
  fi
  printf '%s\n' "$version"
}

release_tag() {
  local tag="${1:-${DOCPRIMS_RELEASE_TAG:-}}"
  if [[ -z "$tag" ]]; then
    echo "error: DOCPRIMS_RELEASE_TAG is required" >&2
    return 1
  fi
  if [[ ! "$tag" =~ ^v(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$ ]]; then
    echo "error: release tag must be canonical vX.Y.Z" >&2
    return 1
  fi
  if [[ "$tag" != "v$(release_version)" ]]; then
    echo "error: release tag does not match VERSION" >&2
    return 1
  fi
  printf '%s\n' "$tag"
}

require_release_guard() {
  local strict="${1:-0}"
  local root
  root="$(release_repo_root)"
  if [[ "$strict" == "1" ]]; then
    DOCPRIMS_REQUIRE_TAG=1 "$root/scripts/release-guard-tag-version.sh"
  else
    "$root/scripts/release-guard-tag-version.sh"
  fi
}

# One line per CLI archive: platform, archive format, binary name.
release_cli_platforms() {
  cat "$(release_repo_root)/config/release/cli-platforms.txt"
}

release_base_assets() {
  local version
  version="$(release_version)"
  printf '%s\n' \
    "LICENSE-APACHE" \
    "LICENSE-MIT" \
    "docprims.h" \
    "sbom-${version}.cdx.json"
  local platform format binary
  while read -r platform format binary; do
    printf 'docprims-%s-%s.%s\n' "$version" "$platform" "$format"
  done < <(release_cli_platforms)
}

release_signable_assets() {
  local tag
  tag="$(release_tag)"
  release_base_assets
  printf 'release-notes-%s.md\n' "$tag"
  printf '%s\n' expected-fingerprints.txt expected-fingerprints.ndjson
}

release_checksummed_assets() {
  release_signable_assets
  printf '%s\n' SHA256SUMS SHA512SUMS
}

release_signed_without_keys_assets() {
  release_checksummed_assets
  printf '%s\n' SHA256SUMS.minisig SHA512SUMS.minisig
  if [[ -n "${DOCPRIMS_PGP_KEY_ID:-}" || -n "${DOCPRIMS_GPG_HOMEDIR:-}" ]]; then
    require_complete_pgp_config
    printf '%s\n' SHA256SUMS.asc SHA512SUMS.asc
  fi
}

release_signed_assets() {
  release_signed_without_keys_assets
  printf '%s\n' docprims-minisign.pub
  if [[ -n "${DOCPRIMS_PGP_KEY_ID:-}" || -n "${DOCPRIMS_GPG_HOMEDIR:-}" ]]; then
    require_complete_pgp_config
    printf '%s\n' docprims-release-signing-key.asc
  fi
}

require_complete_pgp_config() {
  if [[ -z "${DOCPRIMS_PGP_KEY_ID:-}" || -z "${DOCPRIMS_GPG_HOMEDIR:-}" ]]; then
    echo "error: optional PGP signing requires both PGP variables" >&2
    return 1
  fi
}

assert_exact_directory_inventory() (
  local directory="$1"
  local producer="$2"
  if [[ ! -d "$directory" || -L "$directory" ]]; then
    echo "error: release directory is absent or unsafe" >&2
    return 1
  fi

  local expected_file actual_file
  expected_file="$(mktemp "${TMPDIR:-/tmp}/docprims-expected.XXXXXX")"
  actual_file="$(mktemp "${TMPDIR:-/tmp}/docprims-actual.XXXXXX")"
  trap 'rm -f "$expected_file" "$actual_file"' EXIT

  "$producer" | LC_ALL=C sort >"$expected_file"
  find "$directory" -mindepth 1 -maxdepth 1 -print |
    while IFS= read -r entry; do basename "$entry"; done |
    LC_ALL=C sort >"$actual_file"

  if ! cmp -s "$expected_file" "$actual_file"; then
    echo "error: release directory inventory mismatch" >&2
    diff -u "$expected_file" "$actual_file" >&2 || true
    return 1
  fi
  while IFS= read -r name; do
    if [[ ! -f "$directory/$name" || -L "$directory/$name" ]]; then
      echo "error: release asset is not a regular file: $name" >&2
      return 1
    fi
  done <"$expected_file"
)

# A CLI archive holds exactly the binary and both licenses as regular,
# top-level files.
validate_cli_archive() {
  local archive="$1"
  local filename version platform format binary expected_binary=""
  filename="$(basename "$archive")"
  version="$(release_version)"
  while read -r platform format binary; do
    if [[ "$filename" == "docprims-${version}-${platform}.${format}" ]]; then
      expected_binary="$binary"
      break
    fi
  done < <(release_cli_platforms)
  if [[ -z "$expected_binary" ]]; then
    echo "error: unexpected CLI archive name: $filename" >&2
    return 1
  fi
  python3 - "$archive" "$format" "$expected_binary" <<'PY'
import stat
import sys
import tarfile
import zipfile

archive, archive_format, binary = sys.argv[1:]
expected = sorted([binary, "LICENSE-APACHE", "LICENSE-MIT"])
if archive_format == "tar.gz":
    with tarfile.open(archive, "r:gz") as packed:
        members = packed.getmembers()
        names = [m.name for m in members]
        irregular = [m.name for m in members if not m.isreg()]
elif archive_format == "zip":
    with zipfile.ZipFile(archive) as packed:
        members = packed.infolist()
        names = [m.filename for m in members]
        irregular = [
            m.filename
            for m in members
            if m.is_dir() or (m.external_attr >> 16 and not stat.S_ISREG(m.external_attr >> 16))
        ]
else:
    sys.exit(f"error: unknown archive format {archive_format}")
if any("/" in n or "\\" in n or n in ("", ".", "..") for n in names):
    sys.exit(f"error: unsafe or nested member in {archive}")
if irregular:
    sys.exit(f"error: non-regular member in {archive}: {irregular}")
if sorted(names) != expected:
    sys.exit(f"error: {archive} members {sorted(names)} != {expected}")
PY
}

validate_all_cli_archives() {
  local directory="$1"
  local version platform format binary
  version="$(release_version)"
  while read -r platform format binary; do
    validate_cli_archive "$directory/docprims-${version}-${platform}.${format}"
  done < <(release_cli_platforms)
}

assert_github_release_state() (
  local expected_assets_producer="$1"
  local tag root tag_commit
  tag="$(release_tag)"
  root="$(release_repo_root)"
  tag_commit="$(git -C "$root" rev-parse "refs/tags/${tag}^{}")"

  if [[ -n "${GH_REPO+x}" ]]; then
    echo "error: GH_REPO must be unset; repository authority is fixed" >&2
    return 1
  fi
  for command_name in gh jq; do
    command -v "$command_name" >/dev/null 2>&1 || {
      echo "error: required release command is unavailable" >&2
      return 1
    }
  done
  if [[ "$(gh repo view "$DOCPRIMS_REPOSITORY" \
    --json nameWithOwner --jq .nameWithOwner)" != "$DOCPRIMS_REPOSITORY" ]]; then
    echo "error: GitHub repository identity mismatch" >&2
    return 1
  fi

  local state expected actual
  state="$(mktemp "${TMPDIR:-/tmp}/docprims-release-state.XXXXXX")"
  expected="$(mktemp "${TMPDIR:-/tmp}/docprims-remote-expected.XXXXXX")"
  actual="$(mktemp "${TMPDIR:-/tmp}/docprims-remote-actual.XXXXXX")"
  trap 'rm -f "$state" "$expected" "$actual"' EXIT
  gh release view "$tag" --repo "$DOCPRIMS_REPOSITORY" \
    --json tagName,targetCommitish,isDraft,assets >"$state"

  if [[ "$(jq -r .tagName "$state")" != "$tag" ||
  "$(jq -r .targetCommitish "$state")" != "$tag_commit" ||
  "$(jq -r .isDraft "$state")" != "true" ]]; then
    echo "error: GitHub release tag, target, or draft state mismatch" >&2
    return 1
  fi
  "$expected_assets_producer" | LC_ALL=C sort >"$expected"
  jq -r '.assets[].name' "$state" | LC_ALL=C sort >"$actual"
  if ! cmp -s "$expected" "$actual"; then
    echo "error: remote draft asset inventory mismatch" >&2
    diff -u "$expected" "$actual" >&2 || true
    return 1
  fi
)
