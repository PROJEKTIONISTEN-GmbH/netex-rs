# netex-rs

[![crates.io](https://img.shields.io/crates/v/netex-rs.svg)](https://crates.io/crates/netex-rs)
[![docs.rs](https://docs.rs/netex-rs/badge.svg)](https://docs.rs/netex-rs)

Read, check, index and write **NeTEx** — the European standard for exchanging
public transport timetables and networks, CEN/TS 16614 — in Rust, from a single
document up to a whole national delivery.

## Why you might need it

Official timetable data in Europe is published as NeTEx: across the EU, the
national access points are to provide timetables in it (Delegated Regulation
(EU) 2017/1926). Germany's complete national timetable comes from DELFI as
NeTEx, and so do the timetables of Austria, Switzerland, South Tyrol and
France.
Building on that data means getting past three things:

- **Size.** A national delivery is tens of thousands of XML files — Germany's
  is 28 176 files, 76 GB of XML.
- **Interpretation.** References cross files, one stop place is defined in
  hundreds of them, the days a journey runs follow rules spread over several
  elements, and every country uses a profile of its own.
- **Two releases.** NeTEx 1.x and 2.0 share one namespace, and the version a
  document declares cannot be relied on.

netex-rs handles that, so that you can:

- **build a journey planner, a departure board or a stop finder** on official
  data — stops with their quays and levels, lines, journeys with their calls
  and passing times, the days each journey runs, walks between stops and
  planned interchanges; Germany's whole delivery is indexed in about a minute
  and a half on eight threads;
- **turn a delivery into GTFS** for software that reads only GTFS, with
  [netex2gtfs](crates/netex2gtfs/README.md) or `netex convert`;
- **check a delivery** before you publish or import it: every reference to
  nothing, every contradiction and every decision the reader takes is reported
  as a finding that names the file and the object (`netex validate`);
- **cut out and combine** — a region or an operator out of a national
  delivery, several deliveries into one (`netex extract`, `netex merge`);
- **write NeTEx** for release 1.x or 2.0, or rewrite a document for the other
  release with a report of everything it cannot hold;
- **follow the regular updates**: a manifest says which files of a delivery are
  new, changed or gone since the last one.

## Getting started

As a library:

```sh
cargo add netex-rs
```

As a command, without writing any Rust:

```sh
cargo install netex-cli
netex validate netex.zip                     # the checking report: what is missing or contradicts
netex stats netex.zip                        # what the delivery holds
netex convert netex.zip --to gtfs gtfs.zip   # the delivery as a GTFS feed
netex extract netex.zip --bbox 9.35,52.2,10.15,52.65 --to region
```

- Licence: MIT OR Apache-2.0 · MSRV: Rust 1.86 · no `unsafe`
- In this repository: **netex-rs**, the library;
  [netex2gtfs](crates/netex2gtfs/README.md), a delivery as a GTFS feed;
  [netex-cli](crates/netex-cli/README.md), the `netex` command.

## Example

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
same with times and headways; the directories under `tests/fixtures/own/` are
small deliveries of the profiles below.

## What it does

- **Reads and writes**, streaming on the `quick-xml` tokenizer, a typed model of
  the framework (`PublicationDelivery`, frames, versions, validity, references,
  key lists, codespaces, frame defaults), of Part 1 (stop places, quays, access
  spaces, entrances, levels, accessibility, site connections, topographic
  places, lines, routes, scheduled stop points, stop assignments, journey
  patterns, destination displays, notices, operators, authorities, vehicle
  types) and of Part 2 (service calendars, day types, operating periods and
  days, day type assignments, availability conditions, service journeys,
  dated and template journeys with their headway groups, passing times,
  calls, train numbers, interchanges and interchange rules).
- **Loses nothing.** Every element and attribute the model does not name —
  fares (Part 3), new modes (Part 5), every rarer child of a typed element — is
  carried through as it was written, at the position it was read, and written
  back there.
- **Both releases in one model.** The releases share one namespace, and the
  `version` attribute of a publication cannot be trusted (most official 2.0
  examples declare `1.0`), so the crate tells them apart by the elements,
  attributes and enumeration tokens one release declares and the other does
  not — a table derived from both schema sets (`docs/VERSIONS.md`). Writing a
  document for the other release leaves those out and reports each one.
- **A delivery, not just a document.** `dataset::Source` reads a zip archive
  or a directory on every core, largest file first. `index::Index` resolves
  the references across files and holds stops by identifier and `GlobalID`
  with their quays and levels, lines, journeys — in the line they name, or
  their route or pattern names — with their calls, passing times, boarding and
  alighting and the days they run, walks between stops and planned
  interchanges — the same index whatever order and however many threads read
  the files.
- **Each document's own, the delivery's shared.** Stop places, quays, stop
  points and levels are gathered once, the repetitions across files merged;
  lines, patterns, journeys and calendars are each document's own, and a
  reference resolves in its own document first, then in the delivery — so that
  a delivery reusing its calendars' identifiers per document reads right.
- **The days, by the rules of the standard.** Day type assignments in their
  order, operating periods and their day bits, availability conditions, and
  the days the version of its line a journey names governs (`calendar`). A
  point the vehicle passes through is no call; a template journey stands for
  the runs of its headway groups.
- **Findings, not silence.** A reference nothing defines, a definition repeated
  with other content, a point passed through, days a line version does not
  govern, a link geometry written longitude first — each decision is an
  `index::Finding` naming the document and the entity, counted always and
  listed in a fixed order; an `index::Policy` refuses an index whose counts
  exceed what the caller tolerates.
- **Lean when that is enough.** `stream::read` types only the kinds of entity a
  `stream::Selection` names and skips the rest at the tokenizer;
  `stream::read_with` hands the rest to the caller in the same walk.
  `dataset::Manifest` fingerprints a delivery from the archive's central
  directory and says which files are new, changed or gone since the last one.
- **Robust against foreign input.** No `unsafe`. Entities are never expanded
  and DTDs never fetched; nesting depth, text length, attribute count and
  fragment size are bounded (`Limits`); errors name the line and the element
  path. Two fuzz targets (`fuzz/`) run the reader and the release rewrite.

## Reading one document

```rust,no_run
use netex_rs::model::{Frame, StopPlaceItem};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let xml = std::fs::read_to_string("line.xml")?;
    let document = netex_rs::read(&xml)?;
    for frame in document.delivery.frames() {
        if let Frame::Site(site) = frame {
            for item in site.stop_places.iter().flat_map(|s| &s.items) {
                if let StopPlaceItem::StopPlace(place) = item {
                    let global_id = place.key_list.as_ref().and_then(|k| k.get("GlobalID"));
                    println!("{:?} {:?}", place.name.as_ref().map(|n| n.as_str()), global_id);
                }
            }
        }
    }

    // Back out, as it was — or for the other release, with the losses reported.
    std::fs::write("same.xml", document.write())?;
    let for_20 = document.write_for(netex_rs::Release::V2);
    for loss in for_20.report.losses() {
        eprintln!("{loss}");
    }
    std::fs::write("line-2.0.xml", for_20.xml)?;
    Ok(())
}
```

## Reading lean, and only what changed

An importer does not need a model of every document. `stream::read` hands each
entity of a selected `stream::Kind` to a closure as soon as it is read, typed by
the same readers the full model uses, with the frames it stands in:

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

`Source::manifest` fingerprints every entry of a delivery — name, inflated size,
CRC-32 — from the archive's central directory without inflating anything, and
`Manifest::diff` (or `diff_by`, for publishers that put a date into every file
name) says what changed since a previous manifest; `Index::build_from` indexes
only those entries. What it saves depends on the publisher: the German delivery
is regenerated each week down to renumbered identifiers, so there no file is
ever unchanged.

## Profiles

NeTEx is used through national and European profiles. The crate reads the
documents of any of them into the same model; the index applies the rules each
profile relies on.

| Profile | How its deliveries are laid out | What the crate reads for it | Read in full |
|---|---|---|---|
| **DELFI** (Germany, `ntx:1.1`) | one document per line, 28 176 in one zip archive; each repeats the stops it serves | stop places merged across documents with all their quays, `GlobalID` (DHID) keys, site connections, interchanges | yes, every file of the national delivery |
| **Austria** | one document per line, each with its own calendar; lines in versions | identifiers that are each document's own, the days each version of a line governs, `OperatingPeriodRef`s naming UIC periods, flexible lines left out and counted | yes, all nine deliveries of the national access point |
| **Switzerland** | shared documents (stop points, lines, calendar) and timetable documents naming them | journeys stating their `Call`s, `StopUse` pass-throughs, availability conditions as the calendar, template journeys with headway groups | yes, a part of the national delivery (five shared and 20 timetable documents) |
| **EPIP** (European Passenger Information Profile) | composite frames of typed frames (`epip:EU_PI_*`) | read as any NeTEx; the index as for the others | the official EPIP example and the crate's own |
| **IT-ITH1** (Italy, South Tyrol; the Italian national access point) | one document of EPIP frames (`IT:ITH1:…` identifiers), lines in versions | every name with the language it states; patterns naming their line only through a route; stop places stating a submode beside their mode; train numbers in the timetable frame; `OperatingPeriodRef`s naming UIC periods | yes, levels 1 and 3 of the delivery |
| **NeTEx France** (the French national access point) | per network: stops, calendar, network and one document per line, everything in general frames typed by the part (`FR:TypeOfFrame:NETEX_ARRET`, …); or specific frames typed so; SNCF's trains in one document of 647 MB | general frames; quays defined beside their stop place, which names them by `QuayRef`; passing times naming no point, by position; day types given their days by a validity and days of the week, the assignments their exceptions; plain periods by the days of the week of their day type | yes, five deliveries: Fluo Grand Est (Bas-Rhin), Naolib, TBM, Normandy's aggregate, SNCF |

`tests/fixtures/own/` has a small delivery of each kind but DELFI, written for
the tests; `tests/fixtures/fragments/` has small sets of frames and entities,
each pinning what the reader makes of one kind of thing a delivery states
(times, white space, references, calendars, levels, axis order …);
`tests/fixtures/delfi/` has five files of the German delivery.
`docs/DELFI-PROFILE.md` lists what the German delivery uses of NeTEx: 193
distinct elements, of which the typed model covers all but four, which are
carried through. `docs/IT-ITH1-PROFILE.md` does the same for South Tyrol's,
with what it states and how the crate reads it, and `docs/FR-PROFILE.md` for
five French deliveries, checked against the GTFS feeds their producers publish.

## How conformant

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
| written for the same release: lossless, valid | 238, 238 | 269, 269 |
| written for the other release: lossless / with reported losses | 237 / 1 | 157 / 112 |
| … and valid there | 120 of 238 | 148 of 269 |
| release detection contradicting the validator | 0 | 0 |
| `should-fail` rejected by the validator and found by `check::duplicate_ids` | — | 3 / 3 |

A 2.0 document written for 1.x loses what 1.x does not have — `privateCodes`,
`FarePriceFrame`, `deckPlans`, `PropulsionTypes` — and every loss is in the
report. A lossless document can still fail the other release's validator,
because the releases also differ in what they *require* (2.0 reshaped its
identity constraints and tightened enumerations); the crate does not invent
identifiers or tokens to satisfy that.

## Speed

Measured with `examples/index_delivery.rs --lean` (the routing selection), a
release build on 8 threads of a shared 8-CPU, 16 GiB machine; wall clock and
peak resident memory of the process, the mean of two runs, version 0.3.0:

| Delivery | XML | time | peak memory |
|---|---:|---:|---:|
| Germany, the whole national delivery (28 176 files) | 75.8 GB | 95.6 s | 11.7 GB |
| Germany, the 1 569 files of Deutsche Bahn | 3.49 GB | 4.8 s | 0.68 GB |
| Austria, Verkehrsverbund Ost-Region | 5.29 GB | 6.9 s | 1.4 GB |
| Austria, ÖBB | 1.04 GB | 1.3 s | 0.23 GB |
| Switzerland, five shared and 20 timetable documents | 1.41 GB | 2.3 s | 1.0 GB |

Reading a single 1.1 MB line file on one thread runs at about 150 MB/s, full
model or lean.
`docs/PERFORMANCE.md` has the curves over the threads, the phases, and how they
were measured.

## Limits

- **Reading, not converting.** The crate writes NeTEx back in either release, but
  it does not turn one profile into another (no EPIP writer); GTFS is
  netex2gtfs's, beside it.
- **Reference systems behind a feature.** Positions in a national system —
  LV95/LV03, MGI, RD, Lambert-93, UTM — are turned into WGS 84 only with the
  feature `crs` (`netex_rs::crs`); without it the index reads latitude and
  longitude as before.
- **Typed where routing needs it.** Fares (Part 3), vehicle schedules and many
  rarer elements are carried through untyped: kept and written back, but not
  modelled. Flexible (on-demand) lines are not indexed; their journeys are left
  out and counted.
- **No schema validation at run time.** The crate reads what is well-formed
  within its limits; validity against the schemas is checked by its tests, with
  `xmllint`, not by the library.
- **The index is in memory.** A whole national delivery needs about as much
  memory as the table above says; the lean stream without an index needs well
  under a gigabyte.
- **Zip archives with deflate** (or stored) entries, and directories; XML 1.0
  in UTF-8.

## What a reader decides

NeTEx leaves open how a reader turns what a delivery states into a timetable — a
reference to nothing, a stop place defined in three hundred documents, a line
in versions, a point a train passes through. `docs/INTERPRETATION.md` lists
these questions as the German, Austrian, Swiss, South Tyrolean and French
deliveries pose them, with what the standard says and what this crate does, and why;
`docs/PROFILE-UEBERFUEHRUNG.md` says what is gained and lost turning one
profile into another.

## Layout

| Path | What |
|---|---|
| `src/xml/` | the cursor over the tokens, the emitter, carried-through fragments |
| `src/model/` | the typed model and its macros |
| `src/version/` | release detection, the release difference table, the rewrite with its loss report |
| `src/check.rs` | duplicate identifiers |
| `src/dataset/` | zip archives and directories, read in parallel; manifests |
| `src/stream.rs` | the lean mode: entities by selection, and what the walk skips |
| `src/index/` | the index: gathering, merging, where a reference resolves, resolving, findings and policy |
| `src/calendar.rs` | the calendar rules: period days, the order of assignments, what a line version governs |
| `src/geometry.rs` | the axis order of a link's geometry |
| `src/crs/` | the feature `crs`: national reference systems into WGS 84, their corrections generated by `tools/crs_corrections.py` |
| `crates/netex2gtfs/` | a delivery as a GTFS feed, with its report and the comparison against a national GTFS of the same stand |
| `src/merge.rs` | several deliveries written as one: stops paired by identifier or global identifier, objects stated alike kept once, identifiers taken renamed, journeys stated twice kept once |
| `src/written/` | a document as written: where each element and attribute value stands (`scan`), the objects of its frames (`outline`), changes applied as it is written out (`edit`) |
| `src/findings.rs` | what a rewrite of deliveries reports, by class |
| `crates/netex-cli/` | the `netex` command: `validate`, `stats`, `convert`, `extract`, `merge` (the library's) |
| `examples/` | `timetable` (a delivery on one day), `inventory` (the profile of a dataset), `index_delivery` (the measurement above) |
| `benches/` | criterion benchmarks: reading, writing, indexing |
| `fuzz/` | cargo-fuzz targets `read`, `rewrite`, `convert` (netex2gtfs) and `outline` (`netex_rs::written`) |
| `tests/` | the conformance harness and the other suites; `tests/fixtures/README.md` says what the test data is and under which licence |
| `tools/schema_table.py` | the schema walker behind `src/version/table.rs` and `docs/VERSIONS.md` |

## Development

```sh
cargo test --workspace                        # all suites that need no delivery from outside
cargo test -p netex-rs --features crs
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all --check
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --workspace --all-features
cargo +nightly fuzz run read -- -max_total_time=60
```

The schema tests need `xmllint` (libxml2; Debian: `libxml2-utils`) on `PATH` and
the official schemas and examples under `tests/fixtures/netex-1.3` and
`netex-2.0`, which are in the repository but not in the crate's package; where
the schemas are not there, those tests are skipped with a note. Compiling a
NeTEx schema costs `xmllint` about twenty seconds, so the suite takes a few
minutes. Two tests read a whole German delivery from outside the repository
and run only in release builds and when told where it is:

```sh
NETEX_DELFI_ZIP=/path/to/netex.zip cargo test --release --test corpus
NETEX_DELFI_ZIP=/path/to/netex.zip NETEX_PY_EXTRACT_DIR=/path/to/extract cargo test --release --test extractor_parity
```

`tests/publish_hygiene.rs` keeps the repository free of anything that belongs
to the place it was written rather than to the crate; given a revision range,
it searches the commit messages too:

```sh
NETEX_COMMITS=HEAD cargo test --test publish_hygiene
```

## Acknowledgements

### The standard, and what the crate takes from it

- **[TransmodelEcosystem/NeTEx](https://github.com/TransmodelEcosystem/NeTEx)** —
  the XML Schema repository of NeTEx, CEN/TS 16614. The schemas and the official
  example documents of both releases are copied from it unaltered: 1.x from
  branch `v1.3` at `4f42794` (22 September 2025; publication schema version
  1.1) into `tests/fixtures/netex-1.3/`, 2.0 from branch `v2.0` at `8056d5a`
  (18 September 2026) into `tests/fixtures/netex-2.0/`. They are published
  under the GPL-3.0, the repository's `LICENSE`, copied beside them; the schema
  files carry the notice © CEN, Crown Copyright (eleven of them: CEN, VDV,
  RTIG). They are test data of this repository, not part of the crate's
  package.
- **What is derived from them** are names and facts of structure only — which
  element, attribute, child and enumeration value belongs to which release,
  listed in `docs/VERSIONS.md` and `src/version/table.rs` and generated from
  the schemas by `tools/schema_table.py` — and the names of the elements and
  attributes the typed model reads and writes; no text of the schemas and no
  passage of their documentation.
- The normative text of CEN/TS 16614 is published by CEN and is not open
  access. Where the documents here cite the standard, they cite the schemas,
  by file and element.

### Test data

- The official examples under `tests/fixtures/netex-1.3/` and
  `tests/fixtures/netex-2.0/` are copied unaltered and stand under their own
  licence, as above.
- `tests/fixtures/delfi/` holds five line files of the German national
  delivery, unaltered: © DELFI e.V., CC BY 4.0.
- `tests/fixtures/own/`, `tests/fixtures/fragments/` and the deliveries under
  `crates/netex2gtfs/tests/fixtures/` and `crates/netex-cli/tests/fixtures/` are
  written for these tests, with invented places, lines and times, and licensed
  as the crate is.
- The corrections of the feature `crs` (`src/crs/corrections.rs`) and the
  reference points it is checked against (`tests/fixtures/crs/`) are generated
  by `tools/crs_corrections.py` with PROJ and the official grid
  transformations — the BEV's AT_GIS_GRID (© BEV, CC BY 4.0), swisstopo's
  CHENyx06 (© swisstopo, free use) and, for the Dutch points, NSGI's
  RDNAPTRANS2018 — at the stops of the Swiss service point register
  (opentransportdata.swiss) and of the Austrian stop register (data.gv.at,
  CC BY 4.0).

`tests/fixtures/README.md` records file by file what was copied, from where,
and what was left out.

### Profiles

The profiles in the table above are read as their publishers deliver them: DELFI
e.V.'s national NeTEx delivery for Germany (`NX-PI-01_DE_NAP_LINE`,
[opendata-oepnv.de](https://www.opendata-oepnv.de/)); the deliveries of the
nine Austrian Verkehrsverbünde at the Austrian national access point; the
Swiss national NeTEx timetable; the European Passenger Information Profile
(EPIP, CEN/TS 16614-4) through its official example in the NeTEx repository;
South Tyrol's timetable in the Italian profile IT-ITH1 at the Italian
national access point; and in the French profile, at the French national access
point ([transport.data.gouv.fr](https://transport.data.gouv.fr/)), the
timetables of Fluo Grand Est in the Bas-Rhin (Région Grand Est), Naolib
(Nantes Métropole) and Normandy's networks (Région Normandie) under the Licence
Ouverte 2.0, TBM's (Bordeaux Métropole) under the Licence Ouverte 1.0, and
SNCF's under the ODbL. `docs/INTERPRETATION.md` and the profile pages name the
deliveries each figure was counted on. None of these deliveries is in the
repository beyond the five DELFI files above.

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
