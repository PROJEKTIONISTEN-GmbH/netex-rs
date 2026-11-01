# Changelog

## Unreleased

The French profile, NeTEx France (`docs/FR-PROFILE.md`): deliveries that write
everything in general frames, define quays beside their stop place, state
passing times without the point they are at and give day types their days by a
validity and days of the week are read in full. Five deliveries of the French
national access point were read before and after; the three whose producers
publish GTFS beside their NeTEx give the same trips, days and stop times
(`docs/INTERPRETATION.md` §§ 23–27). Not compatible with 0.7: `Frame` and
`QuayItem`, which are exhaustive, have a variant more.

- **model**: `GeneralFrame`, its `members` (`GeneralFrameMembers`,
  `GeneralFrameMember`) and `Frame::General`; `QuayItem::QuayRef`, a quay a stop
  place names by reference.
- **stream**: `FrameKind::General`; `Kind::Quay`, a quay standing on its own,
  in `Selection::routing`.
- **index**: a quay defined on its own belongs to the stop place naming it among
  its quays, else to the one its `SiteRef` names; a passing time naming no point
  is at the point of its position; an operating period without day bits gives the
  days of the week its day type names, and a day type with a validity none of
  whose assignments gives it a day of its own runs on its validity's days of the
  week, its assignments adding and taking away. New findings:
  `QuayOfTwoStopPlaces`, `PassingTimesOtherThanPoints`, `PropertiesOfDayUnread`,
  `WalkWithoutDuration` (a walk without a duration, before left out without a
  word). A stop place and its quay naming each other are no use of either in
  `Referrers::unreferenced`.
- **calendar**: `DaysOfWeek`, `Properties` and `properties`, the rule for every
  reader.
- Own example `tests/fixtures/own/fr/`, the fragment
  `quays-beside-their-stop-place.xml`.

The Nordic NeTEx Profile (`docs/NORDIC-PROFILE.md`): journeys dated by the
dated journeys naming them, flexible lines as lines, operating periods stated
by their operating days, and the stop register a delivery names read along
with it. Norway's national delivery was read before and after: 106 259
journeys dated only by dated journeys run on their days now, and its GTFS
conversion has, trip for trip, the days, stops and times of the GTFS feed Entur
publishes from it, but for differences each explained (`docs/INTERPRETATION.md`
§§ 28–32). A calendar no longer spans conditions no journey names: on
Norway's delivery `netex stats` took 356 s and 8.6 GiB, and takes 9 s and
1.8 GiB. Not compatible with
0.7 either: `LineItem` and `GeneralFrameMember` have a variant more, `Line`
and `ServiceJourney` a field more.

- **model**: `LineItem::FlexibleLine` and `GeneralFrameMember::FlexibleLine`,
  read into `Line`, which types `FlexibleLineType`;
  `ServiceJourney::dated_service_journey_refs`, the dated journeys a dated
  journey replaces as the Nordic profile names them (`DatedServiceJourneyRef`).
- **stream**: `Kind::FlexibleLine`, in `Selection::routing`.
- **dataset**: `Source::with`, several sources read as one — a delivery with
  the stop register it names, published apart —, the documents of each source
  after the first named under its file or directory name.
- **index**: a dated journey naming a journey and an operating day and stating
  no journey of its own gives the journey that day, unless it is cancelled or
  replaced; a flexible line is a line, and a journey whose line, route's line or
  pattern's line is flexible is left out (`Concern::FlexibleLineJourney`); an
  operating period stated by `FromOperatingDayRef`/`ToOperatingDayRef` takes
  their dates; the calendar spans only the days that can give a journey a day,
  and lists a calendar's days by its set bits.
- Own example `tests/fixtures/own/nordic/` with its stop register
  `tests/fixtures/own/nordic-stops/`.

The European Passenger Information Profile as four neighbours publish it
(`docs/EPIP-NEIGHBOURS.md`): Luxembourg's and Denmark's national deliveries,
the four Belgian operators' and the Czech national ones were read before and
after. Luxembourg and Denmark read as they did. The Belgian operators write
the bounds of their operating periods as instants in UTC, and every period
began a day early: STIB's and TEC's journeys now run, trip for trip, on the
days of the GTFS feeds the operators publish beside the NeTEx. The Czech
buses and city transport number their stop places from one in every
document: the bus delivery had 82 stops, every journey calling at another
document's; it has 98 129 now, each journey calling at its own document's
(`docs/INTERPRETATION.md` §§ 33–35).

- **calendar**: `bound_day`, the day a bound of an operating period names —
  from noon on, an instant with a zone names the next day.
- **model**: `DateTime::states_zone`.
- **index**: a stop place, quay or stop point is gathered by identifier,
  version and name; one another document names otherwise is a stop of its
  own, which the references of the documents defining it resolve to
  (`Concern::LocalIdentifier`; `Index::stop_by_id` names the stop of the
  document named first). A `StopPlaceRef` of a stop assignment naming a
  quay, or a `QuayRef` naming a stop place, is counted as resolved to
  another class.
- Own examples `tests/fixtures/own/lu/`, `dk/`, `be/` and `cz/`.

### netex2gtfs

A delivery read with the stop register it names converts with its stops
(`Source::with`); journeys dated by dated journeys run on their days. Flexible
lines, typed now, are reported as not carried, as they were untyped.

A quay defined beside its stop place gives its platform code, level and
accessibility to its stop; train numbers are read wherever they stand, among a
general frame's members as well. A French delivery naming no time zone takes
none from its profile: France has more than one.

### netex-cli

`netex validate`, `stats`, `convert` and `extract` read documents along with
the source: `--with PATH`, a stop register published apart, as Norway's.
`netex validate` reports the class `local-identifier`, with the journeys
calling at such stops.

`netex validate` reports the four new findings (`quay-of-two-stop-places`,
`passing-times-other-than-points`, `properties-of-day-unread`,
`walk-without-duration`), in its JSON schema too.

## 0.7.2 — 2026-10-29

The README says what the crates are for: why official timetable data comes as
NeTEx, what makes a national delivery hard to read, what one builds with the
crate, and how to start with the library and with the `netex` command; it
carries the badges of crates.io and docs.rs. The code is that of 0.7.1.
netex2gtfs 0.1.3 and netex-cli 0.1.3 are published alongside, on netex-rs
0.7.2, with the same badges; netex-cli's README installs it from crates.io.

## 0.7.1 — 2026-10-29

The first published release. The code is that of 0.7.0; added are the
acknowledgements of the standard and its test data, a test that keeps the
repository's prose free of internal references, the packaging metadata for
crates.io and docs.rs, and the continuous integration of all three packages.
netex2gtfs 0.1.2 and netex-cli 0.1.2 are published alongside, on netex-rs 0.7.1.

## 0.7.0 — 2026-10-26

The merge of several deliveries is the library's: `netex_rs::merge`, which the
`netex merge` command of netex-cli 0.1.0 brought, so that whoever joins NeTEx
deliveries can call it without the command. Moved, not changed — the same rules,
findings and output; its tests moved with it (`tests/merge.rs`).

- **`merge`**: `merge(inputs, to)`, `Input`, `Report`, `DeliveryReport` — stop
  places paired by identifier, else by global identifier (`GlobalID`, `SLOID`),
  objects stated alike kept once, identifiers taken renamed into the delivery's
  codespace, journeys stated in two deliveries kept once (`docs/INTERPRETATION.md`
  § 16).
- **`written`**: a document as written — `scan` (where each element and the value
  of each attribute stand), `outline` (the objects of its frames, what they refer
  to and hold), `edit` (elements left out, values replaced, every other byte as it
  was) —, what the merge rewrites by and what a cut of a delivery needs.
- **`findings`**: the classes of findings with their counts and first examples, and
  the errors that stop a rewrite.
- **The feature `serde`** makes the reports serializable (`Report`, `Finding`); the
  crate needs no serde without it.

### netex-cli 0.1.2

Built on netex-rs 0.7.0: `netex merge` calls `netex_rs::merge`, `extract` writes by
`netex_rs::written` and reports by `netex_rs::findings`; the command, its arguments
and its reports are unchanged.

### netex2gtfs 0.1.2

Built on netex-rs 0.7.0; unchanged.

## Before 0.7.0

The versions before 0.7.0 were not published. In short, from the first tagged
version on:

- **0.2** (2026-10-04 to 10-11): NeTEx 1.x and 2.0 read and written into one
  typed model, everything the model does not name carried through as written;
  the releases told apart by the elements, attributes and enumeration tokens
  one declares and the other does not, and a document written for either with
  its losses reported; datasets read on every core; the index; the lean stream
  (`stream::read`, `stream::read_with`).
- **0.3** (2026-10-14): references resolved in their own document first; every
  decision of the index a finding, with a policy over their counts; the
  calendar rules (`calendar`); calls, availability conditions and template
  journeys (the Swiss form); passed points; the axis order of a link's
  geometry.
- **0.4** (2026-10-18): the package without the official schemas and examples,
  which stay test data of the repository; the licence files, own example
  deliveries under `tests/fixtures/own/`, the README for readers from outside
  and a CI workflow.
- **0.5** (2026-10-18): the repository a workspace with netex2gtfs 0.1.0 and
  netex-cli 0.1.0 beside the core; who refers to an object
  (`Index::referrers`); positions in national reference systems, the feature
  `crs`; own test fragments under `tests/fixtures/fragments/`.
- **0.6** (2026-10-20): the Italian profile of South Tyrol (IT-ITH1) read in
  full; a line named through a route; train numbers of a timetable frame and
  submodes beside a mode typed; netex2gtfs 0.1.1 and netex-cli 0.1.1 on it.
