"""Tests of the benchmark tools: the trips and trip days of a feed written
here, two feeds compared, the measurement of a process and the lock it waits
for, the sizes of a delivery, the tables — and, with the `netex` command named
by `NETEX_BIN`, a whole run over two of the repository's own small deliveries.

Run from the repository root:

    NETEX_BIN=target/debug/netex python3 -m unittest discover -s tools/bench -v
"""

import contextlib
import datetime
import fcntl
import json
import os
import sys
import tempfile
import unittest
import zipfile
from pathlib import Path
from types import SimpleNamespace

import gtfs_days
import netex_bench

REPOSITORY = Path(__file__).resolve().parents[2]
MONDAY = datetime.date(2026, 11, 2)
SUNDAY = datetime.date(2026, 11, 8)


def write_feed(directory, files):
    """A feed of the given files, each a header and its rows."""
    directory.mkdir(parents=True, exist_ok=True)
    for name, (header, rows) in files.items():
        lines = [",".join(header)] + [",".join(row) for row in rows]
        (directory / name).write_text("\n".join(lines) + "\n", encoding="utf-8")
    return directory


def zipped(directory, archive):
    with zipfile.ZipFile(archive, "w") as written:
        for file in sorted(directory.iterdir()):
            written.write(file, file.name)
    return archive


CALENDAR_HEADER = ["service_id", *gtfs_days.WEEKDAYS, "start_date", "end_date"]


def week_feed(directory, saturday_trip=True):
    """A feed with four trips over the week of 2 November 2026: one on the
    working days, one on the working days but Wednesday, one added on the
    Saturday alone, and one whose calendar ended in October."""
    trips = [["r1", "wk", "t1"], ["r2", "wkx", "t2"], ["r1", "old", "t4"]]
    if saturday_trip:
        trips.append(["r1", "sat", "t3"])
    return write_feed(
        directory,
        {
            "routes.txt": (["route_id", "route_type"], [["r1", "3"], ["r2", "2"]]),
            "trips.txt": (["route_id", "service_id", "trip_id"], trips),
            "calendar.txt": (
                CALENDAR_HEADER,
                [
                    ["wk", "1", "1", "1", "1", "1", "0", "0", "20261001", "20261231"],
                    ["wkx", "1", "1", "1", "1", "1", "0", "0", "20261001", "20261231"],
                    ["old", "1", "1", "1", "1", "1", "1", "1", "20261001", "20261031"],
                ],
            ),
            "calendar_dates.txt": (
                ["service_id", "date", "exception_type"],
                [["wkx", "20261104", "2"], ["sat", "20261107", "1"], ["wk", "20261201", "2"]],
            ),
            "stops.txt": (
                ["stop_id", "location_type"],
                [["s1", ""], ["s2", "0"], ["p1", "1"]],
            ),
            "frequencies.txt": (
                ["trip_id", "start_time", "end_time", "headway_secs"],
                [["t1", "06:00:00", "09:00:00", "600"]],
            ),
        },
    )


class TripDays(unittest.TestCase):
    def test_a_week_counted_by_the_days_each_service_runs(self):
        with tempfile.TemporaryDirectory() as scratch:
            counted = gtfs_days.count(week_feed(Path(scratch) / "feed"), MONDAY, SUNDAY)
        self.assertEqual(counted["trips"], 4)
        self.assertEqual(counted["trips_in_window"], 3)
        # t1 five working days, t2 four (not Wednesday), t3 the Saturday.
        self.assertEqual(counted["trip_days"], 10)
        self.assertEqual(
            list(counted["trip_days_per_day"].values()), [2, 2, 1, 2, 2, 1, 0]
        )
        self.assertEqual(counted["trip_days_per_route_type"], {"2": 4, "3": 6})
        self.assertEqual(counted["stops_per_location_type"], {"0": 2, "1": 1})
        self.assertEqual(counted["frequencies"], 1)

    def test_a_zip_archive_counts_as_its_directory(self):
        with tempfile.TemporaryDirectory() as scratch:
            directory = week_feed(Path(scratch) / "feed")
            archive = zipped(directory, Path(scratch) / "feed.zip")
            self.assertEqual(
                gtfs_days.count(archive, MONDAY, SUNDAY),
                gtfs_days.count(directory, MONDAY, SUNDAY),
            )

    def test_a_window_ending_before_it_begins_is_refused(self):
        with self.assertRaises(ValueError):
            gtfs_days.window(SUNDAY, MONDAY)

    def test_the_converted_feed_against_the_published_one(self):
        with tempfile.TemporaryDirectory() as scratch:
            converted = week_feed(Path(scratch) / "converted")
            published = week_feed(Path(scratch) / "published", saturday_trip=False)
            compared = gtfs_days.compare(converted, published, MONDAY, SUNDAY)
        self.assertEqual(compared["trip_days"], {"difference": 1, "relative": 1 / 9})
        self.assertEqual(compared["trips_in_window"]["difference"], 1)
        self.assertEqual(compared["trip_days_per_day"]["2026-11-07"], 1)
        self.assertEqual(sum(compared["trip_days_per_day"].values()), 1)
        self.assertEqual(compared["trip_days_per_route_type"], {"2": 0, "3": 1})


class Measuring(unittest.TestCase):
    def test_the_peak_memory_and_the_times_of_a_process(self):
        allocate = "b = bytearray(96 * 1024 * 1024); import time; time.sleep(1.2)"
        with tempfile.TemporaryDirectory() as scratch:
            out, err = Path(scratch) / "out", Path(scratch) / "err"
            measured = netex_bench.measure([sys.executable, "-c", allocate], out, err)
        self.assertGreaterEqual(measured["peak_rss_bytes"], 96 * 1024 * 1024)
        self.assertGreaterEqual(measured["wall_s"], 1.2)
        self.assertGreater(measured["cpu_s"], 0)
        self.assertTrue(measured["rss_samples"], "the resident memory is read while it runs")
        self.assertTrue(
            all(rss <= measured["peak_rss_bytes"] for _, rss in measured["rss_samples"])
        )
        self.assertEqual(len(measured["load_before"]), 3)

    def test_the_output_goes_to_the_files_given(self):
        with tempfile.TemporaryDirectory() as scratch:
            out, err = Path(scratch) / "out", Path(scratch) / "err"
            netex_bench.measure(
                [sys.executable, "-c", "import sys; print('out'); print('err', file=sys.stderr)"],
                out,
                err,
            )
            self.assertEqual(out.read_text(), "out\n")
            self.assertEqual(err.read_text(), "err\n")

    def test_a_failing_command_raises_with_its_error_output(self):
        with tempfile.TemporaryDirectory() as scratch:
            with self.assertRaisesRegex(netex_bench.RunFailed, "exited with 3:\nbroken"):
                netex_bench.measure(
                    [sys.executable, "-c", "import sys; print('broken', file=sys.stderr); sys.exit(3)"],
                    Path(scratch) / "out",
                    Path(scratch) / "err",
                )

    def test_a_run_waits_for_the_lock_and_gives_up_after_the_time_allowed(self):
        with tempfile.TemporaryDirectory() as scratch:
            lock = Path(scratch) / "lock"
            out, err = Path(scratch) / "out", Path(scratch) / "err"
            command = [sys.executable, "-c", "pass"]
            with open(lock, "w", encoding="utf-8") as held:
                fcntl.flock(held, fcntl.LOCK_EX)
                with self.assertRaises(netex_bench.LockTimeout):
                    netex_bench.measure(command, out, err, lock=lock, lock_wait=0.3)
                self.assertFalse(out.exists(), "nothing ran without the lock")
                fcntl.flock(held, fcntl.LOCK_UN)
            measured = netex_bench.measure(command, out, err, lock=lock, lock_wait=0.3)
        self.assertLess(measured["lock_wait_s"], 0.3)

    def test_the_samples_kept_are_spread_and_keep_the_peak_and_the_end(self):
        samples = [(n / 2, n if n != 777 else 10**9) for n in range(1000)]
        kept = netex_bench.thinned(samples, 100)
        self.assertLessEqual(len(kept), 102)
        self.assertIn(samples[777], kept)
        self.assertEqual(kept[-1], samples[-1])
        self.assertEqual(kept, sorted(kept))
        self.assertEqual(netex_bench.thinned(samples[:5], 100), samples[:5])


class Sizes(unittest.TestCase):
    def test_the_xml_of_an_archive_and_of_a_directory(self):
        with tempfile.TemporaryDirectory() as scratch:
            directory = Path(scratch) / "delivery"
            directory.mkdir()
            (directory / "line-1.xml").write_text("<a/>" * 100)
            (directory / "line-2.XML").write_text("<b/>" * 50)
            (directory / "notes.txt").write_text("not a document")
            archive = zipped(directory, Path(scratch) / "delivery.zip")
            from_directory = netex_bench.input_size([directory])
            from_archive = netex_bench.input_size([archive])
            both = netex_bench.input_size([directory, archive])
        self.assertEqual(from_directory["xml_bytes"], 600)
        self.assertEqual(from_directory["documents"], 2)
        self.assertEqual(from_directory["bytes_on_disk"], 600 + len("not a document"))
        self.assertEqual(from_archive["xml_bytes"], 600)
        self.assertEqual(from_archive["documents"], 2)
        self.assertEqual(both["documents"], 4)

    def test_sizes_and_memory_in_their_units(self):
        self.assertEqual(netex_bench.size(75_093_762_780), "75.09 GB")
        self.assertEqual(netex_bench.size(16_500_000), "16.5 MB")
        self.assertEqual(netex_bench.memory(12.5 * 1024**3), "12.50 GiB")
        self.assertEqual(netex_bench.memory(300 * 1024**2), "300 MiB")
        self.assertEqual(netex_bench.seconds(95.64), "95.6 s")
        self.assertEqual(netex_bench.seconds(1260), "21.0 min")


# A stand-in for `java -jar <validator>`: like the validator it makes the
# output directory and writes its report there, with notices of each severity,
# and it keeps its arguments beside it.
FAKE_JAVA = """#!{python}
import json, os, sys
arguments = sys.argv[1:]
into = arguments[arguments.index("--output_base") + 1]
os.makedirs(into)
with open(into + "/arguments.json", "w") as kept:
    json.dump(arguments, kept)
notices = [
    {{"code": "invalid_url", "severity": "ERROR", "totalNotices": 2}},
    {{"code": "foreign_key_violation", "severity": "ERROR", "totalNotices": 5}},
    {{"code": "fast_travel_between_consecutive_stops", "severity": "WARNING", "totalNotices": 7}},
    {{"code": "unknown_file", "severity": "INFO", "totalNotices": 1}},
]
with open(into + "/report.json", "w") as report:
    json.dump({{"notices": notices}}, report)
with open(into + "/system_errors.json", "w") as errors:
    json.dump({{"notices": [{{"code": "runtime_exception_in_validator_error", "totalNotices": 1}}]}}, errors)
"""

# A validator that dies, as one out of memory does.
FAILING_JAVA = """#!{python}
import sys
print("java.lang.OutOfMemoryError: Java heap space", file=sys.stderr)
sys.exit(1)
"""


@contextlib.contextmanager
def java_on_path(directory, script):
    """`java` on the path, the given script."""
    java = directory / "java"
    directory.mkdir(parents=True, exist_ok=True)
    java.write_text(script.format(python=sys.executable))
    java.chmod(0o755)
    path = os.environ["PATH"]
    os.environ["PATH"] = f"{directory}{os.pathsep}{path}"
    try:
        yield
    finally:
        os.environ["PATH"] = path


class Validator(unittest.TestCase):
    def test_the_notices_counted_by_severity_and_code(self):
        with tempfile.TemporaryDirectory() as scratch:
            scratch = Path(scratch)
            into = scratch / "checked"
            into.mkdir()
            (into / "left-over").write_text("from an earlier run")
            with java_on_path(scratch / "bin", FAKE_JAVA):
                checked = netex_bench.validate(
                    Path("validator.jar"), scratch / "feed.zip", into, MONDAY, 8
                )
            arguments = json.loads((into / "arguments.json").read_text())
            self.assertFalse((into / "left-over").exists(), "an earlier report is cleared")
        self.assertEqual(checked["notices"], {"ERROR": 7, "WARNING": 7, "INFO": 1})
        self.assertEqual(
            list(checked["codes"]["ERROR"].items()),
            [("foreign_key_violation", 5), ("invalid_url", 2)],
        )
        self.assertEqual(checked["codes"]["WARNING"], {"fast_travel_between_consecutive_stops": 7})
        self.assertEqual(checked["system_errors"], {"runtime_exception_in_validator_error": 1})
        self.assertEqual(arguments[:2], ["-jar", "validator.jar"])
        self.assertIn("--skip_validator_update", arguments)
        self.assertEqual(arguments[arguments.index("--date") + 1], "2026-11-02")
        self.assertEqual(arguments[arguments.index("--threads") + 1], "8")


def measured_delivery():
    """A delivery as `OUT/deliveries/<id>.json` keeps it, two thread counts."""

    def summary(wall, cpu, peak):
        return {
            "wall_s": {"mean": wall, "min": wall, "max": wall},
            "cpu_s": {"mean": cpu, "min": cpu, "max": cpu},
            "peak_rss_bytes": {"mean": peak, "min": peak, "max": peak},
        }

    return {
        "id": "xx",
        "name": "Example",
        "group": "Example",
        "publisher": "Example e.V.",
        "licence": "CC0",
        "stand": "2026-11-01",
        "netex": "netex 0.3.0",
        "machine": {
            "cpu": "Example CPU",
            "logical_cpus": 64,
            "cpus_usable": 16.0,
            "memory_bytes": 256 * 1024**3,
            "memory_limit_bytes": 64 * 1024**3,
            "swap_bytes": 0,
            "kernel": "6.8.0",
            "file_systems": {},
        },
        "input": {"bytes_on_disk": 2_000_000_000, "xml_bytes": 75_000_000_000, "documents": 28_000},
        "measurements": [
            {"step": "stats", "threads": 8, "shapes": False, "summary": summary(100.0, 700.0, 12 * 1024**3)},
            {"step": "stats", "threads": 16, "shapes": False, "summary": summary(60.0, 800.0, 13 * 1024**3)},
            {"step": "convert", "threads": 8, "shapes": False, "summary": summary(150.0, 900.0, 20 * 1024**3)},
            {"step": "convert", "threads": 16, "shapes": False, "summary": summary(110.0, 1000.0, 21 * 1024**3)},
        ],
        "stats": {"journeys": 2},
        "feeds": {
            "plain": {"file": "feeds/xx.zip", "bytes": 600_000_000,
                      "rows": {"trips.txt": 1_234_567, "stop_times.txt": 30_000_000}},
        },
        "comparison": {
            "window": ["2026-11-02", "2026-11-08"],
            "converted": {"trips_in_window": 10, "trip_days": 10},
            "published": {"trips_in_window": 9, "trip_days": 9},
            "trip_days": {"difference": 1, "relative": 1 / 9},
        },
    }


class Tables(unittest.TestCase):
    def test_a_delivery_on_its_most_threads_and_over_the_threads(self):
        table = netex_bench.markdown([measured_delivery()])
        self.assertIn(
            "| Example | 2026-11-01 | 28 000 | 75.00 GB | 16 | 60.0 s | 13.00 GiB "
            "| 110.0 s | 16.7 min | 21.00 GiB | 600.0 MB | 1 234 567 | 30 000 000 |",
            table,
        )
        self.assertIn("| 8 | 100.0 s | 11.7 min | 12.00 GiB | 150.0 s | 15.0 min | 20.00 GiB |", table)
        self.assertIn("| Example | 10 | 9 | 10 | 9 | +1 (+11.11 %) |", table)
        self.assertIn("16.0 of 64 logical CPUs usable; 256.00 GiB memory, 64.00 GiB usable", table)
        self.assertNotIn("## With shapes", table)
        self.assertNotIn("## GTFS validator", table)

    def test_the_results_follow_the_order_of_the_list(self):
        with tempfile.TemporaryDirectory() as scratch:
            out = Path(scratch)
            (out / "deliveries").mkdir()
            for identifier, name in (("a", "Alpha"), ("b", "Beta")):
                delivery = dict(measured_delivery(), id=identifier, name=name)
                (out / "deliveries" / f"{identifier}.json").write_text(json.dumps(delivery))
            deliveries = netex_bench.write_results(out, ["b", "a"])
            self.assertEqual([d["name"] for d in deliveries], ["Beta", "Alpha"])
            written = json.loads((out / "results.json").read_text())
            self.assertEqual([d["id"] for d in written["deliveries"]], ["b", "a"])
            self.assertLess(
                (out / "results.md").read_text().index("| Beta |"),
                (out / "results.md").read_text().index("| Alpha |"),
            )


@unittest.skipUnless(os.environ.get("NETEX_BIN"), "NETEX_BIN names no netex command")
class WholeRun(unittest.TestCase):
    """The tool over two small deliveries of the repository: the Swiss form
    of `netex2gtfs`'s tests, compared with a feed written by hand of the one
    journey it runs and given to a validator that fails, and the crate's own
    Austrian example."""

    def test_two_deliveries_measured_converted_and_compared(self):
        listed = {
            "deliveries": [
                {
                    "id": "seetal",
                    "name": "Seetal",
                    "netex": "crates/netex2gtfs/tests/fixtures/seetal",
                    "gtfs": "published",
                    "publisher": "invented",
                    "licence": "MIT OR Apache-2.0",
                    "stand": "2026-10-12",
                    "threads": [1, 2],
                    "shapes": True,
                    "validate": True,
                },
                {
                    "id": "at",
                    "name": "Austrian example",
                    "netex": "tests/fixtures/own/at",
                    "publisher": "invented",
                    "licence": "MIT OR Apache-2.0",
                    "stand": "2026-10-12",
                    "threads": [2],
                },
            ]
        }
        with tempfile.TemporaryDirectory() as scratch:
            scratch = Path(scratch)
            data = scratch / "data"
            data.mkdir()
            for path in ("crates", "tests"):
                (data / path).symlink_to(REPOSITORY / path)
            # The journey of the delivery, as a publisher would write it: every
            # day of the week of 12 October 2026.
            write_feed(
                data / "published",
                {
                    "routes.txt": (["route_id", "route_type"], [["r", "2"]]),
                    "trips.txt": (["route_id", "service_id", "trip_id"], [["r", "daily", "j"]]),
                    "calendar.txt": (
                        CALENDAR_HEADER,
                        [["daily", "1", "1", "1", "1", "1", "1", "1", "20261012", "20261018"]],
                    ),
                },
            )
            (scratch / "list.json").write_text(json.dumps(listed))
            out = scratch / "out"
            with java_on_path(scratch / "bin", FAILING_JAVA):
                with open(os.devnull, "w") as quiet, contextlib.redirect_stderr(quiet):
                    netex_bench.run(
                        SimpleNamespace(
                            list=scratch / "list.json",
                            netex=os.environ["NETEX_BIN"],
                            data=data,
                            out=out,
                            runs=2,
                            only=[],
                            lock=scratch / "lock",
                            lock_wait=10.0,
                            window=(datetime.date(2026, 10, 12), datetime.date(2026, 10, 18)),
                            validator=Path("validator.jar"),
                        )
                    )
            seetal = json.loads((out / "deliveries" / "seetal.json").read_text())
            austrian = json.loads((out / "deliveries" / "at.json").read_text())
            table = (out / "results.md").read_text()
            with zipfile.ZipFile(out / seetal["feeds"]["plain"]["file"]) as feed:
                trips = feed.read("trips.txt").decode().strip().splitlines()

        self.assertEqual(
            [(m["step"], m["threads"], m["shapes"]) for m in seetal["measurements"]],
            [("stats", 1, False), ("stats", 2, False), ("convert", 1, False),
             ("convert", 2, False), ("convert", 2, True)],
        )
        self.assertTrue(all(len(m["runs"]) == 2 for m in seetal["measurements"]))
        self.assertEqual(seetal["input"]["documents"], 2)
        self.assertEqual(seetal["stats"]["journeys"], 1)
        self.assertEqual(seetal["stats"]["journey_days"], 7)
        self.assertEqual(seetal["feeds"]["plain"]["rows"]["trips.txt"], len(trips) - 1)
        self.assertEqual(seetal["feeds"]["plain"]["rows"]["stop_times.txt"], 3)
        self.assertIn("with_shapes", seetal["feeds"])
        self.assertEqual(seetal["comparison"]["trip_days"]["difference"], 0)
        self.assertEqual(seetal["comparison"]["converted"]["trip_days"], 7)
        self.assertNotIn("comparison", austrian)
        self.assertEqual(austrian["stats"]["journeys"], austrian["feeds"]["plain"]["rows"]["trips.txt"])
        self.assertIn("| Seetal | 1 | 1 | 7 | 7 | 0 (+0.00 %) |", table)
        # The validator failed on both feeds; the measurements stand.
        self.assertIn("OutOfMemoryError", seetal["validation"]["converted"]["failed"])
        self.assertEqual(set(seetal["validation"]), {"converted", "published"})
        self.assertIn(
            "| Seetal | converted | — | — | — | the validator failed: "
            "java.lang.OutOfMemoryError: Java heap space |",
            table,
        )
        self.assertNotIn("validation", austrian)
        self.assertLess(table.index("| Seetal |"), table.index("| Austrian example |"))


if __name__ == "__main__":
    unittest.main()
