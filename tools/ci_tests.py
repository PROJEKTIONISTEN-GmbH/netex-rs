#!/usr/bin/env python3
"""Runs the CI's job "build and test" as a runner runs it.

A runner checks the repository out as `actions/checkout` does — a shallow
clone of the one commit, without tags, without anything beside the
repository — and runs the job's commands in an environment of its own. A test
that passes only on the machine it was written on, because it reads the
history, a file outside the repository or a variable set there, fails here
before it fails on the runner.

The commands are those of the job `test` in `.github/workflows/ci.yml`, every
`- run: cargo …` line in its order, so that the two cannot drift apart. The
environment keeps what finds the toolchain (`PATH`, `HOME`, `CARGO_HOME`,
`RUSTUP_HOME`) and nothing else.

What the runner image brings is not the repository's to clone: the schema
tests read what its `xmllint` writes, and libxml2 words its complaints
otherwise from one release to the next. The script prints the version it
finds and the image the job names; `--xmllint` puts another validator first
on `PATH` (with its libraries beside it, `../lib`, where it has them).

Only the standard library is used. Usage, from the repository root:

    python3 tools/ci_tests.py [--xmllint PATH] [--target-dir DIR] [--list]

`--target-dir` keeps the build between runs; by default it is built afresh
inside the clone, which is removed afterwards. Uncommitted changes are not
part of the clone and are named, not tested.
"""

import argparse
import os
import re
import shlex
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
WORKFLOW = ROOT / ".github" / "workflows" / "ci.yml"
JOB = "test"
KEPT_VARIABLES = ("PATH", "HOME", "CARGO_HOME", "RUSTUP_HOME")


def job_lines(workflow, job):
    """The lines of `job` under `jobs:` in the workflow's text."""
    lines = workflow.splitlines()
    try:
        start = lines.index(f"  {job}:") + 1
    except ValueError:
        sys.exit(f"{WORKFLOW} has no job `{job}`")
    end = next(
        (
            index
            for index in range(start, len(lines))
            if re.match(r"  \S", lines[index])
        ),
        len(lines),
    )
    return lines[start:end]


def cargo_commands(lines):
    """Every `- run: cargo …` command of a job's lines, in order."""
    return [
        match.group(1)
        for line in lines
        if (match := re.fullmatch(r"\s+- run: (cargo .+)", line))
    ]


def runner_image(lines):
    """The image the job's `runs-on` names."""
    for line in lines:
        if match := re.fullmatch(r"\s+runs-on: (\S+)", line):
            return match.group(1)
    return "not named"


def environment(xmllint, target_dir):
    """The caller's environment reduced to what finds the toolchain."""
    kept = {name: os.environ[name] for name in KEPT_VARIABLES if name in os.environ}
    if xmllint:
        binary = Path(xmllint).resolve()
        directory = binary if binary.is_dir() else binary.parent
        kept["PATH"] = os.pathsep.join([str(directory), kept.get("PATH", "")])
        libraries = directory.parent / "lib"
        if libraries.is_dir():
            kept["LD_LIBRARY_PATH"] = str(libraries)
    if target_dir:
        kept["CARGO_TARGET_DIR"] = str(Path(target_dir).resolve())
    return kept


def first_line(command, env, cwd):
    """The first line `command` prints, on either stream; or why there is none."""
    try:
        result = subprocess.run(command, env=env, cwd=cwd, capture_output=True, text=True)
    except FileNotFoundError:
        return f"{command[0]} is not on PATH"
    lines = (result.stdout + result.stderr).splitlines()
    return lines[0] if lines else f"{command[0]} printed nothing"


def git(*arguments):
    return subprocess.run(
        ["git", *arguments], cwd=ROOT, check=True, capture_output=True, text=True
    ).stdout.strip()


def main():
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--xmllint", help="an xmllint binary, or its directory, to put first on PATH")
    parser.add_argument("--target-dir", help="the cargo target directory, kept between runs")
    parser.add_argument("--list", action="store_true", help="print the commands and stop")
    arguments = parser.parse_args()

    lines = job_lines(WORKFLOW.read_text(encoding="utf-8"), JOB)
    commands = cargo_commands(lines)
    if not commands:
        sys.exit(f"the job `{JOB}` of {WORKFLOW} runs no cargo command")
    if arguments.list:
        print("\n".join(commands))
        return

    head = git("rev-parse", "HEAD")
    if git("status", "--porcelain"):
        print("uncommitted changes are not part of the clone and are not tested", flush=True)
    env = environment(arguments.xmllint, arguments.target_dir)
    with tempfile.TemporaryDirectory(prefix="ci-tests-") as scratch:
        clone = Path(scratch) / "repository"
        subprocess.run(
            ["git", "clone", "--quiet", "--depth", "1", "--no-tags", ROOT.as_uri(), str(clone)],
            check=True,
        )
        shallow = subprocess.run(
            ["git", "rev-parse", "--is-shallow-repository"],
            cwd=clone,
            check=True,
            capture_output=True,
            text=True,
        ).stdout.strip()
        print(f"commit {head[:12]}, shallow clone: {shallow}", flush=True)
        print(f"rustc: {first_line(['rustc', '--version'], env, clone)}", flush=True)
        print(f"xmllint: {first_line(['xmllint', '--version'], env, clone)}", flush=True)
        print(f"the runner's xmllint is that of the image {runner_image(lines)}", flush=True)
        for command in commands:
            print(f"== {command}", flush=True)
            result = subprocess.run(shlex.split(command), cwd=clone, env=env)
            if result.returncode != 0:
                sys.exit(f"`{command}` failed with exit status {result.returncode}")
    print(f"the job `{JOB}` passes on {head[:12]} as a runner runs it")


if __name__ == "__main__":
    main()
