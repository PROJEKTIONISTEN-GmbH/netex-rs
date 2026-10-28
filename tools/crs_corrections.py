#!/usr/bin/env python3
"""The datum corrections behind `src/crs/corrections.rs`, and the reference
points `tests/fixtures/crs/reference.tsv` checks the `crs` feature against.

Two of the reference systems the feature reads are older than the satellite
datums: MGI (Austria) and CH1903 (Switzerland's LV03). Their official
transformations to ETRS89 are grids — the BEV's AT_GIS_GRID and swisstopo's
CHENyx06 — because the old networks are distorted by up to a metre and a half
against any single Helmert transformation. The crate holds no grid: it applies
the EPSG Helmert transformation and then a polynomial of the remaining offset,
fitted here to the grid at the stops of the country — the places a timetable
has positions for. Half the stops fit the polynomial; the other half check it.

The reference points are positions the publishers state, turned into each
system with PROJ and its national grid where it has one:
  - Switzerland: the service points of the Swiss register (opentransportdata.swiss),
    each stated in LV95 and in WGS 84 by the publisher; LV03 from the WGS 84
    position through CHENyx06.
  - Austria: the stops of the Austrian register (data.gv.at, CC BY 4.0), in
    WGS 84; MGI Lambert and Gauss-Krüger M31 through AT_GIS_GRID.
  - The Netherlands: a lattice over the country, RD New through RDNAPTRANS2018.
  - France and the UTM zones: a lattice, a projection of ETRS89 alone.
ETRS89 is taken for WGS 84, as every timetable of the three countries does
(the two differ by less than a metre in 2026, the same everywhere in Europe).

Needs: pyproj and numpy; the grids at_bev_AT_GIS_GRID_2021_09_28.tif,
ch_swisstopo_CHENyx06_ETRS.tif and nl_nsgi_rdtrans2018.tif (cdn.proj.org) in
the directory PROJ_GRIDS names; the two registers as below.

    PROJ_GRIDS=/path/to/grids python3 tools/crs_corrections.py \
        --austria haltestellen.csv --switzerland actual-date-swiss-service-point.csv
"""

import argparse
import csv
import os
import sys

import numpy as np
import pyproj
from pyproj import CRS
from pyproj.transformer import TransformerGroup

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DEGREE = 7
METRES_PER_DEGREE = 111_195.0


def transformer(source, target, grid):
    """The best transformation PROJ knows from source to target: through the
    national grid where `grid`, else the first it lists (Helmert)."""
    group = TransformerGroup(CRS.from_epsg(source), CRS.from_epsg(target), always_xy=True)
    for candidate in group.transformers:
        if ("hgridshift" in candidate.definition) == grid:
            return candidate
    sys.exit(f"no {'grid' if grid else 'Helmert'} transformation from EPSG:{source} to EPSG:{target}")


def metres(dlon, dlat, lat):
    return np.hypot(dlat * METRES_PER_DEGREE, dlon * METRES_PER_DEGREE * np.cos(np.radians(lat)))


def terms(lon, lat, centre):
    u, v = lon - centre[0], lat - centre[1]
    return np.stack([u**i * v**j for i in range(DEGREE + 1) for j in range(DEGREE + 1 - i)], 1)


def fit(name, epsg, lon, lat):
    """The polynomial of the offset left after the Helmert transformation,
    over the positions the Helmert transformation gives, fitted on every
    other stop; the worst and mean offset left at the others."""
    grid = transformer(4258, epsg, grid=True)
    helmert = transformer(epsg, 4326, grid=False)
    x, y = grid.transform(lon, lat)
    ok = np.isfinite(x) & np.isfinite(y)
    lon, lat, x, y = lon[ok], lat[ok], x[ok], y[ok]
    hlon, hlat = helmert.transform(x, y)
    order = np.random.default_rng(1321).permutation(len(lon))
    fitting, checking = order[::2], order[1::2]
    centre = (round(float(hlon.mean()), 2), round(float(hlat.mean()), 2))
    design = terms(hlon[fitting], hlat[fitting], centre)
    dlon = np.linalg.lstsq(design, (lon - hlon)[fitting], rcond=None)[0]
    dlat = np.linalg.lstsq(design, (lat - hlat)[fitting], rcond=None)[0]
    check = terms(hlon[checking], hlat[checking], centre)
    before = metres((lon - hlon)[checking], (lat - hlat)[checking], lat[checking])
    after = metres(
        (lon - hlon)[checking] - check @ dlon,
        (lat - hlat)[checking] - check @ dlat,
        lat[checking],
    )
    print(
        f"{name}: {len(checking)} stops checked; Helmert alone {before.max():.3f} m worst, "
        f"{before.mean():.3f} m mean; corrected {after.max():.3f} m worst, {after.mean():.3f} m mean",
        file=sys.stderr,
    )
    return centre, dlon, dlat, (lon[checking], lat[checking])


def rust_table(name, doc, centre, dlon, dlat):
    def numbers(values):
        # The shortest text that reads back as the same number.
        return "\n".join(f"        {float(value)!r}," for value in values)

    return f"""{doc}
pub(super) const {name}: Correction = Correction {{
    centre: ({centre[0]!r}, {centre[1]!r}),
    degree: {DEGREE},
    longitude: &[
{numbers(dlon)}
    ],
    latitude: &[
{numbers(dlat)}
    ],
}};
"""


def austria(path):
    lon, lat = [], []
    with open(path, encoding="utf-8") as file:
        for row in csv.DictReader(file):
            try:
                lon.append(float(row["hst_x"]))
                lat.append(float(row["hst_y"]))
            except ValueError:
                continue
    return np.array(lon), np.array(lat)


def switzerland(path):
    rows = []
    with open(path, encoding="utf-8-sig") as file:
        for row in csv.DictReader(file, delimiter=";"):
            if row["isoCountryCode"] != "CH" or not row["lv95East"] or not row["wgs84East"]:
                continue
            rows.append(
                (float(row["lv95East"]), float(row["lv95North"]), float(row["wgs84East"]), float(row["wgs84North"]))
            )
    return np.array(rows)


def sample(count, *columns):
    """`count` rows spread over the list, the same every run."""
    picked = np.linspace(0, len(columns[0]) - 1, count).astype(int)
    return [column[picked] for column in columns]


def lattice(west, east, south, north, count=7):
    lon, lat = np.meshgrid(np.linspace(west, east, count), np.linspace(south, north, count))
    return lon.ravel(), lat.ravel()


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--austria", required=True, help="haltestellen.csv of the Austrian register")
    parser.add_argument("--switzerland", required=True, help="the Swiss service point register")
    arguments = parser.parse_args()
    pyproj.datadir.append_data_dir(os.environ["PROJ_GRIDS"])

    at_lon, at_lat = austria(arguments.austria)
    swiss = switzerland(arguments.switzerland)
    mgi = fit("MGI", 31287, at_lon, at_lat)
    ch1903 = fit("CH1903", 21781, swiss[:, 2], swiss[:, 3])

    header = """// Generated by tools/crs_corrections.py — do not edit by hand.
//
// What is left of the official grid transformation to ETRS89 after the EPSG
// Helmert transformation, as a polynomial of longitude and latitude, fitted at
// the stops of the country. Derived from the BEV's AT_GIS_GRID (CC BY 4.0,
// © BEV) and swisstopo's CHENyx06 (free use, © swisstopo), and from the stops
// of the Austrian register (data.gv.at, CC BY 4.0) and the Swiss register
// (opentransportdata.swiss).

use super::Correction;
"""
    tables = [
        rust_table("MGI", "/// MGI (Austria): after the Helmert transformation MGI to WGS 84 (3), against AT_GIS_GRID.", *mgi[:3]),
        rust_table("CH1903", "/// CH1903 (Switzerland, LV03): after the translation CH1903 to WGS 84 (2), against CHENyx06.", *ch1903[:3]),
    ]
    with open(os.path.join(ROOT, "src/crs/corrections.rs"), "w", encoding="utf-8") as file:
        file.write(header + "\n" + "\n".join(tables))

    lines = [
        "# Reference points of the crs feature: EPSG code, the two coordinates as a position in that",
        "# system is written (its axis order), latitude and longitude in WGS 84 (ETRS89), where from.",
        "# Generated by tools/crs_corrections.py from the Swiss service point register",
        "# (opentransportdata.swiss), the Austrian stop register (data.gv.at, CC BY 4.0) and PROJ.",
    ]

    def emit(epsg, first, second, lat, lon, origin):
        for a, b, la, lo in zip(first, second, lat, lon):
            lines.append(f"{epsg}\t{a:.3f}\t{b:.3f}\t{la:.8f}\t{lo:.8f}\t{origin}")

    lv95_e, lv95_n, swiss_lon, swiss_lat = sample(40, *swiss.T)
    emit(2056, lv95_e, lv95_n, swiss_lat, swiss_lon, "Swiss register")
    lv03 = transformer(4258, 21781, grid=True)
    x, y = lv03.transform(swiss_lon, swiss_lat)
    emit(21781, x, y, swiss_lat, swiss_lon, "Swiss register, LV03 through CHENyx06")
    at_check_lon, at_check_lat = sample(40, *mgi[3])
    for epsg in (31287, 31258):
        # Both are written northing first.
        x, y = transformer(4258, epsg, grid=True).transform(at_check_lon, at_check_lat)
        emit(epsg, y, x, at_check_lat, at_check_lon, "Austrian register through AT_GIS_GRID")
    nl_lon, nl_lat = lattice(3.6, 7.0, 50.8, 53.4)
    x, y = transformer(4258, 28992, grid=True).transform(nl_lon, nl_lat)
    emit(28992, x, y, nl_lat, nl_lon, "lattice, RDNAPTRANS2018")
    for epsg, box in ((2154, (-4.5, 8.0, 42.5, 51.0)), (25832, (6.0, 12.0, 47.5, 55.0)), (25833, (12.0, 18.0, 47.5, 55.0))):
        lon, lat = lattice(*box)
        x, y = transformer(4258, epsg, grid=False).transform(lon, lat)
        emit(epsg, x, y, lat, lon, "lattice, projection alone")
    path = os.path.join(ROOT, "tests/fixtures/crs/reference.tsv")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "w", encoding="utf-8") as file:
        file.write("\n".join(lines) + "\n")


if __name__ == "__main__":
    main()
