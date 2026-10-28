# netex2gtfs

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
    let options = Options { shapes: true, ..Options::default() };
    let report = convert(&delivery, &options, output::at("gtfs.zip")?)?;
    println!("{}", report.rows["trips.txt"]);
    Ok(())
}
```

## How a feed is made

The delivery is read once, on every core. The index of netex-rs gives stops,
lines, journeys with their calls and the days they run, walks and interchanges —
each reference resolved in its own document first, then in the delivery, where
the document whose name sorts first answers; what a feed needs beyond it is read
in the same walk of each document and resolved by the same rule.

| File | From | Rules |
|---|---|---|
| `agency.txt` | the operator of each line, else its first additional operator, else its authority (as a journey planner's importer orders them); the operator a journey names where its line names none | name, `ContactDetails/Url` and `Phone`; the time zone of the document defining it, else the delivery's first, else the one given, else the profile's country's |
| `stops.txt` | every stop place, quay and stop point a trip calls at | a quay under its stop place as a station (`location_type` 1) — under the parent site that stop place is within (`ParentSiteRef`), where it is within one, as GTFS stations do not nest —; a stop place called at itself a stop; a stop point no stop is assigned a stop at its own position; `wheelchair_boarding` from `MobilityImpairedAccess` (or `AllAreasWheelchairAccessible`), `platform_code` from a quay's `PublicCode`, `level_id` from its `LevelRef` |
| `levels.txt` | the levels of quays in the feed | `level_index` from the level's `PublicCode`, else `RelativeLevelOrder` |
| `routes.txt` | the lines of the trips | one route per NeTEx identifier — a line's versions are one route —; `route_short_name` its `PublicCode` (else `ShortName`), `route_type` from its mode, colours from `Presentation` |
| `trips.txt` | the journeys of the index | `service_id` the journey's calendar; `trip_headsign` the destination display at its first call; `trip_short_name` its train number (`ForAdvertisement`), else its `PublicCode`; `direction_id` from its, else its pattern's `DirectionType` or `DirectionRef`; `wheelchair_accessible` from its accessibility assessment |
| `stop_times.txt` | the calls the index makes of each journey | a point passed through is no call; boarding and alighting as the point or call says; times past midnight past 24 hours; `stop_headsign` where the display shown from a point on differs from the trip's |
| `frequencies.txt` | the headway groups of template journeys | from the first departure to the last, `exact_times` 0: a headway group states an interval, not departures |
| `calendar.txt`, `calendar_dates.txt` | the calendars of the trips | the weekdays a calendar runs on on at least half their dates from its first day to its last, and each date it differs from them on |
| `transfers.txt` | walks between stops (`SiteConnection`), planned interchanges | a walk `transfer_type` 2 with its duration, the longest of those stated between two stops; an interchange staying seated 4, guaranteed 1, else 0, once between the same trips at the same stops |
| `shapes.txt` | the geometry of the service links between a trip's calls (with `Options::shapes`) | each link read in its reference system — its own `srsName`, else its document's — and in the axis order whose first position lies near the link's start |
| `feed_info.txt`, `attributions.txt` | the publisher given, else the delivery's data source with a web address; the data sources and participants | the feed's period from its calendars, its version the latest publication timestamp |

GTFS requires positions in WGS 84; netex2gtfs reads netex-rs with its `crs`
feature, so a delivery writing its positions in a national reference system
converts as one in WGS 84 does.

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
| `trip-at-unplaced-stops` | a journey calling at a stop the feed cannot place: left out |
| `stop-without-position` | a stop without a position of its own, its station's or its quays': not in the feed |
| `quay-at-its-stop-place` | a quay without a position: placed at its stop place |
| `stop-place-at-its-quays` | a stop place without a position: placed at the mean of its quays' |
| `stop-place-called-at-beside-its-quays` | calls at a stop place that is also the station of its quays: a stop `<id>#place` under it |
| `stop-made-unique` | a global identifier (GlobalID, SLOID) several stops of the feed carry (with global stop identifiers): the second and each after it `<id>#<n>` |
| `stop-point-as-stop` | a stop point assigned to no stop: a stop of its own |
| `stop-named-by-its-station` | a stop without a name: named as its stop place |
| `level-without-number` | a level with no number: the quay is written without it |
| `route-made-unique` | a line defined again otherwise: a route `<id>#<n>` |
| `route-type-unknown` | a line of a mode GTFS has no type for: written as a bus |
| `route-without-agency` | a line naming no organisation the delivery defines: the publisher given is its agency; without one, the route names none |
| `agency-without-url` | an organisation without a web address: the publisher's |
| `trip-made-unique` | a journey identifier the index holds twice: `<id>#<n>` |
| `trip-time-goes-back` | a journey whose passing times go back along its calls: left out |
| `walk-stated-twice` | a walk between two stops stated again: one transfer, the longest |
| `interchange-left-out` | an interchange with a trip or stop the feed does not hold |
| `interchange-repeated` | an interchange between the same trips at the same stops stated again — in another document —: one transfer, the first |
| `shape-incomplete` | a trip with a pair of calls no link with a geometry joins: no shape |
| `shape-in-unknown-system` | a link geometry in a reference system netex-rs does not know: its stretch left out |
| `time-zone-of-profile` | no document names a time zone and none is given: the zone of the delivery's profile's country (DELFI Europe/Berlin, Austria Europe/Vienna, Switzerland Europe/Zurich) |
| `feed-info-without-publisher` | no publisher and no data source with a web address: no `feed_info.txt` |

## Licence

Licensed under either of the [Apache License, Version 2.0](LICENSE-APACHE) or the
[MIT licence](LICENSE-MIT), at your option.
