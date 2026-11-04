# netex-rs

[![crates.io](https://img.shields.io/crates/v/netex-rs.svg)](https://crates.io/crates/netex-rs)
[![docs.rs](https://docs.rs/netex-rs/badge.svg)](https://docs.rs/netex-rs)

Read, check, convert and write **NeTEx** — the European standard for public
transport timetables and networks, CEN/TS 16614 — in Rust, from one document
up to a whole country's delivery.

## Why

Across the EU, the national access points publish timetables as NeTEx
(Delegated Regulation (EU) 2017/1926): Germany's complete timetable comes from
DELFI as NeTEx, and so do those of Austria, Switzerland, South Tyrol, France,
Norway, Sweden, Finland, Denmark, Luxembourg, Belgium, Czechia and the
Netherlands. Using that data means getting past three things: **size** — a
national delivery is tens of thousands of XML files, Germany's 75 GB —;
**interpretation** — references cross files, the days a journey runs follow
rules spread over several elements, and every country writes a profile of its
own —; and **two releases**, NeTEx 1.x and 2.0, which share one namespace.

netex-rs handles that, so that you can

- **turn a whole national delivery into GTFS**, for software that reads only
  GTFS — Germany's in about five minutes in under 13 GiB of memory (below);
- **build a journey planner, a departure board or a stop finder** on the
  official data: stops with their quays and levels, lines, journeys with their
  calls and the days they run, walks and planned interchanges, resolved across
  the delivery;
- **check a delivery** before you publish or import it: every reference to
  nothing, every contradiction and every decision the reader takes is a
  finding that names the file and the object;
- **cut out and combine**: a region or an operator out of a national
  delivery, several deliveries into one;
- **write NeTEx** for either release, with a report of everything the other
  release cannot hold, and **follow the regular updates** by what changed
  since the last delivery.

Nothing the model does not name is lost: every element and attribute it does
not type — fares, new modes, rarer children — is carried through as it was
written and written back there.

## Install

```sh
cargo install netex-cli    # the `netex` command
cargo add netex-rs         # the library
cargo add netex2gtfs       # NeTEx → GTFS as a library
```

Licence MIT OR Apache-2.0 · MSRV Rust 1.86 · no `unsafe`. The repository holds
three packages: **netex-rs**, the library; [netex2gtfs](crates/netex2gtfs/README.md),
a delivery as a GTFS feed; [netex-cli](crates/netex-cli/README.md), the `netex`
command.

## In five minutes

A delivery is a zip archive or a directory of NeTEx documents, read on every
core. The commands below run in a clone of this repository, on its own small
example of the Nordic profile: four line documents and the stop register they
name, published apart (`--with`).

**Check it** — every finding by class, with examples and the journeys it
affects; exit status 0 when the delivery is accepted, 1 when the policy given
(`--refuse`, `--at-most`, `--strict`) refuses it, `--format json` for a machine:

```sh
netex validate tests/fixtures/own/nordic --with tests/fixtures/own/nordic-stops
```

```text
nordic: 5 documents, 11 stops, 5 stop points, 3 lines, 4 journeys, 4 calendars, 60 references
1 × flexible-line-journey — journey of a flexible line, not runs at fixed times: left out
    ServiceJourney EX:ServiceJourney:hent in EX_FlexibleLine-hent_Hent-meg-Eksempelby.xml (EX:FlexibleLine:hent)
6 × unreferenced — definition nothing refers to
    StopPlace NSR:StopPlace:900 in the delivery
    …
accepted
```

**Count it** — stops, lines, journeys and journey days, in all and by
operator, over the whole calendar or `--from`/`--to`:

```sh
netex stats tests/fixtures/own/nordic --with tests/fixtures/own/nordic-stops
```

```text
nordic — the whole calendar
all: 5 stops, 2 lines, 4 journeys, 48 journey days, 2026-10-01 to 2026-10-30
Eksempel Trafikk (EX:Operator:1): 5 stops, 2 lines, 4 journeys, 48 journey days, 2026-10-01 to 2026-10-30
```

**Convert it to GTFS** — a zip archive or a directory; the rows written, every
decision taken and everything the feed does not carry, `--report` as JSON:

```sh
netex convert tests/fixtures/own/nordic --with tests/fixtures/own/nordic-stops --to gtfs gtfs.zip
```

```text
agency.txt: 1 rows
…
stop_times.txt: 8 rows
stops.txt: 8 rows
transfers.txt: 1 rows
trips.txt: 4 rows
1 × agency-without-url: an organisation stating no web address: the publisher's is written, GTFS requires one
1 × feed-info-without-publisher: neither a publisher given nor a data source with a web address in the delivery: no feed_info.txt
5 × stop-named-by-its-station: a stop without a name: named as its stop place
not carried: 1 × FlexibleLine (first in EX_FlexibleLine-hent_Hent-meg-Eksempelby.xml)
not carried: 1 × Network (first in _EX_shared_data.xml)
```

**Cut out a part** — the journeys of operators, lines, an area (`--bbox`,
`--area` GeoJSON) and a period, with everything they refer to, as NeTEx of
the source's release:

```sh
netex extract tests/fixtures/own/nordic --with tests/fixtures/own/nordic-stops --line EX:Line:1 --to part
```

```text
tests/fixtures/own/nordic → part: 3 of 5 documents, 2 journeys chosen, 2 written, 27 objects
1 × reference-to-nothing: a reference of an object taken to an identifier no document defines: written as it was
    EX:Network:1
```

`netex merge` combines several deliveries into one. Every command, option and
finding: [netex-cli](crates/netex-cli/README.md).

## As a library

The journeys a delivery runs on one day, with the stops they call at:

```rust
use netex_rs::dataset::Source;
use netex_rs::index::{Index, LineId};
use netex_rs::model::Date;
use netex_rs::stream::{ReadOptions, Selection};

fn main() -> Result<(), netex_rs::Error> {
    // A zip archive or a directory of NeTEx documents, read on every core.
    let source = Source::open("tests/fixtures/own/ch")?;
    let index = Index::build(&source, &ReadOptions::new(Selection::routing()))?;
    let day = Date::from("2026-10-14");
    for line in (0..index.line_count() as u32).map(LineId) {
        for journey in index.journeys_of_line_on(line, &day) {
            let journey = index.journey(journey);
            let stops: Vec<&str> =
                journey.stop_points.iter().map(|&p| index.stop_point(p).name).collect();
            println!("{}: {}", index.line(line).name, stops.join(" → "));
        }
    }
    for finding in index.findings() {
        eprintln!("{finding}"); // "point passed through, no call: Call … in ch-timetable.xml (…)"
    }
    Ok(())
}
```

`cargo run --example timetable -- tests/fixtures/own/ch 2026-10-14` runs the
same with times and headways. An importer that needs no model of every
document reads entity by entity — `stream::read` types only the kinds a
`Selection` names and skips the rest at the tokenizer:

```rust,no_run
use netex_rs::stream::{self, Entity, Kind, ReadOptions, Selection};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let xml = std::fs::read_to_string("line.xml")?;
    let options = ReadOptions::new(Selection::none().with(Kind::ServiceJourney));
    stream::read(&xml, &options, |entity, scope| {
        if let Entity::ServiceJourney(journey) = entity {
            let times = journey.passing_times.as_ref().map_or(0, |p| p.passing_times.len());
            let frame = scope.frames().last().map(|frame| &frame.id);
            println!("a journey with {times} passing times in frame {frame:?}");
        }
        Ok(())
    })?;
    Ok(())
}
```

Reading a single document into the typed model and writing it back, for
either release, is the first example of the [crate documentation](https://docs.rs/netex-rs);
a delivery as a GTFS feed from code is [netex2gtfs](crates/netex2gtfs/README.md)'s.
`Source::manifest` fingerprints a delivery from the archive's central directory
and `Manifest::diff` says which files are new, changed or gone since the last
one.

## Profiles

The crate reads the documents of any profile into the same model; the index
applies the rules each profile relies on — calendars per document, lines in
versions, dated journeys, stop registers published apart, time demand types
instead of passing times, national reference systems. Each has been read on
its publishers' deliveries:

| Profile | Deliveries read | In detail |
|---|---|---|
| DELFI (Germany) | the national delivery, every file | [DELFI-PROFILE.md](docs/DELFI-PROFILE.md) |
| Austria | all nine deliveries of the national access point | [INTERPRETATION.md](docs/INTERPRETATION.md) |
| Switzerland | the national timetable | [INTERPRETATION.md](docs/INTERPRETATION.md) |
| IT-ITH1 (Italy, South Tyrol) | South Tyrol's delivery at the Italian access point | [IT-ITH1-PROFILE.md](docs/IT-ITH1-PROFILE.md) |
| NeTEx France | five deliveries: Fluo Grand Est (Bas-Rhin), Naolib, TBM, Normandy's aggregate, SNCF | [FR-PROFILE.md](docs/FR-PROFILE.md) |
| Nordic NeTEx Profile | the national deliveries of Norway, Sweden and Finland, with their stop registers | [NORDIC-PROFILE.md](docs/NORDIC-PROFILE.md) |
| EPIP (European Passenger Information Profile) | Luxembourg, Denmark, the Belgian operators SNCB, De Lijn, TEC, STIB, Czechia | [EPIP-NEIGHBOURS.md](docs/EPIP-NEIGHBOURS.md) |
| Netherlands (BISON) | six operators with the national stop register | [NL-BISON-PROFILE.md](docs/NL-BISON-PROFILE.md) |

How each lays out its deliveries and what the crate reads for it:
[docs/PROFILES.md](docs/PROFILES.md). NeTEx leaves open how a reader turns
what a delivery states into a timetable — a reference to nothing, a stop place
defined in three hundred documents, a point a train passes through;
[docs/INTERPRETATION.md](docs/INTERPRETATION.md) lists these questions with
what the standard says and what this crate does, and why.
`tests/fixtures/own/` holds a small invented delivery of each profile.

## Performance

Every national delivery below reads whole and converts whole to GTFS with
`netex convert --to gtfs`, without error. Measured with
[`tools/bench/`](tools/bench/README.md) on an AMD EPYC 7502P host, 16 of its
64 hardware threads and 64 GiB granted to the container, deliveries on an
NVMe SSD; netex-cli 0.3.0 (netex-rs 0.8.1, the code of 0.8.2), release
build, 16 threads; wall clock and peak resident memory of the
process, the mean of two runs. The host ran other work (load 3.5 to 42);
every run held a lock against other measurements on it. Where a country
publishes several deliveries, their times are summed and the memory is the
largest one's.

| Delivery | Documents | XML | Read (`stats`) | NeTEx → GTFS | Peak memory | Trips | Stop times |
|---|---:|---:|---:|---:|---:|---:|---:|
| Germany | 28 190 | 74.7 GB | 65 s | 5.3 min | 12.8 GiB | 4 838 062 | 99 495 018 |
| Switzerland | 521 | 39.3 GB | 58 s | 2.7 min | 16.9 GiB | 2 583 297 | 41 602 402 |
| Belgium, four operators | 5 145 | 27.5 GB | 34 s | 89 s | 6.1 GiB | 788 905 | 20 810 951 |
| Austria, nine deliveries | 3 765 | 16.2 GB | 19 s | 68 s | 2.0 GiB | 1 145 569 | 20 610 392 |
| Czechia, three deliveries | 27 508 | 10.1 GB | 13 s | 12 s | 1.6 GiB | 0 | 0 |
| Finland | 6 098 | 6.3 GB | 8 s | 49 s | 1.9 GiB | 521 058 | 15 831 600 |
| Sweden | 6 903 | 6.1 GB | 8 s | 34 s | 1.8 GiB | 438 778 | 9 702 256 |
| Norway | 4 818 | 5.7 GB | 8 s | 29 s | 1.9 GiB | 374 843 | 8 986 827 |
| Denmark | 1 560 | 3.9 GB | 3 s | 15 s | 1.1 GiB | 196 818 | 4 690 714 |
| France, five deliveries | 1 047 | 2.8 GB | 9 s | 26 s | 0.7 GiB | 311 926 | 7 052 810 |
| Netherlands, six operators and the stop register | 7 | 1.1 GB | 12 s | 27 s | 0.6 GiB | 292 515 | 6 072 223 |
| Luxembourg | 713 | 0.7 GB | 0.7 s | 3 s | 0.2 GiB | 40 289 | 990 029 |

- **Germany** converts on one, four, eight and sixteen threads in 13.5, 7.0,
  5.8 and 5.3 minutes. Reading scales with the threads (9.5 minutes on one,
  65 s on sixteen); writing the feed, about four minutes, ran on one, most
  of it deflating the zip archive. Since 0.9.0 each file of the archive is
  deflated on every thread: on eight threads Germany converts in 200 s
  instead of 364 s, Austria's nine deliveries in 39 s instead of 68 s —
  measured before and after on one machine, the feed the same bytes on one,
  eight and sixteen threads. Its peak, 11.1 to
  12.8 GiB by the threads, is the reading's: converting needs no more memory
  than reading alone, and the feed is written from 6 to 8 GiB.
- **Switzerland** needed 16.9 GiB on sixteen threads, 12.4 GiB on one. Since
  0.10.0 the threads' builders are merged without copying their strings, and
  a call's identifier is kept only where a finding names it: it converts in
  7.4 to 7.5 GiB on one to sixteen threads — measured before and after on one
  machine, the feed the same bytes.
- **Czechia** reads whole but converts to an empty feed: no stop of its
  deliveries states a position, and GTFS requires one.
- **Each feed against its publisher's GTFS**, trip days of a week: Norway,
  TEC, STIB and three French networks equal, De Lijn four trip days apart;
  Sweden, Denmark, Switzerland and SNCB within 0.84 %, NeTEx and GTFS one to
  eleven days apart; Germany +3.06 %, 616 lines DELFI's own GTFS export does
  not carry; Finland +26.95 %, a defect of its delivery that
  `netex validate` reports.
- **The GTFS validator** of MobilityData finds no error in Germany's feed
  written with the publisher given (`--publisher-name`, `--publisher-url`;
  DELFI's own GTFS: 22 027), and nine in Norway's (Entur's: 23), from what
  the delivery states.

Every delivery and every comparison with its explanation, the curves over the
threads, where the memory goes, and what 0.9.0 changed:
[docs/PERFORMANCE.md](docs/PERFORMANCE.md).

## NeTEx → GTFS: what the feed carries, and what not

`netex convert --to gtfs` (and [netex2gtfs](crates/netex2gtfs/README.md), the
library behind it) writes from the resolved delivery:

| GTFS | From NeTEx |
|---|---|
| `agency.txt` | the operator of each line, else its additional operator, else its authority; the time zone a document names, else that of the country the identifiers or the stops show |
| `stops.txt`, `levels.txt` | stop places as stations, their quays as stops under them, stop points no stop is assigned as stops of their own; accessibility, platform codes, levels |
| `routes.txt` | lines — a line's versions are one route —, their mode as a route type, their colours |
| `trips.txt`, `stop_times.txt` | journeys with their calls and passing times, the destination shown, train numbers, direction, accessibility; boarding and alighting; a point passed through is no call |
| `calendar.txt`, `calendar_dates.txt` | the days each journey runs, by the calendar rules of the delivery's profile |
| `frequencies.txt` | template journeys run at a headway |
| `transfers.txt` | walks between stops and planned interchanges (staying seated, guaranteed) |
| `shapes.txt` | the course a trip's journey names, as Bordeaux's deliveries state one for each journey, else the geometry of the service links it runs over (`--shapes`), those its pattern names first: the geometry a link states on itself, as Austria's and Denmark's deliveries do, or in its link sequence projection, as Norway's, Sweden's and Finland's |
| `feed_info.txt`, `attributions.txt` | the publisher given, the delivery's data sources and participants |

Positions in a national reference system are written in WGS 84. GlobalIDs
(DHIDs) or SLOIDs can be the stop identifiers (`--stop-ids global`).

What a feed **does not** carry, and the report counts with the first document
holding it: **fares** (NeTEx Part 3; no `fare_*` files), **flexible and
on-demand lines** (no GTFS-Flex; their journeys are left out), **notices**,
interchange rules, **station paths and entrances** (no `pathways.txt`),
names in other languages (no `translations.txt`), vehicle types and blocks.
What the conversion decides on its way — a call at a stop point placed
nowhere, a line of a mode GTFS has no type for, a walk stated twice — is a
decision in the same report, counted with an example.

## Conformance

The conformance harness (`tests/conformance.rs`) reads every official example
of both releases, writes it back, compares the two element by element, and
validates what was written against the release's schemas with `xmllint`. Then
it crosses the releases.

| | 1.x (branch `v1.3`) | 2.0 (branch `v2.0`) |
|---|---|---|
| NeTEx example documents | 245 | 272 (3 of them `should-fail`) |
| round-trip, element for element | 245 / 245 | 272 / 272 |
| … read into the typed model / carried through as fragments | 229 / 16 | 257 / 15 |
| originals the validator accepts | 238 | 269 |
| written documents that validate again | 238 / 238 | 269 / 269 |
| written for the other release: lossless / with reported losses | 237 / 1 | 157 / 112 |
| … and valid there | 120 of 238 | 148 of 269 |
| release detection contradicting the validator | 0 | 0 |

The releases tell apart by the elements, attributes and enumeration tokens
one declares and the other does not — a table derived from both schema sets
(`docs/VERSIONS.md`), as the `version` a document declares cannot be trusted.
A 2.0 document written for 1.x loses what 1.x does not have (`privateCodes`,
`FarePriceFrame`, `deckPlans`, …), every loss in the report; the crate does not
invent identifiers or tokens to satisfy the other release's validator.

## Limits

- **The index is in memory.** A whole national delivery needs the memory
  the table under Performance says — Germany 12.8 GiB, Switzerland 16.9 GiB
  on sixteen threads; since 0.10.0, on eight, Germany 11.6 GiB, Switzerland
  7.4 GiB —; the lean stream without an index needs well under a gigabyte.
- **Typed where routing and GTFS need it.** Fares (Part 3), vehicle schedules
  and many rarer elements are carried through untyped: kept and written back,
  not modelled. Flexible (on-demand) lines are not indexed; their journeys are
  left out and counted.
- **Reference systems behind a feature.** Positions in a national system —
  LV95/LV03, MGI, RD, Lambert-93, UTM — are turned into WGS 84 with the feature
  `crs` (`netex_rs::crs`), which netex2gtfs and netex-cli turn on.
- **No profile-to-profile conversion.** NeTEx is written back in either
  release, not rewritten into another profile (no EPIP writer);
  `docs/PROFILE-UEBERFUEHRUNG.md` says what that would gain and lose.
- **No schema validation at run time.** The crate reads what is well-formed
  within its limits; validity against the schemas is checked by its tests.
- **Zip archives with deflate** (or stored) entries, and directories; XML 1.0
  in UTF-8. Entities are never expanded, DTDs never fetched; nesting depth,
  text length, attribute count and fragment size are bounded (`Limits`). Fuzz
  targets under `fuzz/` run the reader, the release rewrite, the conversion
  and the document outline.

## Development

```sh
cargo test --workspace                        # all suites that need no delivery from outside
cargo test -p netex-rs --features crs
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace --all-features
cargo +nightly fuzz run read -- -max_total_time=60
```

The schema tests need `xmllint` (Debian: `libxml2-utils`) on `PATH`; the
official schemas and examples are in the repository (`tests/fixtures/netex-1.3`,
`netex-2.0`) but not in the crate's package, and where they are missing those
tests are skipped with a note. Two tests read a whole German delivery from
outside the repository, in release builds only:

```sh
NETEX_DELFI_ZIP=/path/to/netex.zip cargo test --release --test corpus
NETEX_DELFI_ZIP=/path/to/netex.zip NETEX_PY_EXTRACT_DIR=/path/to/extract cargo test --release --test extractor_parity
```

The benchmark of the table above is `tools/bench/` (Python, standard library
only; its tests: `NETEX_BIN=target/debug/netex python3 -m unittest discover -s tools/bench`).
`tests/publish_hygiene.rs` keeps the repository free of anything that belongs
to the place it was written rather than to the crate
(`NETEX_COMMITS=HEAD cargo test --test publish_hygiene` searches the commit
messages too).

| Path | What |
|---|---|
| `src/xml/`, `src/model/`, `src/version/` | the tokenizer cursor and emitter, the typed model, release detection and rewrite |
| `src/dataset/`, `src/stream.rs`, `src/index/`, `src/calendar.rs` | deliveries read in parallel and their manifests, the lean mode, the index, the calendar rules |
| `src/merge.rs`, `src/written/`, `src/findings.rs`, `src/check.rs`, `src/geometry.rs`, `src/crs/` | the merge of deliveries, documents as written, findings, duplicate identifiers, axis order, national reference systems |
| `crates/netex2gtfs/`, `crates/netex-cli/` | NeTEx → GTFS; the `netex` command |
| `examples/`, `benches/`, `fuzz/` | a delivery on one day, the profile of a dataset, the indexing measurement; criterion benchmarks; fuzz targets |
| `tests/`, `tools/` | the suites and their fixtures (`tests/fixtures/README.md`); the schema walker, the package tests, the benchmark |

## Acknowledgements and sources

**The standard.** The schemas and official example documents of both
releases are copied unaltered from
[TransmodelEcosystem/NeTEx](https://github.com/TransmodelEcosystem/NeTEx),
the XML Schema repository of CEN/TS 16614: 1.x from branch `v1.3` at
`4f42794` (22 September 2025; publication schema version 1.1) into
`tests/fixtures/netex-1.3/`, 2.0 from branch `v2.0` at `8056d5a`
(18 September 2026) into `tests/fixtures/netex-2.0/`. They are published under
the GPL-3.0, whose `LICENSE` is copied beside them; the schema files carry the
notice © CEN, Crown Copyright (eleven of them: CEN, VDV, RTIG). They are test
data of this repository, not part of the crate's package. What is derived
from them are names and facts of structure only — which element, attribute,
child and enumeration value belongs to which release (`docs/VERSIONS.md`,
`src/version/table.rs`, generated by `tools/schema_table.py`) — and the names
of the elements and attributes the typed model reads and writes; no text of
the schemas and no passage of their documentation. The normative text of
CEN/TS 16614 is published by CEN and is not open access; where the documents
here cite the standard, they cite the schemas, by file and element.

**Test data.** `tests/fixtures/delfi/` holds five line files of the German
national delivery, unaltered: © DELFI e.V., CC BY 4.0. `tests/fixtures/own/`,
`tests/fixtures/fragments/` and the deliveries under
`crates/netex2gtfs/tests/fixtures/` and `crates/netex-cli/tests/fixtures/` are
written for the tests, with invented places, lines and times, and licensed as
the crate is. The corrections of the feature `crs` (`src/crs/corrections.rs`)
and the reference points it is checked against (`tests/fixtures/crs/`) are
generated by `tools/crs_corrections.py` with PROJ and the official grid
transformations — the BEV's AT_GIS_GRID (© BEV, CC BY 4.0), swisstopo's
CHENyx06 (© swisstopo, free use) and, for the Dutch points, NSGI's
RDNAPTRANS2018 — at the stops of the Swiss service point register
(opentransportdata.swiss) and of the Austrian stop register (data.gv.at,
CC BY 4.0). `tests/fixtures/README.md` records file by file what was copied,
from where, and what was left out. The borders netex2gtfs takes a delivery's
country from (`crates/netex2gtfs/src/countries.txt`) are Natural Earth's
Admin 0 countries at 1:10m, public domain, simplified by
`tools/countries.py`.

**Deliveries.** The profiles were read, and the figures under Performance
measured, on the deliveries as their publishers publish them. None of them
is in the repository beyond the five DELFI files above; `docs/` and
`tools/bench/deliveries.json` name each delivery, its stand and where it was
fetched.

| Country | Publisher | Licence |
|---|---|---|
| Germany | DELFI e.V., [opendata-oepnv.de](https://www.opendata-oepnv.de/) | CC BY 4.0 |
| Austria | the nine Verkehrsverbünde, [data.mobilitaetsverbuende.at](https://data.mobilitaetsverbuende.at/) | Datenlizenz Mobilitätsverbünde Österreich |
| Switzerland | [opentransportdata.swiss](https://opentransportdata.swiss/) | its terms of use |
| Italy, South Tyrol | STA, at the Italian national access point | none stated |
| France | at [transport.data.gouv.fr](https://transport.data.gouv.fr/): Région Grand Est (Fluo, Bas-Rhin), Nantes Métropole (Naolib), Région Normandie; Bordeaux Métropole (TBM); SNCF | Licence Ouverte 2.0; TBM 1.0; SNCF ODbL |
| Norway | Entur | NLOD 2.0 |
| Sweden | Samtrafiken, through [Trafiklab](https://www.trafiklab.se/) | CC0 1.0 |
| Finland | Fintraffic | CC BY 4.0 |
| Denmark | Rejseplanen, through the Danish national access point | CC BY 4.0 |
| Belgium | SNCB, De Lijn, TEC, STIB (Belgian Mobility Company) | CC BY 4.0 |
| Luxembourg | the national access point, [data.public.lu](https://data.public.lu/) | CC0 |
| Czechia | CIS JŘ | none stated: measured only |
| Netherlands | RET, GVB, Arriva, Connexxion, Qbuzz, HTM and the national stop register, at [NDOV Loket](https://data.ndovloket.nl/) | CC0 |

The European Passenger Information Profile (EPIP, CEN/TS 16614-4) was read
through its official example in the NeTEx repository.

## Licence

Licensed under either of the [Apache License, Version 2.0](LICENSE-APACHE) or the
[MIT licence](LICENSE-MIT), at your option. Unless you explicitly state
otherwise, any contribution intentionally submitted for inclusion in the work
by you, as defined in the Apache-2.0 licence, shall be dual licensed as above,
without any additional terms or conditions.

The test data under `tests/fixtures/` is not all the crate's own:
`tests/fixtures/delfi/` is © DELFI e.V. under CC BY 4.0, and the official
NeTEx schemas and examples (GPL-3.0) are part of the repository only, not of
the crate's package. `tests/fixtures/README.md` has the details.
