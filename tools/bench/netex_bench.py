#!/usr/bin/env python3
"""Measures what reading whole NeTEx deliveries and converting them to GTFS
costs, with the `netex` command (netex-cli).

A list of deliveries (`deliveries.json` beside this script) names each
delivery's files, the stop register read along with it, its publisher's own
GTFS feed where there is one, its licence and its stand. For each delivery the
script measures, on each number of threads the list gives:

* `netex stats` — reading the delivery and resolving it, the figures it prints;
* `netex convert --to gtfs` — reading it and writing the whole feed, and with
  `--shapes` as well where the list says so (on the most threads only);

each as often as `--runs` says. Of every run it keeps the wall clock, the CPU
time (user and system), the peak resident memory of the process, its resident
memory over time, and the load of the machine before and after; of the
delivery the size of its archives and of the XML they inflate to, of the feed
its size and the rows written to each file. With `--lock FILE` the runs take
an exclusive lock on the file first (`flock`), so that measurements on a
shared machine do not overlap: each run on its own, or with `--lock-scope` the
runs of a delivery on one number of threads, or all of a delivery's runs,
together — a delivery of seconds need not queue for the lock a hundred times.

With `--window FIRST LAST` each feed written is compared with the publisher's
over that window (`gtfs_days.py`); with `--validator JAR` both are checked by
the GTFS validator of MobilityData, where the list asks for it, on as many
threads as the delivery is measured on at most, its rules about time taken on
the first day of the window.

What is measured goes to `OUT/deliveries/<id>.json`, one file per delivery, so
that deliveries may be measured one after the other in separate calls; each
call then writes `OUT/results.json` and `OUT/results.md` from every delivery
measured so far. The feeds written stay under `OUT/feeds/`.

Only the standard library is used; Linux only (`/proc`, `wait4`). Usage:

    python3 tools/bench/netex_bench.py run tools/bench/deliveries.json \\
        --netex target/release/netex --data DIR --out DIR \\
        [--runs 2] [--only ID]... [--lock FILE [--lock-scope run|threads|delivery]]
        [--window FIRST LAST] [--validator JAR]
    python3 tools/bench/netex_bench.py table OUT
"""

import argparse
import contextlib
import datetime
import fcntl
import json
import os
import platform
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
import zipfile
from pathlib import Path

import gtfs_days

GIB = 1024**3
MIB = 1024**2
GB = 1000**3
MB = 1000**2
# How often the resident memory of a running command is read, in seconds,
# and how many of those readings a run keeps.
SAMPLE_EVERY = 0.5
SAMPLES_KEPT = 240
# What the lock is held over: each run, the runs of a delivery on one number of
# threads, or all runs of a delivery.
LOCK_SCOPES = ("run", "threads", "delivery")
# The severities the GTFS validator reports notices with.
SEVERITIES = ("ERROR", "WARNING", "INFO")


class RunFailed(Exception):
    """A measured command that exited with a status other than 0."""


class LockTimeout(Exception):
    """A lock not taken in the time allowed."""


@contextlib.contextmanager
def lock_held(path, wait):
    """Holds an exclusive `flock` on `path` while the block runs, waiting up to
    `wait` seconds for it; yields the seconds waited. Without a path, holds
    nothing.

    The wait is the kernel's, as `flock -w` waits: a process polling for the
    lock loses it to every process blocked on it, whom the kernel wakes the
    moment it is released. An alarm ends the wait (the main thread only)."""
    if path is None:
        yield 0.0
        return

    def give_up(_signal, _frame):
        raise LockTimeout(f"{path}: not taken in {wait} s")

    with open(path, "a", encoding="utf-8") as file:
        started = time.monotonic()
        if wait > 0:
            previous = signal.signal(signal.SIGALRM, give_up)
            signal.setitimer(signal.ITIMER_REAL, wait)
            try:
                fcntl.flock(file, fcntl.LOCK_EX)
            finally:
                signal.setitimer(signal.ITIMER_REAL, 0)
                signal.signal(signal.SIGALRM, previous)
        else:
            try:
                fcntl.flock(file, fcntl.LOCK_EX | fcntl.LOCK_NB)
            except BlockingIOError:
                raise LockTimeout(f"{path}: held by another") from None
        try:
            yield time.monotonic() - started
        finally:
            fcntl.flock(file, fcntl.LOCK_UN)


def load():
    """The load average of the machine over 1, 5 and 15 minutes."""
    return [float(field) for field in Path("/proc/loadavg").read_text().split()[:3]]


def resident(pid):
    """The resident memory of a running process in bytes; None once it has
    exited."""
    try:
        for line in Path(f"/proc/{pid}/status").read_text().splitlines():
            if line.startswith("VmRSS:"):
                return int(line.split()[1]) * 1024
    except (FileNotFoundError, ProcessLookupError):
        pass
    return None


def thinned(samples, kept):
    """`kept` of `samples`, evenly spread, and the last and the highest
    besides."""
    if len(samples) <= kept:
        return samples
    step = len(samples) / kept
    chosen = {int(n * step) for n in range(kept)} | {len(samples) - 1}
    chosen.add(max(range(len(samples)), key=lambda n: samples[n][1]))
    return [samples[n] for n in sorted(chosen)]


def measure(command, stdout, stderr, lock=None, lock_wait=0.0):
    """Runs `command` with its output written to the files `stdout` and
    `stderr`, and returns what it cost. Raises `RunFailed` with the end of
    its error output if it exits with a status other than 0."""
    with lock_held(lock, lock_wait) as waited:
        load_before = load()
        with open(stdout, "wb") as out, open(stderr, "wb") as err:
            started = time.monotonic()
            pid = os.posix_spawnp(
                command[0],
                [str(part) for part in command],
                os.environ,
                file_actions=[
                    (os.POSIX_SPAWN_DUP2, out.fileno(), 1),
                    (os.POSIX_SPAWN_DUP2, err.fileno(), 2),
                ],
            )
            samples = []
            done = threading.Event()

            def sample():
                while not done.wait(SAMPLE_EVERY):
                    rss = resident(pid)
                    if rss is not None:
                        samples.append((round(time.monotonic() - started, 1), rss))

            sampler = threading.Thread(target=sample, daemon=True)
            sampler.start()
            _, status, usage = os.wait4(pid, 0)
            wall = time.monotonic() - started
            done.set()
            sampler.join()
        load_after = load()
    exit_code = os.waitstatus_to_exitcode(status)
    if exit_code != 0:
        tail = Path(stderr).read_text(errors="replace")[-2000:]
        raise RunFailed(f"{' '.join(map(str, command))} exited with {exit_code}:\n{tail}")
    return {
        "wall_s": round(wall, 2),
        "cpu_s": round(usage.ru_utime + usage.ru_stime, 2),
        "user_s": round(usage.ru_utime, 2),
        "system_s": round(usage.ru_stime, 2),
        # The kernel's high-water mark (`ru_maxrss`, KiB on Linux) is kept from
        # per-thread counters folded in late, so a reading of VmRSS taken while
        # the command ran may lie a little above it; the larger counts.
        "peak_rss_bytes": max([usage.ru_maxrss * 1024, *(rss for _, rss in samples)]),
        "rss_samples": thinned(samples, SAMPLES_KEPT),
        "load_before": load_before,
        "load_after": load_after,
        "lock_wait_s": round(waited, 1),
    }


def mean(runs, key):
    """The mean of one figure over the runs."""
    return sum(run[key] for run in runs) / len(runs)


def summary(runs):
    """The mean of the runs, and their spread, of what the tables show."""
    return {
        key: {
            "mean": round(mean(runs, key), 2),
            "min": min(run[key] for run in runs),
            "max": max(run[key] for run in runs),
        }
        for key in ("wall_s", "cpu_s", "peak_rss_bytes")
    }


def input_size(paths):
    """The bytes on disk of the files of a delivery, the bytes of XML they
    hold — inflated, where they are zip archives — and how many documents."""
    on_disk = xml = documents = 0
    for path in paths:
        if path.is_dir():
            files = [file for file in sorted(path.rglob("*")) if file.is_file()]
            on_disk += sum(file.stat().st_size for file in files)
            for file in files:
                if file.suffix.lower() == ".xml":
                    xml += file.stat().st_size
                    documents += 1
        else:
            on_disk += path.stat().st_size
            if path.suffix.lower() == ".xml":
                xml += path.stat().st_size
                documents += 1
            else:
                with zipfile.ZipFile(path) as archive:
                    for entry in archive.infolist():
                        if entry.filename.lower().endswith(".xml"):
                            xml += entry.file_size
                            documents += 1
    return {"bytes_on_disk": on_disk, "xml_bytes": xml, "documents": documents}


def warm(paths):
    """Reads the files of a delivery once, so that every run finds them in the
    page cache alike."""
    for path in paths:
        files = sorted(path.rglob("*")) if path.is_dir() else [path]
        for file in files:
            if file.is_file():
                with open(file, "rb") as read:
                    while read.read(1 << 24):
                        pass


def machine(paths):
    """The machine the measurements run on: processor, the processors and
    memory this process may use, the operating system, and the file systems
    the given paths are on."""
    cpuinfo = Path("/proc/cpuinfo").read_text()
    model = next(
        (line.split(":", 1)[1].strip() for line in cpuinfo.splitlines() if line.startswith("model name")),
        platform.processor(),
    )
    meminfo = {
        line.split(":")[0]: int(line.split()[1]) * 1024
        for line in Path("/proc/meminfo").read_text().splitlines()
        if line.split(":")[0] in ("MemTotal", "SwapTotal")
    }
    return {
        "cpu": model,
        "logical_cpus": os.cpu_count(),
        "cpus_usable": cpu_quota() or len(os.sched_getaffinity(0)),
        "memory_bytes": meminfo["MemTotal"],
        "memory_limit_bytes": memory_limit(),
        "swap_bytes": meminfo.get("SwapTotal", 0),
        "kernel": platform.release(),
        "file_systems": {str(path): file_system(path) for path in paths},
    }


def cpu_quota():
    """The processors a cgroup (v2) quota grants, where one is set."""
    try:
        quota, period = Path("/sys/fs/cgroup/cpu.max").read_text().split()
    except (FileNotFoundError, ValueError):
        return None
    return None if quota == "max" else round(int(quota) / int(period), 1)


def memory_limit():
    """The memory a cgroup (v2) limit grants, where one is set."""
    try:
        limit = Path("/sys/fs/cgroup/memory.max").read_text().strip()
    except FileNotFoundError:
        return None
    return None if limit == "max" else int(limit)


def file_system(path):
    """The device and type of the file system `path` is on, the longest mount
    point holding it, and whether the device is a rotating disk."""
    resolved = str(Path(path).resolve())
    best = None
    for line in Path("/proc/mounts").read_text().splitlines():
        device, point, kind = line.split()[:3]
        inside = resolved == point or resolved.startswith(point.rstrip("/") + "/")
        if inside and (best is None or len(point) > len(best[1])):
            best = (device, point, kind)
    if best is None:
        return None
    device, point, kind = best
    # A partition's device is named after its disk (`nvme0n1p1`, `sda2`); the
    # longest disk name it starts with is its disk.
    name = Path(device).name
    disks = [block for block in Path("/sys/block").glob("*") if name.startswith(block.name)]
    rotational = None
    if disks:
        flag = max(disks, key=lambda block: len(block.name)) / "queue" / "rotational"
        if flag.exists():
            rotational = flag.read_text().strip() == "1"
    return {"device": device, "mount": point, "type": kind, "rotational": rotational}


def resolve(data, name):
    """A path of the list, relative to the data directory."""
    path = Path(name)
    return path if path.is_absolute() else data / path


def netex_command(netex, threads, step, source, along):
    """The `netex` command line of a step, with the documents read along."""
    command = [netex, "--threads", threads, step, source]
    for path in along:
        command += ["--with", path]
    return command


def validate(validator, feed, into, day, threads):
    """The GTFS validator of MobilityData on a feed, its rules about time taken
    on `day`: its notices counted by severity and, for errors and warnings, by
    code, and the errors of the validator itself (`system_errors.json`) — a
    validator that ran out of memory reports there, not in its notices."""
    if into.exists():
        shutil.rmtree(into)
    command = ["java", "-jar", validator, "--input", feed, "--output_base", into,
               "--threads", threads, "--skip_validator_update"]
    if day is not None:
        command += ["--date", day.isoformat()]
    with tempfile.TemporaryDirectory() as scratch:
        measured = measure(command, Path(scratch) / "stdout", Path(scratch) / "stderr")
    notices = json.loads((into / "report.json").read_text())["notices"]
    counted = {severity: 0 for severity in SEVERITIES}
    codes = {severity: {} for severity in SEVERITIES[:2]}
    for notice in notices:
        severity = notice["severity"]
        counted[severity] = counted.get(severity, 0) + notice["totalNotices"]
        if severity in codes:
            codes[severity][notice["code"]] = notice["totalNotices"]
    own_errors = into / "system_errors.json"
    failures = json.loads(own_errors.read_text())["notices"] if own_errors.exists() else []
    return {
        "notices": counted,
        "codes": {severity: dict(sorted(found.items(), key=lambda item: -item[1]))
                  for severity, found in codes.items()},
        "system_errors": {failure["code"]: failure["totalNotices"] for failure in failures},
        "wall_s": measured["wall_s"],
    }


def measure_delivery(delivery, options):
    """Every measurement of one delivery, as `OUT/deliveries/<id>.json` keeps it."""
    data, out = options.data, options.out
    source = resolve(data, delivery["netex"])
    along = [resolve(data, path) for path in delivery.get("with", [])]
    published = resolve(data, delivery["gtfs"]) if delivery.get("gtfs") else None
    threads = delivery["threads"]
    logs = out / "logs" / delivery["id"]
    feeds = out / "feeds"
    logs.mkdir(parents=True, exist_ok=True)
    feeds.mkdir(parents=True, exist_ok=True)
    extra = [str(argument) for argument in delivery.get("convert", [])]

    # The steps on each number of threads: reading, converting, and
    # converting with shapes on the most threads where the list asks for it.
    steps = {
        count: [("stats", False), ("convert", False)]
        + ([("convert", True)] if delivery.get("shapes") and count == max(threads) else [])
        for count in threads
    }
    runs = {(step, count, shapes): [] for count in threads for step, shapes in steps[count]}
    warm([source, *along])
    stats = None
    reports = {}
    waited = []

    def scoped(scope):
        """The lock, held over this scope where the options say so."""
        return lock_held(options.lock if options.lock_scope == scope else None, options.lock_wait)

    with scoped("delivery") as waited_for_delivery:
        waited.append(waited_for_delivery)
        for run in range(1, options.runs + 1):
            for count in threads:
                with scoped("threads") as waited_for_threads:
                    waited.append(waited_for_threads)
                    for step, shapes in steps[count]:
                        name = f"{step}{'-shapes' if shapes else ''}-t{count}-r{run}"
                        feed = feeds / f"{delivery['id']}{'-shapes' if shapes else ''}.zip"
                        report = logs / f"{name}.report.json"
                        command = netex_command(options.netex, count, step, source, along)
                        if step == "stats":
                            command += ["--format", "json"]
                        else:
                            feed.unlink(missing_ok=True)
                            command += ["--to", "gtfs", feed, "--report", report, *extra]
                            if shapes:
                                command.append("--shapes")
                        print(f"{delivery['id']}: {name}", file=sys.stderr, flush=True)
                        lock = options.lock if options.lock_scope == "run" else None
                        measured = measure(
                            command, logs / f"{name}.out", logs / f"{name}.err", lock, options.lock_wait
                        )
                        waited.append(measured["lock_wait_s"])
                        runs[(step, count, shapes)].append(measured)
                        if step == "stats":
                            stats = json.loads((logs / f"{name}.out").read_text())["total"]
                        else:
                            reports[shapes] = (feed, json.loads(report.read_text()))

    result = {
        "id": delivery["id"],
        "name": delivery["name"],
        "group": delivery.get("group", delivery["name"]),
        **{key: delivery.get(key) for key in ("publisher", "licence", "stand", "url")},
        "netex": netex_version(options.netex),
        "machine": machine([data, out]),
        "input": input_size([source, *along]),
        "lock": {"scope": options.lock_scope if options.lock else None,
                 "waited_s": round(sum(waited), 1)},
        "measurements": [
            {"step": step, "threads": count, "shapes": shapes,
             "summary": summary(measured), "runs": measured}
            for (step, count, shapes), measured in runs.items()
        ],
        "stats": stats,
        "feeds": {
            ("with_shapes" if shapes else "plain"): {
                "file": str(feed.relative_to(out)),
                "bytes": feed.stat().st_size,
                "rows": report["rows"],
                "period": report["period"],
                "decisions": {decision["code"]: decision["count"] for decision in report["decisions"]},
            }
            for shapes, (feed, report) in reports.items()
        },
    }
    plain = reports[False][0]
    if published is not None and options.window:
        first, last = options.window
        result["comparison"] = gtfs_days.compare(plain, published, first, last)
    if options.validator and delivery.get("validate"):
        checked = out / "validator" / delivery["id"]
        day = options.window[0] if options.window else None
        result["validation"] = {}
        for which, feed in (("converted", plain), ("published", published)):
            if feed is None:
                continue
            # A validator that fails is a finding about the validator and the
            # feed, not a reason to lose the measurements taken.
            try:
                checked_feed = validate(options.validator, feed, checked / which, day, max(threads))
            except RunFailed as failure:
                checked_feed = {"failed": str(failure)}
            result["validation"][which] = checked_feed
    return result


def netex_version(netex):
    """What `netex --version` says, the version measured."""
    return subprocess.run([netex, "--version"], check=True, capture_output=True, text=True).stdout.strip()


# The tables of `results.md`.


def size(count):
    """Bytes of data, in decimal units."""
    return f"{count / GB:.2f} GB" if count >= GB else f"{count / MB:.1f} MB"


def memory(count):
    """Bytes of memory, in binary units."""
    return f"{count / GIB:.2f} GiB" if count >= GIB else f"{count / MIB:.0f} MiB"


def seconds(count):
    return f"{count:.1f} s" if count < 600 else f"{count / 60:.1f} min"


def number(count):
    return f"{count:,}".replace(",", " ")


def signed(count):
    return f"+{number(count)}" if count > 0 else ("−" + number(-count) if count < 0 else "0")


def percent(share):
    return "—" if share is None else f"{share * 100:+.2f} %".replace("-", "−")


def found(delivery, step, threads, shapes=False):
    return next(
        (m["summary"] for m in delivery["measurements"]
         if m["step"] == step and m["threads"] == threads and m["shapes"] == shapes),
        None,
    )


def system_errors(checked):
    """The validator's own errors, where it reported any, as a note."""
    found = checked.get("system_errors") or {}
    if not found:
        return ""
    return "; the validator's own errors: " + ", ".join(
        f"`{code}` {number(total)}" for code, total in found.items()
    )


def markdown(deliveries):
    """The tables of `results.md`."""
    lines = []
    machines = []
    for delivery in deliveries:
        described = {key: value for key, value in delivery["machine"].items() if key != "file_systems"}
        if described not in machines:
            machines.append(described)
    lines += ["## Machine", ""]
    for described in machines:
        limit = described["memory_limit_bytes"]
        lines.append(
            f"- {described['cpu']}; {described['cpus_usable']} of {described['logical_cpus']} "
            f"logical CPUs usable; {memory(described['memory_bytes'])} memory"
            + (f", {memory(limit)} usable" if limit else "")
            + f"; Linux {described['kernel']}"
        )
    versions = sorted({delivery["netex"] for delivery in deliveries})
    lines += [f"- {', '.join(versions)}", ""]

    lines += [
        "## Each delivery on its most threads",
        "",
        "Wall clock and peak resident memory, the mean of the runs; CPU is user and system time.",
        "",
        "| Delivery | Stand | Documents | XML | Threads | Read: time | Read: peak | → GTFS: time | → GTFS: CPU | → GTFS: peak | Feed | Trips | Stop times |",
        "|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|",
    ]
    for delivery in deliveries:
        threads = max(m["threads"] for m in delivery["measurements"])
        read = found(delivery, "stats", threads)
        converted = found(delivery, "convert", threads)
        feed = delivery["feeds"]["plain"]
        lines.append(
            f"| {delivery['name']} | {delivery['stand']} | {number(delivery['input']['documents'])} "
            f"| {size(delivery['input']['xml_bytes'])} | {threads} "
            f"| {seconds(read['wall_s']['mean'])} | {memory(read['peak_rss_bytes']['mean'])} "
            f"| {seconds(converted['wall_s']['mean'])} | {seconds(converted['cpu_s']['mean'])} "
            f"| {memory(converted['peak_rss_bytes']['mean'])} | {size(feed['bytes'])} "
            f"| {number(feed['rows'].get('trips.txt', 0))} | {number(feed['rows'].get('stop_times.txt', 0))} |"
        )

    over_threads = [d for d in deliveries if len({m["threads"] for m in d["measurements"]}) > 1]
    if over_threads:
        lines += ["", "## Over the threads", ""]
        for delivery in over_threads:
            lines += [
                f"### {delivery['name']}",
                "",
                "| Threads | Read: time | Read: CPU | Read: peak | → GTFS: time | → GTFS: CPU | → GTFS: peak |",
                "|---:|---:|---:|---:|---:|---:|---:|",
            ]
            for threads in sorted({m["threads"] for m in delivery["measurements"]}):
                read = found(delivery, "stats", threads)
                converted = found(delivery, "convert", threads)
                lines.append(
                    f"| {threads} | {seconds(read['wall_s']['mean'])} | {seconds(read['cpu_s']['mean'])} "
                    f"| {memory(read['peak_rss_bytes']['mean'])} | {seconds(converted['wall_s']['mean'])} "
                    f"| {seconds(converted['cpu_s']['mean'])} | {memory(converted['peak_rss_bytes']['mean'])} |"
                )
            lines.append("")

    with_shapes = [d for d in deliveries if "with_shapes" in d["feeds"]]
    if with_shapes:
        lines += [
            "## With shapes",
            "",
            "| Delivery | Threads | → GTFS: time | → GTFS: peak | Feed | Shape points |",
            "|---|---:|---:|---:|---:|---:|",
        ]
        for delivery in with_shapes:
            threads = max(m["threads"] for m in delivery["measurements"])
            converted = found(delivery, "convert", threads, shapes=True)
            feed = delivery["feeds"]["with_shapes"]
            lines.append(
                f"| {delivery['name']} | {threads} | {seconds(converted['wall_s']['mean'])} "
                f"| {memory(converted['peak_rss_bytes']['mean'])} | {size(feed['bytes'])} "
                f"| {number(feed['rows'].get('shapes.txt', 0))} |"
            )
        lines.append("")

    files = sorted({file for d in deliveries for file in d["feeds"]["plain"]["rows"]})
    lines += [
        "## Rows written",
        "",
        "| Delivery | " + " | ".join(f"`{file}`" for file in files) + " |",
        "|---|" + "---:|" * len(files),
    ]
    for delivery in deliveries:
        rows = delivery["feeds"]["plain"]["rows"]
        lines.append(
            f"| {delivery['name']} | " + " | ".join(number(rows.get(file, 0)) for file in files) + " |"
        )

    compared = [d for d in deliveries if d.get("comparison")]
    if compared:
        first, last = compared[0]["comparison"]["window"]
        lines += [
            "",
            f"## Against the publisher's GTFS, {first} to {last}",
            "",
            "Trips running in the window and trip days (a trip once for each day it runs), "
            "in the feed written and in the publisher's.",
            "",
            "| Delivery | Trips: written | Trips: publisher | Trip days: written | Trip days: publisher | Difference |",
            "|---|---:|---:|---:|---:|---:|",
        ]
        for delivery in compared:
            comparison = delivery["comparison"]
            lines.append(
                f"| {delivery['name']} | {number(comparison['converted']['trips_in_window'])} "
                f"| {number(comparison['published']['trips_in_window'])} "
                f"| {number(comparison['converted']['trip_days'])} "
                f"| {number(comparison['published']['trip_days'])} "
                f"| {signed(comparison['trip_days']['difference'])} ({percent(comparison['trip_days']['relative'])}) |"
            )

    validated = [d for d in deliveries if d.get("validation")]
    if validated:
        lines += [
            "",
            "## GTFS validator",
            "",
            "| Delivery | Feed | Errors | Warnings | Infos | Error codes |",
            "|---|---|---:|---:|---:|---|",
        ]
        for delivery in validated:
            for which, checked in delivery["validation"].items():
                if "failed" in checked:
                    reason = checked["failed"].strip().splitlines()[-1]
                    lines.append(f"| {delivery['name']} | {which} | — | — | — | the validator failed: {reason} |")
                    continue
                codes = ", ".join(
                    f"`{code}` {number(total)}" for code, total in checked["codes"]["ERROR"].items()
                )
                lines.append(
                    f"| {delivery['name']} | {which} | {number(checked['notices']['ERROR'])} "
                    f"| {number(checked['notices']['WARNING'])} | {number(checked['notices']['INFO'])} "
                    f"| {codes or '—'}{system_errors(checked)} |"
                )

    lines += [
        "",
        "## Deliveries",
        "",
        "| Delivery | Publisher | Licence | Stand |",
        "|---|---|---|---|",
    ]
    for delivery in deliveries:
        lines.append(
            f"| {delivery['name']} | {delivery['publisher']} | {delivery['licence']} | {delivery['stand']} |"
        )
    return "\n".join(lines) + "\n"


def write_results(out, listed_order):
    """`results.json` and `results.md` from every delivery measured under
    `out`, in the order of the list where it names them."""
    measured = {
        path.stem: json.loads(path.read_text()) for path in sorted((out / "deliveries").glob("*.json"))
    }
    order = [identifier for identifier in listed_order if identifier in measured]
    order += sorted(set(measured) - set(order))
    deliveries = [measured[identifier] for identifier in order]
    (out / "results.json").write_text(json.dumps({"deliveries": deliveries}, indent=1, ensure_ascii=False))
    (out / "results.md").write_text(markdown(deliveries))
    return deliveries


def run(options):
    listed = json.loads(Path(options.list).read_text())["deliveries"]
    identifiers = [delivery["id"] for delivery in listed]
    unknown = set(options.only) - set(identifiers)
    if unknown:
        raise SystemExit(f"not in the list: {', '.join(sorted(unknown))}")
    (options.out / "deliveries").mkdir(parents=True, exist_ok=True)
    for delivery in listed:
        if options.only and delivery["id"] not in options.only:
            continue
        result = measure_delivery(delivery, options)
        (options.out / "deliveries" / f"{delivery['id']}.json").write_text(
            json.dumps(result, indent=1, ensure_ascii=False)
        )
    write_results(options.out, identifiers)


def main(arguments):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    commands = parser.add_subparsers(dest="command", required=True)
    measuring = commands.add_parser("run", help="measure the deliveries of a list")
    measuring.add_argument("list", help="the list of deliveries (JSON)")
    measuring.add_argument("--netex", required=True, help="the netex command to measure")
    measuring.add_argument("--data", required=True, type=Path, help="where the list's paths are")
    measuring.add_argument("--out", required=True, type=Path, help="where the results go")
    measuring.add_argument("--runs", type=int, default=2, help="runs of each measurement")
    measuring.add_argument("--only", action="append", default=[], metavar="ID",
                           help="measure only this delivery; may be given more than once")
    measuring.add_argument("--lock", type=Path, help="a file locked for each run")
    measuring.add_argument("--lock-wait", type=float, default=7200.0,
                           help="seconds to wait for the lock (7200)")
    measuring.add_argument("--lock-scope", choices=LOCK_SCOPES, default="run",
                           help="what the lock is held over: each run (the default), the runs "
                                "on one number of threads, or a delivery's every run")
    measuring.add_argument("--window", nargs=2, type=datetime.date.fromisoformat,
                           metavar=("FIRST", "LAST"), help="the days a feed is compared over")
    measuring.add_argument("--validator", type=Path, help="the GTFS validator of MobilityData (jar)")
    table = commands.add_parser("table", help="write the tables again from what was measured")
    table.add_argument("out", type=Path)
    table.add_argument("--list", help="the list whose order the tables follow")
    parsed = parser.parse_args(arguments)
    if parsed.command == "run":
        if parsed.runs < 1:
            parser.error("--runs must be at least 1")
        run(parsed)
    else:
        listed = json.loads(Path(parsed.list).read_text())["deliveries"] if parsed.list else []
        write_results(parsed.out, [delivery["id"] for delivery in listed])
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
