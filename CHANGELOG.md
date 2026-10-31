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

### netex2gtfs

A quay defined beside its stop place gives its platform code, level and
accessibility to its stop; train numbers are read wherever they stand, among a
general frame's members as well. A French delivery naming no time zone takes
none from its profile: France has more than one.

### netex-cli

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
