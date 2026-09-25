#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/docprims-github-state.XXXXXX")"
fake_bin="$fixture/bin"
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/scripts" "$fake_bin"
mkdir -p "$fixture/config/release"
cp "$SCRIPT_DIR/release-common.sh" "$fixture/scripts/"
cp "$SCRIPT_DIR/../config/release/cli-platforms.txt" "$fixture/config/release/"
printf '0.1.0\n' >"$fixture/VERSION"
git init -q -b main "$fixture"
git -C "$fixture" config user.name "release state test"
git -C "$fixture" config user.email "noreply@example.invalid"
git -C "$fixture" add VERSION scripts/release-common.sh
git -C "$fixture" commit -qm "fixture"
git -C "$fixture" tag -a v0.1.0 -m "fixture"
git -C "$fixture" checkout -q --detach v0.1.0
fixture_commit="$(git -C "$fixture" rev-parse HEAD)"
original_path="$PATH"

cat >"$fake_bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "${FAKE_RELEASE_STATE:?}" in
  wrong-repo)
    if [[ "$1 $2" == "repo view" ]]; then
      printf 'other/docprims\n'
      exit 0
    fi
    ;;
esac
if [[ "$1 $2" == "repo view" ]]; then
	printf '3leaps/docprims\n'
	exit 0
fi
if [[ "$1 $2" == "release list" ]]; then
  printf '[{"tagName":"v0.1.0"}]\n'
  exit 0
fi
if [[ "$1 $2" != "release view" ]]; then
  exit 1
fi
draft=true
target="$FAKE_COMMIT"
extra=""
case "$FAKE_RELEASE_STATE" in
  valid) ;;
  published) draft=false ;;
  wrong-target) target=0000000000000000000000000000000000000000 ;;
  extra-asset) extra=',{"name":"foreign.txt"}' ;;
  missing-asset) ;;
  *) exit 1 ;;
esac
printf '{"tagName":"v0.1.0","targetCommitish":"%s","isDraft":%s,"assets":[' \
  "$target" "$draft"
printf '%s' \
  '{"name":"LICENSE-APACHE"},' \
  '{"name":"LICENSE-MIT"},' \
  '{"name":"docprims.h"},' \
  '{"name":"sbom-0.1.0.cdx.json"}'
while read -r platform format binary; do
  [[ "$FAKE_RELEASE_STATE" == missing-asset && "$platform" == windows-amd64 ]] && continue
  printf ',{"name":"docprims-0.1.0-%s.%s"}' "$platform" "$format"
done <"$FAKE_PLATFORM_FILE"
printf '%s]}\n' "$extra"
EOF
chmod +x "$fake_bin/gh"

expect_fail() {
  if "$@" >/dev/null 2>&1; then
    echo "expected failure: $*" >&2
    exit 1
  fi
}

run_check() {
  local state="$1"
  (
    cd "$fixture"
    export PATH="$fake_bin:$original_path"
    export DOCPRIMS_RELEASE_TAG=v0.1.0
    export FAKE_COMMIT="$fixture_commit"
    export FAKE_RELEASE_STATE="$state"
    export FAKE_PLATFORM_FILE="$fixture/config/release/cli-platforms.txt"
    unset GH_REPO
    # shellcheck source=/dev/null
    source scripts/release-common.sh
    assert_github_release_state release_base_assets
  )
}

run_check valid
expect_fail run_check published
expect_fail run_check wrong-target
expect_fail run_check wrong-repo
expect_fail run_check extra-asset
expect_fail run_check missing-asset
expect_fail env GH_REPO=other/repository \
  PATH="$fake_bin:$original_path" \
  DOCPRIMS_RELEASE_TAG=v0.1.0 \
  FAKE_COMMIT="$fixture_commit" \
  FAKE_RELEASE_STATE=valid \
  FAKE_PLATFORM_FILE="$fixture/config/release/cli-platforms.txt" \
  bash -c "cd '$fixture'; source scripts/release-common.sh; assert_github_release_state release_base_assets"

echo "[ok] GitHub repository, target, draft, and inventory controls passed"
