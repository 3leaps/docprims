#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
"$SCRIPT_DIR/release-crates.py" check
# Every unpublished workspace crate must be refused by cargo itself.
for crate in docprims-ffi docprims-ts-napi; do
  cargo metadata --no-deps --format-version 1 --locked |
    jq -e --arg name "$crate" 'any(.packages[]; .name == $name)' >/dev/null || {
    echo "error: unpublished crate $crate is not a workspace member" >&2
    exit 1
  }
  result="$(cargo publish --dry-run --locked --allow-dirty -p "$crate" 2>&1)" && {
    echo "error: unpublished crate $crate passed publish dry run" >&2
    exit 1
  }
  grep -q 'cannot be published' <<<"$result" || {
    echo "error: $crate failed for a reason other than publish = false" >&2
    exit 1
  }
done
python3 -B - "$SCRIPT_DIR" <<'PY'
import importlib.util
import pathlib
import sys

path = pathlib.Path(sys.argv[1]) / "release-crates.py"
spec = importlib.util.spec_from_file_location("release_crates", path)
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
names = ["docprims-core", "docprims-text", "docprims-ooxml", "docprims"]
core = {"name": "docprims-core"}
packages = [
    {"name": "docprims-core", "publish": None, "dependencies": []},
    {"name": "docprims-text", "publish": None, "dependencies": [core]},
    {"name": "docprims-ooxml", "publish": None, "dependencies": [core]},
    {
        "name": "docprims",
        "publish": None,
        "dependencies": [core, {"name": "docprims-text"}, {"name": "docprims-ooxml", "kind": "dev"}],
    },
    {"name": "docprims-ffi", "publish": [], "dependencies": [{"name": "docprims"}]},
    {"name": "docprims-ts-napi", "publish": [], "dependencies": [{"name": "docprims"}]},
]
manifests = {name: {"package": {"publish": True}} for name in names}
module.validate(packages, names, manifests)


def publishable(name):
    return [{**p, "publish": None} if p["name"] == name else p for p in packages]


for bad_names, bad_packages, bad_manifests in (
    (names[::-1], packages, manifests),
    (["docprims-core", "docprims-text", "docprims", "docprims-ooxml"], packages, manifests),
    (names[:-1], packages, manifests),
    (names + names[:1], packages, manifests),
    (names + ["docprims-ffi"], publishable("docprims-ffi"), {**manifests, "docprims-ffi": {"package": {"publish": True}}}),
    (names, publishable("docprims-ffi"), manifests),
    (names, publishable("docprims-ts-napi"), manifests),
    (names, packages, {**manifests, "docprims-core": {"package": {"publish": False}}}),
    (names, packages, {**manifests, "docprims": {"package": {}}}),
    (["docprimsx"] + names[1:], packages, manifests),
    ([], packages, manifests),
):
    try:
        module.validate(bad_packages, bad_names, bad_manifests)
    except ValueError:
        continue
    raise AssertionError(f"negative control unexpectedly passed: {bad_names}")
print("[ok] publishable crate list and negative controls")
PY
