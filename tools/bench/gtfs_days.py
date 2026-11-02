#!/usr/bin/env python3
"""Trips and trip days of a GTFS feed in a window of days, and two feeds
compared by them.

A trip counts on a day its service runs: the weekdays of `calendar.txt`
between its first and last date, with the dates `calendar_dates.txt` adds and
removes. A trip of `frequencies.txt` counts once a day, as the feed states it
once; the rows of `frequencies.txt` are counted beside, so that a difference
they make can be told. `stop_times.txt` is not read.

This is the check a conversion is held to: the feed written from a NeTEx
delivery against the GTFS feed its publisher writes from the same data — the
same trips on the same days, or a difference that can be explained.

Only the standard library is used. Usage:

    python3 tools/bench/gtfs_days.py days FEED FIRST LAST
    python3 tools/bench/gtfs_days.py compare CONVERTED PUBLISHED FIRST LAST

`FEED` is a zip archive or a directory; `FIRST` and `LAST` are `YYYY-MM-DD`.
Both print JSON.
"""

import argparse
import collections
import csv
import datetime
import io
import json
import sys
import zipfile
from pathlib import Path

WEEKDAYS = ("monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday")


def rows(feed, name):
    """The rows of the file `name` of a feed, as dictionaries; none where the
    feed has no such file."""
    feed = Path(feed)
    if feed.is_dir():
        path = feed / name
        if not path.exists():
            return
        with path.open(encoding="utf-8-sig", newline="") as text:
            yield from csv.DictReader(text)
        return
    with zipfile.ZipFile(feed) as archive:
        if name not in archive.namelist():
            return
        with archive.open(name) as raw:
            yield from csv.DictReader(io.TextIOWrapper(raw, encoding="utf-8-sig", newline=""))


def gtfs_date(text):
    """A GTFS date, `YYYYMMDD`."""
    return datetime.date(int(text[:4]), int(text[4:6]), int(text[6:8]))


def window(first, last):
    """The days from `first` to `last`, both counted."""
    if last < first:
        raise ValueError(f"the window ends ({last}) before it begins ({first})")
    return [first + datetime.timedelta(days=n) for n in range((last - first).days + 1)]


def service_days(feed, days):
    """The days of `days` each service runs on, by service identifier."""
    running = collections.defaultdict(set)
    for row in rows(feed, "calendar.txt"):
        start, end = gtfs_date(row["start_date"]), gtfs_date(row["end_date"])
        weekdays = [row[weekday] == "1" for weekday in WEEKDAYS]
        running[row["service_id"]].update(
            day for day in days if start <= day <= end and weekdays[day.weekday()]
        )
    in_window = set(days)
    for row in rows(feed, "calendar_dates.txt"):
        day = gtfs_date(row["date"])
        if day in in_window:
            if row["exception_type"] == "1":
                running[row["service_id"]].add(day)
            else:
                running[row["service_id"]].discard(day)
    return running


def count(feed, first, last):
    """What a feed runs from `first` to `last`: its trips, those running in the
    window, their trip days in all, by day and by route type, its stops by
    location type and its rows of `frequencies.txt`."""
    days = window(first, last)
    running = service_days(feed, days)
    route_types = {row["route_id"]: row.get("route_type", "") for row in rows(feed, "routes.txt")}
    trips = trips_in_window = trip_days = 0
    per_day = collections.Counter()
    per_route_type = collections.Counter()
    for row in rows(feed, "trips.txt"):
        trips += 1
        on = running.get(row["service_id"], ())
        if on:
            trips_in_window += 1
            trip_days += len(on)
            per_route_type[route_types.get(row["route_id"], "")] += len(on)
            for day in on:
                per_day[day.isoformat()] += 1
    stops = collections.Counter(
        row.get("location_type") or "0" for row in rows(feed, "stops.txt")
    )
    return {
        "window": [first.isoformat(), last.isoformat()],
        "trips": trips,
        "trips_in_window": trips_in_window,
        "trip_days": trip_days,
        "trip_days_per_day": {day.isoformat(): per_day[day.isoformat()] for day in days},
        "trip_days_per_route_type": dict(sorted(per_route_type.items())),
        "stops_per_location_type": dict(sorted(stops.items())),
        "frequencies": sum(1 for _ in rows(feed, "frequencies.txt")),
    }


def difference(converted, published):
    """`converted` less `published`, and the share of `published` it is."""
    return {
        "difference": converted - published,
        "relative": (converted - published) / published if published else None,
    }


def compare(converted, published, first, last):
    """Two feeds counted over the same window, and what the converted one has
    more (or less) than the published one, in all, by day and by route type."""
    ours = count(converted, first, last)
    theirs = count(published, first, last)
    route_types = sorted(
        set(ours["trip_days_per_route_type"]) | set(theirs["trip_days_per_route_type"])
    )
    return {
        "window": ours["window"],
        "converted": ours,
        "published": theirs,
        "trips_in_window": difference(ours["trips_in_window"], theirs["trips_in_window"]),
        "trip_days": difference(ours["trip_days"], theirs["trip_days"]),
        "trip_days_per_day": {
            day: ours["trip_days_per_day"][day] - theirs["trip_days_per_day"][day]
            for day in ours["trip_days_per_day"]
        },
        "trip_days_per_route_type": {
            route_type: ours["trip_days_per_route_type"].get(route_type, 0)
            - theirs["trip_days_per_route_type"].get(route_type, 0)
            for route_type in route_types
        },
    }


def main(arguments):
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    commands = parser.add_subparsers(dest="command", required=True)
    days = commands.add_parser("days", help="the trips and trip days of one feed")
    days.add_argument("feed")
    compared = commands.add_parser("compare", help="a converted feed against a published one")
    compared.add_argument("converted")
    compared.add_argument("published")
    for command in (days, compared):
        command.add_argument("first", type=datetime.date.fromisoformat)
        command.add_argument("last", type=datetime.date.fromisoformat)
    parsed = parser.parse_args(arguments)
    if parsed.command == "days":
        result = count(parsed.feed, parsed.first, parsed.last)
    else:
        result = compare(parsed.converted, parsed.published, parsed.first, parsed.last)
    json.dump(result, sys.stdout, indent=2, ensure_ascii=False)
    print()


if __name__ == "__main__":
    main(sys.argv[1:])
