# netex2gtfs

[![crates.io](https://img.shields.io/crates/v/netex2gtfs.svg)](https://crates.io/crates/netex2gtfs)
[![docs.rs](https://docs.rs/netex2gtfs/badge.svg)](https://docs.rs/netex2gtfs)

NeTEx to GTFS in Rust: a NeTEx delivery — a zip archive or a directory of
documents, read and resolved by [netex-rs](../../README.md) — written as a GTFS
feed, with a report of every decision the conversion took and of everything in
the delivery the feed does not carry.

- Licence: MIT OR Apache-2.0 · MSRV: Rust 1.86 · no `unsafe`
- The command line around it: `netex convert --to gtfs` ([netex-cli](../netex-cli/README.md)).

```rust,no_run
use netex_rs::dataset::Source;
use netex2gtfs::{convert, output, Delivery, Options, Reading};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = Source::open("netex.zip")?;
    let delivery = Delivery::read(&source, Reading { geometry: true, ..Reading::default() })?;
    let mut options = Options::default();
    options.shapes = true;
    let report = convert(&delivery, &options, output::at("gtfs.zip")?)?;
    println!("{}", report.rows["trips.txt"]);
    Ok(())
}
```

## Whole national deliveries

Every national delivery netex-rs reads converts whole, without error — the
Czech ones into empty feeds, as none of their stops states a position. On 16
threads of an AMD EPYC 7502P with 64 GiB, with `netex convert`:

| Delivery | XML | NeTEx → GTFS | Peak memory | Trips | Stop times |
|---|---:|---:|---:|---:|---:|
| Germany | 74.7 GB | 5.3 min | 12.8 GiB | 4 838 062 | 99 495 018 |
| Switzerland | 39.3 GB | 2.7 min | 16.9 GiB | 2 583 297 | 41 602 402 |
| Norway | 5.7 GB | 29 s | 1.9 GiB | 374 843 | 8 986 827 |
| Denmark | 3.9 GB | 15 s | 1.1 GiB | 196 818 | 4 690 714 |

These figures are of netex2gtfs 0.2, which wrote the feed on one thread,
most of it deflating the zip archive; 0.3 deflates each file on every
thread, and on 8 threads converts Germany in 200 s instead of 364 s.
Converting needs no more memory than reading the delivery. Each feed was
held against the GTFS feed its
publisher writes from the same data: Norway's has the same trips on the same
days. All deliveries, the method and every comparison: the
[README of netex-rs](../../README.md#performance) and its
`docs/PERFORMANCE.md`.

## How a feed is made

The delivery is read once, on every core. The index of netex-rs gives stops,
lines, journeys with their calls and the days they run, walks and interchanges —
each reference resolved in its own document first, then in the delivery, where
the document whose name sorts first answers; what a feed needs beyond it is read
in the same walk of each document and resolved by the same rule.

| File | From | Rules |
|---|---|---|
| `agency.txt` | the operator of each line, else its first additional operator, else its authority (as a journey planner's importer orders them); the operator a journey names where its line names none; the feed's publisher for a line naming none | name, else its identifier; `ContactDetails/Url` (an empty `<Url/>` is none, as is one without its scheme, `www.…`), else the feed's publisher's; `Phone`; the feed's one time zone — GTFS has one for all agencies —: the first a document names by a name the time zone database holds (`Europe/Zurich`), in the order of the documents' names, else the one given, else that of the country most of the journeys' identifiers show (`DE::` Europe/Berlin, `at:` Europe/Vienna, `ch:` Europe/Zurich, whatever profile the frames are typed as), else that of the country most of the stops with a position lie in (below) |
| `stops.txt` | every stop place, quay and stop point a trip calls at | a quay under its stop place as a station (`location_type` 1) — under the parent site that stop place is within (`ParentSiteRef`), where it is within one with a position, as GTFS stations do not nest —; a stop place called at itself a stop; a stop without a name named as its stop place, else by its identifier; a stop point no stop is assigned a stop at its own position; `wheelchair_boarding` from `MobilityImpairedAccess` (or `AllAreasWheelchairAccessible`), `platform_code` from a quay's `PublicCode`, `level_id` from its `LevelRef`; `stop_timezone` of a station, or of a stop under none, the trips of an agency in another zone than the feed's call at, where all trips calling there are in that zone (the column only where one is); the stations first, then the stops, each in the order of their identifiers |
| `levels.txt` | the levels of quays in the feed | `level_index` from the level's `PublicCode` where it is a finite number, else `RelativeLevelOrder` |
| `routes.txt` | the lines of the trips | one route per NeTEx identifier — a line's versions are one route —, numbered `<id>#<n>` where definitions of one identifier differ or another route took it; `route_short_name` its `PublicCode` (else `ShortName`), `route_long_name` its name, else, without a short name, its identifier; `route_type` from its mode (a trolleybus 11), colours from `Presentation` |
| `trips.txt` | the journeys of the index | `service_id` the journey's calendar — for a journey of an agency in another time zone written on other days, days of its own, `c<n>-<k>` (below) —; `trip_headsign` the destination display at its first call; `trip_short_name` its train number (`ForAdvertisement`), else its `PublicCode`; `direction_id` from its, else its pattern's `DirectionType` or `DirectionRef`; `wheelchair_accessible` from its accessibility assessment |
| `stop_times.txt` | the calls the index makes of each journey at a stop the feed places | a point passed through is no call; from the first call with a time to the last, as GTFS requires times there; boarding and alighting as the point or call says; times past midnight past 24 hours; `stop_headsign` where the display shown from a point on differs from the trip's, a call the points do not name showing the one before |
| `frequencies.txt` | the headway groups of template journeys | from the first departure to a second after the last — GTFS runs a frequency up to, not including, its end —, each as much later as the trip's first call is after the journey's, where the feed leaves out the journey's first; `exact_times` 0: a headway group states an interval, not departures |
| `calendar.txt`, `calendar_dates.txt` | the calendars of the trips | the weekdays a calendar runs on on at least half their dates from its first day to its last, and each date it differs from them on |
| `transfers.txt` | walks between stops (`SiteConnection`), planned interchanges | a walk `transfer_type` 2 with its duration, the longest of those stated between two stops; an interchange staying seated 4, guaranteed 1, else 0, once between the same trips at the same stops — as the document whose name sorts first states it; each in the order of the identifiers of the stops and stop points they start at |
| `shapes.txt` | the geometry of the link sequence projection a trip's journey names, else of the service links it runs over (with `Options::shapes`) | the projection a journey names (`LinkSequenceProjectionRef`), standing among a frame's members as Bordeaux's deliveries state it, where it states a geometry, read in the axis order whose first position lies near the journey's first stop; else, from each call on, the links its pattern names in sequence (`linksInSequence`) to a later call's stop point — one link to the next call each, as Norway's and Sweden's deliveries name them, or one for the whole way, as Finland's —, else a link the delivery states between the two stop points; each resolved as the trip's document sees it, so that the links of a codespace's shared data serve its line files; a link's geometry its `gml:LineString`, else the one of its `LinkSequenceProjection`; read in its reference system — its own `srsName` (`EPSG:4326`, `urn:ogc:def:crs:EPSG::4326`, a bare `4326`, …), else its document's — and in the axis order whose first position lies near the link's start |
| `feed_info.txt`, `attributions.txt` | the publisher given, else the delivery's data source with a web address; the data sources and participants | the feed's period from its calendars, its version the latest publication timestamp in time — an offset stated taken away, one without as if in UTC |

GTFS requires positions in WGS 84; netex2gtfs reads netex-rs with its `crs`
feature, so a delivery writing its positions in a national reference system
converts as one in WGS 84 does. A position that is none on the earth — a
latitude beyond ±90°, a longitude beyond ±180°, Sweden's register states a
latitude of 56162332 — or is 0, 0, where a position left unset is written, is
taken as none: a quay then stands at its stop place, a shape's stretch is left
out.

GTFS requires one time zone for all agencies of a feed, named as in the time
zone database (`Europe/Zurich`), and many deliveries name none — Austria's
national deliveries, typed as the European profile's, and the Belgian
operators' among them. A document's `TimeZone` is free text in NeTEx; the
examples of the standard write abbreviations of an offset (`CET`, `CEST`,
`BST`), which are taken as naming none. Where neither a document nor the
caller names one, the journeys' identifiers may show the country; else the
country is the one more than half of the stops with a position lie in, by the
borders of Natural Earth's Admin 0 countries at 1:10m (public domain) for the
member states of the European Union and of EFTA, the United Kingdom and the
small states among them — Cyprus the part its Republic governs —, simplified
to 0.005° (`src/countries.txt`, written by the repository's
`tools/countries.py`). A delivery whose stops lie elsewhere, or in no country
of the table at all, takes no zone: `convert` fails with `Error::NoTimeZone`,
and a zone must be given — one the database holds, else `convert` fails with
`Error::UnknownTimeZone`. The report says which rule gave the zone. The time
zone database is the one bundled with jiff, so that a feed is the same on
every system.

An agency whose document names another zone than the feed's — Greenland's
buses in Denmark's delivery, America/Nuuk three hours behind Copenhagen — is
written in the feed's zone, and so are its trips' times: each trip runs at the
instants its times state in its agency's zone. A GTFS time counts from twelve
hours before noon of its day, so a trip's times are as much later as noon of
the day is later in its agency's zone than in the feed's; a trip that would
begin before its day runs on the day before. Where that difference changes
over a trip's days — the zones keeping summer time on other days, as New York
and Zürich do between the second and the last Sunday of March —, the trip is
split: a trip for each set of days of one difference, the second and each after
it `<id>#<n>`, each with days of its own; a planned interchange holds for each.
The stops its trips call at are in its zone: `stop_timezone` says so at their
station.

A feed written into a directory replaces a feed there: each file the GTFS
reference defines that the conversion did not write — a `shapes.txt` or
`feed_info.txt` of an earlier run — is removed when it finishes; files of
other names stay.

A feed is the same bytes on any number of threads reading and writing it: its
rows are written in an order the delivery fixes, not the order the threads
took its documents in, and its zip archive (ZIP64 where a file or the archive
exceeds 4 GiB) is written by netex2gtfs itself, each file deflated in pieces of
1 MiB on every thread of the current rayon pool, each piece's compressor
primed with the 32 KiB before it, the pieces one deflate stream.

## The report

`convert` returns a `Report`: the rows written by file, the feed's period, the
findings of the index by concern, the profile the delivery was read as (DELFI,
Austria, Switzerland, EPIP or other), every decision taken with its count and a
first example, and every element the feed does not carry (fare frames, notices,
flexible lines, interchange rules, …) with its count and the first document
holding one. Nothing is left out silently.

| Decision | What happens |
|---|---|
| `journey-without-line` | a journey naming no line, nor a pattern naming one: left out |
| `journey-without-days` | a journey running on no day: left out |
| `call-without-place` | a call at a stop point assigned to no stop and without a position: left out of its trip |
| `trip-without-two-calls` | a journey left with fewer than two calls at a place: left out |
| `trip-at-unplaced-stops` | a journey left with fewer than two calls once those at stops the feed cannot place are left out: left out |
| `trip-untimed-end-cut` | a journey whose first or last call at a placed stop has no time — the calls before or after it left out —: cut to the first and last with a time |
| `stop-without-position` | a stop without a position of its own, its station's or its quays': not in the feed |
| `quay-at-its-stop-place` | a quay without a position: placed at its stop place |
| `stop-place-at-its-quays` | a stop place without a position: placed at the mean of its quays' |
| `stop-place-called-at-beside-its-quays` | calls at a stop place that is also the station of its quays: a stop `<id>#place` under it |
| `stop-made-unique` | an identifier several stops of the feed would carry — a global identifier (GlobalID, SLOID, with global stop identifiers), or one another stop's number or `#place` took —: the second and each after it `<id>#<n>`, the next number no identifier has taken |
| `stop-point-as-stop` | a stop point assigned to no stop: a stop of its own |
| `stop-named-by-its-station` | a stop without a name: named as its stop place |
| `stop-named-by-its-identifier` | a stop without a name, nor a stop place with one: named by its identifier |
| `parent-site-without-position` | a parent site without a position of its own or of its quays: no station, the stop places within it stand on their own |
| `level-without-number` | a level with no number: the quay is written without it |
| `route-made-unique` | a line defined again otherwise, or one whose identifier another route took: a route `<id>#<n>` |
| `route-type-unknown` | a line of a mode GTFS has no type for: written as a bus |
| `route-named-by-its-identifier` | a line without a name or a number: its long name its identifier |
| `route-without-agency` | a line naming no organisation the delivery defines: the feed's publisher (the one given, else the delivery's data source with a web address) is its agency; without one, the route names none |
| `agency-without-url` | an organisation without a web address, or with an empty one or one without its scheme: the feed's publisher's |
| `agency-named-by-its-identifier` | an organisation without a name: named by its identifier |
| `agency-time-zone-unified` | an organisation whose document names another time zone than the feed's: written in the feed's; its trips at the instants their times state in its zone, its stops in its zone (`stop_timezone`) |
| `trip-times-in-feed-zone` | a journey of such an organisation whose times differ in the feed's zone: written in it, the instants kept — on the day before where it would begin before its day |
| `trip-split-by-offset` | a journey of such an organisation whose zone's offset from the feed's changes over its days: a trip for each set of days of one difference, `<id>#<n>` |
| `trip-days-beyond-zone-rules` | days of such a journey beyond the instants the time zone rules reach (9999-12-30): left out of its trip |
| `stop-time-zone-ambiguous` | a stop, or its station, called at by trips of agencies in different time zones: in the feed's |
| `url-without-scheme` | a web address without its scheme (`www.…`, `//…`): taken as none |
| `position-off-the-earth` | a stop place, quay or stop point at a latitude beyond ±90° or a longitude beyond ±180°: taken as without a position |
| `position-at-zero-zero` | a stop place, quay or stop point at 0, 0: taken as without a position |
| `trip-made-unique` | a journey identifier the index holds twice: `<id>#<n>` |
| `trip-time-goes-back` | a journey whose passing times go back along its calls: left out |
| `walk-stated-twice` | a walk between two stops stated again: one transfer, the longest |
| `interchange-left-out` | an interchange with a trip or stop the feed does not hold |
| `interchange-repeated` | an interchange between the same trips at the same stops stated again — in another document —: one transfer, the first |
| `link-without-geometry` | a service link stating no geometry, on itself or in a link sequence projection: no shape follows it |
| `shape-incomplete` | a trip with a pair of calls no link with a geometry joins: no shape |
| `shape-without-two-positions` | a trip whose course gives fewer than two positions — every stretch in a reference system netex-rs does not know, or one position —: no shape |
| `projection-without-geometry` | a link sequence projection stating no geometry: a journey naming it takes the shape of its service links |
| `shape-in-unknown-system` | a geometry of a link or projection in a reference system netex-rs does not know: its stretch left out |
| `shape-off-the-earth` | a geometry of a link or projection with a position off the earth or at 0, 0: its stretch left out |
| `time-zone-not-a-name` | a document naming its time zone otherwise than by a name the time zone database holds (`CET`, `CEST`, `BST`): taken as naming none |
| `time-zone-of-identifiers` | no document names a time zone and none is given: the zone of the country most of the journeys' identifiers show (`DE::` Europe/Berlin, `at:` Europe/Vienna, `ch:` Europe/Zurich), with how many show it |
| `time-zone-of-stops` | no document names a time zone, none is given, and the identifiers show no country: the zone of the country more than half of the stops with a position lie in (the European part of France's, for a French delivery), with how many lie there |
| `feed-info-without-publisher` | no publisher and no data source with a web address: no `feed_info.txt` |

## Licence

Licensed under either of the [Apache License, Version 2.0](LICENSE-APACHE) or the
[MIT licence](LICENSE-MIT), at your option.
