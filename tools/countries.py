#!/usr/bin/env python3
"""Writes the borders netex2gtfs takes a delivery's country from:
`crates/netex2gtfs/src/countries.txt`.

A delivery that names no time zone, given none and whose identifiers show no
country is of the country most of its stops lie in. The borders are Natural
Earth's Admin 0 countries at 1:10m (public domain), version 5.1.2: the member
states of the European Union and of EFTA, the United Kingdom and the small
states among them, each with the time zone of its European part, cut to
Europe (the overseas parts and islands far out lie outside), every ring
simplified to 0.005° (Douglas–Peucker) and rings smaller than 0.0001 square
degrees — some 0.8 km² — left out, but a country's largest and the holes
small states make in their neighbours. At 1:10m the
coast runs up to a few kilometres off — Narvik's shore lies in its fjord,
Tromsøya is not there —: a stop there lies in no country of the table.

Only the standard library is used. Usage, from the repository root:

    curl -LO https://raw.githubusercontent.com/nvkelso/natural-earth-vector/v5.1.2/geojson/ne_10m_admin_0_countries.geojson
    python3 tools/countries.py ne_10m_admin_0_countries.geojson
"""

from __future__ import annotations

import hashlib
import json
import sys
from pathlib import Path

#: The file written, from the repository root.
OUTPUT = Path("crates/netex2gtfs/src/countries.txt")

#: The SHA-256 of the Natural Earth file the table is written from.
SOURCE_SHA256 = "239eec57ac17f100a11e2536cffc56752c318b50ae765b0918ff7aab4ce8f255"

#: Each country by its ISO 3166 code, with the time zone of its European part.
ZONES = {
    "AD": "Europe/Andorra",
    "AT": "Europe/Vienna",
    "BE": "Europe/Brussels",
    "BG": "Europe/Sofia",
    "CH": "Europe/Zurich",
    "CZ": "Europe/Prague",
    "DE": "Europe/Berlin",
    "DK": "Europe/Copenhagen",
    "EE": "Europe/Tallinn",
    "ES": "Europe/Madrid",
    "FI": "Europe/Helsinki",
    "FR": "Europe/Paris",
    "GB": "Europe/London",
    "GR": "Europe/Athens",
    "HR": "Europe/Zagreb",
    "HU": "Europe/Budapest",
    "IE": "Europe/Dublin",
    "IT": "Europe/Rome",
    "LI": "Europe/Vaduz",
    "LT": "Europe/Vilnius",
    "LU": "Europe/Luxembourg",
    "LV": "Europe/Riga",
    "MC": "Europe/Monaco",
    "MT": "Europe/Malta",
    "NL": "Europe/Amsterdam",
    "NO": "Europe/Oslo",
    "PL": "Europe/Warsaw",
    "PT": "Europe/Lisbon",
    "RO": "Europe/Bucharest",
    "SE": "Europe/Stockholm",
    "SI": "Europe/Ljubljana",
    "SK": "Europe/Bratislava",
    "SM": "Europe/San_Marino",
    "VA": "Europe/Vatican",
}

#: Europe, as west, south, east and north in degrees: a ring is kept where a
#: position of it lies within.
EUROPE = (-11.0, 34.5, 32.0, 71.5)

#: How far a simplified ring may stray from the border, in degrees.
TOLERANCE = 0.005

#: Rings smaller than this, in square degrees, are left out.
SMALLEST = 0.0001

#: The positions are written in units of this many degrees.
QUANTUM = 1e-4


def simplified(ring: list[tuple[float, float]]) -> list[tuple[float, float]]:
    """The ring by Douglas–Peucker, within `TOLERANCE`; its ends kept."""
    keep = [False] * len(ring)
    keep[0] = keep[-1] = True
    pending = [(0, len(ring) - 1)]
    while pending:
        first, last = pending.pop()
        (ax, ay), (bx, by) = ring[first], ring[last]
        dx, dy = bx - ax, by - ay
        length = dx * dx + dy * dy
        farthest, distance = None, TOLERANCE * TOLERANCE
        for at in range(first + 1, last):
            px, py = ring[at]
            t = 0.0 if length == 0 else max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / length))
            off = (px - ax - t * dx) ** 2 + (py - ay - t * dy) ** 2
            if off > distance:
                farthest, distance = at, off
        if farthest is not None:
            keep[farthest] = True
            pending += [(first, farthest), (farthest, last)]
    return [position for position, kept in zip(ring, keep) if kept]


def area(ring: list[tuple[float, float]]) -> float:
    return abs(sum(x0 * y1 - x1 * y0 for (x0, y0), (x1, y1) in zip(ring, ring[1:]))) / 2


def in_europe(ring: list[tuple[float, float]]) -> bool:
    west, south, east, north = EUROPE
    return any(west <= x <= east and south <= y <= north for x, y in ring)


def rings_of(geometry: dict) -> list[tuple[list[tuple[float, float]], bool]]:
    """Every ring of a geometry, each with whether it is a hole in a polygon."""
    polygons = geometry["coordinates"] if geometry["type"] == "MultiPolygon" else [geometry["coordinates"]]
    return [
        ([(x, y) for x, y in ring], at > 0)
        for polygon in polygons
        for at, ring in enumerate(polygon)
    ]


def line(ring: list[tuple[float, float]]) -> str:
    """A ring as latitude,longitude in quanta, each after the first as the
    difference to the one before; its closing position, the first again, left out."""
    parts, last = [], (0, 0)
    for x, y in ring[:-1]:
        here = (round(y / QUANTUM), round(x / QUANTUM))
        parts.append(f"{here[0] - last[0]},{here[1] - last[1]}")
        last = here
    return " ".join(parts)


def main(source: str) -> None:
    data = Path(source).read_bytes()
    digest = hashlib.sha256(data).hexdigest()
    if digest != SOURCE_SHA256:
        sys.exit(f"{source}: SHA-256 {digest}, not the Natural Earth file the table is written from")
    # A code can stand for several features: France's for Clipperton Island
    # too, which lies outside Europe.
    features: dict[str, list[dict]] = {}
    for feature in json.loads(data)["features"]:
        code = feature["properties"]["ISO_A2_EH"]
        if code in ZONES:
            features.setdefault(code, []).append(feature)
    missing = sorted(ZONES.keys() - features.keys())
    if missing:
        sys.exit(f"{source}: no border of {', '.join(missing)}")
    lines = [
        "# The countries a stop of a delivery can lie in, each with the time zone",
        "# of its European part: Natural Earth's Admin 0 countries at 1:10m,",
        "# version 5.1.2 (public domain), cut to Europe and simplified to 0.005°,",
        "# written by tools/countries.py. A line naming a country and its zone, then",
        "# its rings, a line each: latitude,longitude in ten-thousandths of a degree,",
        "# each position after the first as the difference to the one before.",
    ]
    vertices = 0
    for code in sorted(ZONES):
        lines.append(f"{code} {ZONES[code]}")
        rings = [
            (ring, hole)
            for feature in features[code]
            for ring, hole in rings_of(feature["geometry"])
            if in_europe(ring)
        ]
        largest = max((ring for ring, _ in rings), key=area)
        for ring, hole in rings:
            kept = simplified(ring)
            # A small state keeps its border, and a country the hole a small
            # state makes in it: the Vatican is smaller than any island kept.
            if hole or ring is largest:
                if len(kept) < 4:
                    kept = ring
            elif len(kept) < 4 or area(ring) < SMALLEST:
                continue
            vertices += len(kept) - 1
            lines.append(line(kept))
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"{OUTPUT}: {len(ZONES)} countries, {vertices} positions, {OUTPUT.stat().st_size} bytes")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    main(sys.argv[1])
