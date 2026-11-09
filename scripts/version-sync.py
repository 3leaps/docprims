#!/usr/bin/env python3
"""Write VERSION to every version site that scripts/check-version.sh checks."""

import json
import pathlib
import re
import subprocess
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
NPM = ROOT / "bindings/typescript/docprims"


def sync(version):
    cargo = ROOT / "Cargo.toml"
    text = cargo.read_text()
    text, count = re.subn(
        r'(?ms)^(\[workspace\.package\]\n(?:(?!^\[).)*?^version = )"[^"]*"',
        rf'\1"{version}"',
        text,
        count=1,
    )
    if count != 1:
        raise ValueError("Cargo.toml has no [workspace.package] version")
    text, count = re.subn(
        r'(?m)^(docprims(?:-core|-text|-ooxml)? = \{ version = )"[^"]*"',
        rf'\1"{version}"',
        text,
    )
    if count != 4:
        raise ValueError(f"expected 4 docprims workspace dependencies, found {count}")
    cargo.write_text(text)

    package = json.loads((NPM / "package.json").read_text())
    package["version"] = version
    for name in package.get("optionalDependencies", {}):
        package["optionalDependencies"][name] = version
    (NPM / "package.json").write_text(json.dumps(package, indent=2) + "\n")

    lock = json.loads((NPM / "package-lock.json").read_text())
    lock["version"] = version
    root = lock["packages"][""]
    root["version"] = version
    for name in root.get("optionalDependencies", {}):
        root["optionalDependencies"][name] = version
    (NPM / "package-lock.json").write_text(json.dumps(lock, indent=2) + "\n")

    subprocess.run(["cargo", "update", "--workspace", "--offline", "--quiet"], cwd=ROOT, check=True)


def main():
    version = (ROOT / "VERSION").read_text().strip()
    if not re.fullmatch(r"(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)", version):
        raise ValueError("VERSION must contain one stable semantic version")
    sync(version)
    print(f"[ok] Synced version sites to {version}")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, subprocess.CalledProcessError) as error:
        sys.exit(f"error: {error}")
