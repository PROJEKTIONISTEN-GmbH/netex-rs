# netex-cli

[![crates.io](https://img.shields.io/crates/v/netex-cli.svg)](https://crates.io/crates/netex-cli)

The `netex` command: a checking report for NeTEx deliveries, their statistics,
their conversion to GTFS, regional cuts of them and the merge of several into
one — built on [netex-rs](../../README.md) and [netex2gtfs](../netex2gtfs/README.md).

- Licence: MIT OR Apache-2.0 · MSRV: Rust 1.86 · no `unsafe`
- Install: `cargo install netex-cli` (or from the repository: `cargo install --path crates/netex-cli`)

A delivery is a zip archive or a directory of NeTEx documents, read on every
core (`--threads N` to use fewer).

## `netex validate` — the checking report

```sh
netex validate netex.zip                              # the report for a person
netex validate netex.zip --format json > report.json  # the report as JSON
netex validate netex.zip --refuse unresolved --at-most journey-without-travel-time=100
```

The report lists every class with findings — the concerns the index of
netex-rs finds (references to nothing, definitions repeated with other content,
points passed through, days a line version does not govern, link geometries
written longitude first, …) and the checks of the report's own:

| Class | What it finds |
|---|---|
| `journey-without-travel-time` | a journey whose calls are all at one time |
| `time-goes-back` | a journey whose passing times go back along its calls |
| `stop-point-without-place` | a stop point called at that no assignment places at a stop place or quay |
| `stop-point-without-position` | a stop point called at without a position of its own or its stop's |
| `unreferenced` | a definition nothing refers to: a stop place none of whose quays is used either, a stop point, line, pattern, day type, operating period or day, availability condition |

Each class has its count, the first findings (`--examples N`, five by default)
naming the document, the element and the entity, and the journeys it affects:
found backwards through the references of the delivery — from a stop
assignment naming a quay nothing defines to the stop point it places, the
patterns calling there and the journeys following them.

**Policy and exit status.** Nothing is refused unless the command line says so:
`--refuse CLASS` refuses any finding of a class, `--at-most CLASS=COUNT`
tolerates up to a count, `--strict` refuses every class. The exit status is
**0** when the policy accepts the delivery, **1** when it refuses it, **2** when
the delivery cannot be read or the command line is wrong.

**JSON.** `--format json` prints the report as JSON, `--report FILE` writes it
as well; its layout is [`schema/validate-report.schema.json`](schema/validate-report.schema.json)
(JSON Schema 2020-12), version 1 (`report_version`). In short:

```json
{
  "report_version": 1,
  "source": "netex.zip",
  "counts": { "documents": 2, "stops": 4, "stop_points": 3, "lines": 2, "journeys": 4,
              "passing_times": 12, "calendars": 2, "transfers": 0, "interchanges": 0,
              "objects": 20, "references": 22 },
  "refused": false,
  "classes": [
    { "class": "unresolved", "kind": "index", "says": "reference to nothing", "count": 2,
      "affected_journeys": 4, "limit": null, "refused": false,
      "examples": [
        { "document": "a-line-1.xml", "element": "PassengerStopAssignment",
          "entity": "m:PassengerStopAssignment:irgendwo", "about": "<QuayRef> m:Quay:fehlt",
          "affected_journeys": 4,
          "affected_examples": ["m:ServiceJourney:gut", "m:ServiceJourney:flach"] } ] } ]
}
```

## `netex stats` — what a delivery holds

```sh
netex stats netex.zip
netex stats netex.zip --from 2026-10-12 --to 2026-10-18 --format json
```

Stops called at, lines, journeys and journey days (each journey once for every
day it runs on), and the first and last day, for the whole delivery and by
operator — the line's operator, else its first additional operator, else its
authority, else the operator the journey names —, over the whole calendar or
the days from `--from` to `--to`.

## `netex convert --to gtfs` — a GTFS feed

```sh
netex convert netex.zip --to gtfs gtfs.zip --shapes \
  --publisher-name "Example" --publisher-url https://example.org --report conversion.json
```

Writes the feed (a zip archive if the path ends in `.zip`, else a directory) as
[netex2gtfs](../netex2gtfs/README.md) makes it, prints the rows written, every
decision taken and what the feed does not carry, and writes the conversion's
report as JSON with `--report`. `--stop-ids global` gives stops their `GlobalID`
(DHID) or, without one, their Swiss `SLOID`, numbered `#2`, `#3` where stops
share one; `--time-zone` names the agencies' time zone where no document of the
delivery does.

## `netex extract` — a part of a delivery

```sh
netex extract delfi.zip --bbox 9.35,52.2,10.15,52.65 --to region-hannover
netex extract delfi.zip --area region.geojson --leaving cut --to region
netex extract netex.zip --operator DE::Operator:123:: --line DE::Line:456:: \
  --period 2026-10-12..2026-10-18 --to part --report part.json
```

Writes the journeys meeting every criterion given — run by an operator (the
line's, as `stats` counts it), of a line, calling in an area (a box in WGS 84 or
the polygons of a GeoJSON file), running on a day of a period — with everything
they refer to, into a new or empty directory, each document under its name:
the bytes of the source, the objects not taken left out and containers left
empty with them, so the part is NeTEx of the source's release.

What is taken: what a kept object refers to, resolved as netex-rs resolves it —
in the object's own document first, else in the document whose name sorts
first —; the stop, day type and notice assignments of what is kept, where they
stand in its document; an interchange or connection where everything it joins
is kept. A journey calling outside the area as well is kept whole
(`--leaving whole`, the default) or cut to its calls from the first to the last
in the area (`--leaving cut`; one calling there only once is left out). The
calendar is kept whole.

| Finding | What it is |
|---|---|
| `journey-leaves-area` | a journey calling outside the area too, kept whole |
| `journey-cut` | a journey whose calls before its first and after its last in the area are left out |
| `journey-leaves-and-returns` | a journey cut, leaving the area and coming back: its calls outside in between kept |
| `journey-touches-area` | a journey calling in the area only once, left out when cutting |
| `journey-not-cut` | a journey whose pattern stands in another document, kept whole |
| `journey-referenced` | a journey not chosen that a kept object refers to, kept |
| `interchange-left-out` | an interchange with a journey not taken |
| `reference-to-nothing` | a reference of a kept object to an identifier no document defines, written as it was |

On the German delivery of 2026-10-12 (28 286 documents), Region Hannover as a
box takes 60 876 journeys from 317 documents in 3:40 and 12.5 GB; each reads as
the journey of the same identifier and document of the whole delivery, and the
part holds every journey calling in the box (`tests/extract.rs`, opt-in).

## `netex merge` — several deliveries into one

```sh
netex merge oebb=oebb.zip vor=vor.zip --to merged --report merge.json
```

Merges the deliveries in the order given — the home source of what they share
first — into a new or empty directory, by the merge of netex-rs (`netex_rs::merge`,
since netex-rs 0.7.0; the command reads its arguments and prints or writes its
report), each under its rank and label
(`1-oebb/…`, `2-vor/…`), so that netex-rs, answering a reference outside its
document from the document whose name sorts first, answers it as the
deliveries are ranked. A delivery named by its path alone is labelled by its
file name. Not the first delivery wins, but these rules:

- **Stops pair by identifier, else by global identifier** (`GlobalID`, `SLOID`):
  a stop place whose every quay pairs with a quay of an earlier delivery's stop
  place is that stop place, its references pointed there; a name or a position
  more than 50 m away that differs is a finding naming both; each stop place and
  quay is counted by how it paired. A stop place with a quay pairing with none
  is kept apart, renamed where its identifier is taken.
- **A place under an earlier delivery's identifier and global identifier** — a
  park and ride — is that place stated otherwise: a finding, the earlier kept.
- **An object stated alike is kept once**: written alike, white space aside,
  referring to objects alike and assigned alike — a day type with the same
  assignments.
- **An identifier taken with other content is renamed** into the delivery's
  codespace, `<label>:<identifier>`, with every reference finding it.
- **A journey stated twice is kept once**: one calling at the same stops at the
  same times as an earlier delivery's on days that one runs on too is left out,
  its references pointed there; one running on other days is kept and found.
- What only objects left out needed is left out too — not a place, which stands
  for itself —, and so are the assignments of what is left out.

| Finding | What it is |
|---|---|
| `stop-paired-by-identifier` | a stop place or quay with an earlier delivery's identifier |
| `stop-paired-by-global-id` | a stop place or quay with an earlier delivery's global identifier, under another identifier |
| `stop-named-otherwise` | a paired stop whose name differs; the earlier name kept |
| `stop-placed-otherwise` | a paired stop more than 50 m from the earlier one; the earlier position kept |
| `stop-kept-apart` | a stop place with a quay pairing with none |
| `place-stated-otherwise` | another place under an earlier delivery's identifier and global identifier, stated otherwise; the earlier kept |
| `object-alike` | an object stated alike by an earlier delivery, left out |
| `identifier-renamed` | an identifier an earlier delivery answers otherwise, renamed |
| `journey-in-both` | a journey stated by an earlier delivery on these days too, left out |
| `journey-in-both-other-days` | a journey stated by an earlier delivery on other days, kept |
| `object-no-longer-needed` | what only the left-out needed, left out |

All documents of the deliveries are held in memory while they are merged.

## Licence

Licensed under either of the [Apache License, Version 2.0](LICENSE-APACHE) or the
[MIT licence](LICENSE-MIT), at your option.
