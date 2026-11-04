#!/usr/bin/env python3
"""Runs the tests of the workspace's packages as they will be published.

`cargo package --workspace` writes each member as a `.crate` archive under
`<target>/package`. A package carries less than the repository: the official
schemas and examples, the tools and the other members stay out. A test that
reads what its package does not carry fails only here, not in the repository.

For every member the script unpacks its archive into a directory of its own,
outside the repository, and runs `cargo test` there; the members it depends on
are taken from their unpacked archives too (`patch.crates-io`), so that a
version not yet on crates.io is tested against what will be published with it.
It fails if any package's tests fail.

Only the standard library is used. Usage, from the repository root, after
`cargo package --workspace`:

    python3 tools/package_tests.py
"""

import json
import os
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path


def members():
    """Each member's name, version and the members it depends on, in the
    order the workspace lists them; and the target directory."""
    metadata = json.loads(
        subprocess.run(
            ["cargo", "metadata", "--no-deps", "--format-version", "1"],
            check=True,
            capture_output=True,
            text=True,
        ).stdout
    )
    names = {package["name"] for package in metadata["packages"]}
    listed = [
        (
            package["name"],
            package["version"],
            sorted(
                {
                    dependency["name"]
                    for dependency in package["dependencies"]
                    if dependency["name"] in names
                }
            ),
        )
        for package in metadata["packages"]
    ]
    return listed, Path(metadata["target_directory"])


def unpack(archive, into):
    """Unpacks a `.crate` archive into `into`; the directory it holds."""
    with tarfile.open(archive, "r:gz") as tar:
        # Pythons since 3.12 (and some patch releases before) check what an
        # archive writes; the older ones have no such check to ask for.
        if hasattr(tarfile, "data_filter"):
            tar.extraction_filter = tarfile.data_filter
        tar.extractall(into)
    unpacked = into / archive.name.removesuffix(".crate")
    # Cargo takes an unpacked package for unchanged by its files' times,
    # which the archive keeps, and a test binary built in an earlier run
    # would read its fixtures from that run's directory, gone since: the
    # files dated now are built anew, what they depend on is not.
    for file in unpacked.rglob("*"):
        os.utime(file)
    return unpacked


def main():
    listed, target = members()
    packages = target / "package"
    failed = []
    with tempfile.TemporaryDirectory(prefix="package-tests-") as scratch:
        unpacked = {}
        for name, version, _ in listed:
            archive = packages / f"{name}-{version}.crate"
            if not archive.is_file():
                sys.exit(f"{archive} is missing: run `cargo package --workspace` first")
            unpacked[name] = unpack(archive, Path(scratch))
        # One target directory for every package: they share their
        # dependencies, and the unpacked members are built once each.
        environment = dict(os.environ, CARGO_TARGET_DIR=str(target / "package-tests"))
        for name, version, dependencies in listed:
            patches = [
                argument
                for dependency in dependencies
                for argument in (
                    "--config",
                    f'patch.crates-io.{dependency}.path="{unpacked[dependency]}"',
                )
            ]
            print(f"== {name} {version}", flush=True)
            result = subprocess.run(
                ["cargo", "test", "--no-fail-fast", *patches],
                cwd=unpacked[name],
                env=environment,
            )
            if result.returncode != 0:
                failed.append(f"{name} {version}")
    if failed:
        sys.exit("the tests of the published package fail: " + ", ".join(failed))
    print("the tests of every published package pass")


if __name__ == "__main__":
    main()
