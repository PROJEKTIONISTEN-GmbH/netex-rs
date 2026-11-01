# The Dutch profile (BISON) and the stop register

The Netherlands publish their timetables operator by operator in the Dutch
NeTEx profile of BISON (TMI9 NeTEx-NL, *Dienstregeling* 9.3.0, frames typed
`NL:BISON:TypeOfFrame:NL_TT_BASELINE`), at the national access point NDOV
Loket under CC0. The stops are those of the national stop register, the
*Centraal Haltebestand* (CHB), published apart, in NeTEx and in a format of
its own. Positions are in RD New (EPSG:28992). This page says what the
deliveries state, how `netex-rs` reads them, what changed for them, how they
compare with the national GTFS feed built from the same timetables, and what
they do not give. Nothing of any delivery is in this repository: the crate's
own examples of their forms, invented, are `tests/fixtures/own/nl/` and
`tests/fixtures/own/nl-register/`.

## The deliveries

NDOV Loket (`data.ndovloket.nl/netex/`) holds a folder per operator, each
with one or a few documents, gzipped, plus the profile's shared lists
(`enum/`) and the register in NeTEx (`epiap/`); the register's own export
is under `haltes/`. The national railway publishes no NeTEx (its timetable is
the IFF format), and neither do the operators whose folders are empty. Six
operators of different makes were read, with the register:

| | Published | Gzipped, inflated | Frames | Identifiers |
|---|---|---|---|---|
| RET (Rotterdam: metro, tram, bus) | 2026-10-26 | 3.2 MB, 144 MB | a resource and a service frame, 76 timetable frames | `NL:RET:ServiceJourney:LGB020\|11787\|WE\|5\|477902` |
| GVB (Amsterdam: metro, tram, bus, ferry) | 2026-10-28 | 4.3 MB, 204 MB | the profile's lists in five general frames, the timetable's frames, a calendar and a vehicle schedule frame | `NL:GVB:ServiceJourney:4-16695-5192574` |
| Arriva (every concession, trains among them) | 2026-10-23 | 13 MB, 473 MB | the timetable's frames, a calendar and a vehicle schedule frame | `NL:ARR:Line:13304#GT:P11431` |
| Connexxion, Arnhem-Nijmegen | 2026-10-31 | 2.0 MB, 89 MB | as GVB's | `NL:CXX:Line:A001` |
| Qbuzz, Groningen-Drenthe | 2026-10-23 | 2.7 MB, 78 MB | the lists in general frames, no calendar frame | `NL:QBUZZ:Line:g051` |
| HTM, trams | 2026-10-19 | 0.8 MB, 33 MB | as Arriva's | `NL:HTM:ServiceJourney:255138:3:36005` |
| The register (NeTEx, `epiap`) | 2026-10-31 | 3.3 MB, 89 MB | one site frame: 25 631 stop places, 49 836 quays | `NL:CHB:Quay:31000263`, code `NL:Q:31000263` |

Every document states `version="ntx:1.1"` but writes NeTEx 2.0's
`privateCodes`. HTM's, checked with `xmllint`, is valid against the schemas
of NeTEx 2.0 but for its key references to what other documents define (the
register's quays, the profile's codespaces and frame types); against NeTEx
1.x, `privateCodes` is not allowed.

## What the profile states, and how the crate reads it

- **Journeys timed by a time demand type.** No journey lists its passing
  times. Each states its departure (`DepartureTime`, `DepartureDayOffset`)
  and names a `TimeDemandType`, whose run times (`JourneyRunTime`) are given
  per timing link and wait times (`JourneyWaitTime`) per timing point; its
  pattern names, for each point, the timing link onward
  (`OnwardTimingLinkRef`), and passes timing points that are no stop
  (`TimingPointInJourneyPattern`). The journey leaves its first point at its
  departure and runs from point to point; `docs/INTERPRETATION.md` § 36.
- **Run times to a tenth of a second.** GVB states `PT1M3.4S`. The times add
  up to the millisecond, each passing time rounded to the second, a half up
  (§ 37).
- **Loops.** A pattern may pass a link, or a point, twice; the type then
  states a run time per pass, in pattern order — RET's line 612 runs the same
  links through a town twice, 30 s the first time and 46 s the second. A wait
  stated once for a point visited twice is the layover at its last visit
  (§ 38).
- **The calendar is availability conditions.** Each journey names one, of
  day bits, defined in the timetable frame's `contentValidityConditions`, a
  container of every NeTEx frame the crate did not read before. GVB's,
  Arriva's and Connexxion's journeys name a day type beside it, assigned by
  dates, which the profile states for information: where the two differ, the
  day bits lead (*Dienstregeling* 9.3.0, § 3.2, rule 7). The index keeps the
  days both give; in the three deliveries, every one of the 228 245
  journeys naming both is given each day of its conditions by its day types
  too, so its days are its conditions'. A condition not available
  (`IsAvailable` false) shows the validity a journey no longer has; a
  journey naming only such a condition runs on no day (§ 39).
- **Private codes.** Lines, journeys, stop points, timing points and the
  register's stops state codes in NeTEx 2.0's `privateCodes`: line planning
  numbers, journey numbers, user stop codes, and the register's
  `StopPlaceCode` `NL:S:…` and `QuayCode` `NL:Q:…`. The model reads
  `privateCodes` beside `keyList` (`PrivateCodes`), but on a passing time and
  a stop point in a pattern, where it stays untyped.
- **The register's codes are the stops' global identifiers.** The German
  delivery names the Dutch stops it serves by `GlobalID` `NL:Q:…` and
  `NL:S:…`; the register states the same keys as private codes. A stop's
  global identifier (`Stop::dhid`, `Index::stop_by_dhid`) is its `GlobalID`,
  else its register code: one key finds the stop in both deliveries,
  `netex convert --stop-ids global` writes it as the stop's identifier, and
  `netex merge` pairs the two stops by it (§ 40).
- **Positions in RD New**, the frames' `DefaultLocationSystem`, read with
  the feature `crs` (see below).
- **The stops are the register's.** The timetables define no site; their
  stop assignments name the register's quays (`NL:CHB:Quay:…`), and are read
  with it: `netex … --with <register>`, `Source::with`.

### Read before and after

`netex stats <operator> --with <register>`, before this change and after;
the register's own stops are counted where a journey calls at them.

| Delivery | Before | After: stops, lines, journeys, journey days (days) |
|---|---|---|
| RET | no journey | 1 976, 96, 43 528, 422 754 (2026-10-26 to 2027-01-03) |
| GVB | no journey | 1 313, 76, 70 743, 880 686 (2026-11-01 to 2027-01-31) |
| Arriva | no journey | 10 523, 594, 142 905, 4 660 631 (2026-10-23 to 2027-12-11) |
| Connexxion | no journey | 2 728, 112, 13 902, 207 311 (2026-11-03 to 2026-12-12) |
| Qbuzz | no journey | 3 970, 119, 14 290, 165 658 (2026-11-01 to 2026-12-12) |
| HTM | no journey | 490, 22, 7 147, 90 131 (2026-11-02 to 2026-11-27) |

Before, every journey was left out (`too-few-timed-calls`): it states no
passing time, and its calendar was in a container the crate did not read.
Connexxion's journeys counted are those running on a day: 1 544 more name
only a condition not available and 695 a condition of no day bit.

The share of the element occurrences the typed model carries through as
fragments (`examples/inventory`), before and after:

| Delivery | Before | After | What is left untyped |
|---|---|---|---|
| RET | 3.484 % | 0.544 % | GML positions, stop points' projections and stop areas, destination display variants |
| GVB | 5.426 % | 1.729 % | dead runs and their patterns, GML positions, projections, stop areas, tariff zones |
| Arriva | 5.664 % | 2.371 % | as GVB's |
| Connexxion | 4.487 % | 1.885 % | as GVB's |
| Qbuzz | 6.225 % | 3.035 % | GML positions, projections, tariff zones, stop areas |
| HTM | 3.003 % | 1.048 % | as GVB's |
| The register | 12.971 % | 6.672 % | GML positions (76 167), polygons, equipment, path links |

A GML position is an element of another namespace, carried through by the
model and read by the feature `crs` where it stands.

## Positions in RD New

RD New is read by EPSG's Helmert transformation `Amersfoort to WGS 84 (4)`
alone, without the correction grid of RDNAPTRANS2018, the official
transformation. Against RDNAPTRANS2018 through its grid
(`nl_nsgi_rdtrans2018.tif`, PROJ 9.5.1), over every one of the 74 736 stop
places and quays of the register stating a position, at the position the
register writes: median 0.056 m, 99 % within 0.182 m, at worst 0.236 m
(Ihrhove, in Germany). Over a lattice spanning the country
(`tests/fixtures/crs/reference.tsv`), 0.182 m at worst. The register writes
its positions to the whole metre — half a metre of rounding —, and so do five
of the six timetables' stop points (Qbuzz's to the centimetre): a correction
grid would not make a position more precise than the register states it.
`tests/crs.rs` pins six quays of the register, among them the centre of the
system, a stop in Belgium, two in Germany and the worst one.

The national GTFS feed places its stops otherwise: by the operators' own
positions. Of 46 354 quays both have, 26 % lie within half a metre of each
other, 83 % within 10 m; 301 lie more than 100 m apart, 14 more than a
kilometre.

## Checked against GTFS-NL

OVapi publishes a national GTFS feed (`gtfs.ovapi.nl/nl/gtfs-nl.zip`, of
2026-11-01) built from the operators' timetables.
Each operator's delivery was converted with the register
(`netex convert --with`) and compared with it day by day: a trip identified
by its line planning number and journey number (the private codes; GTFS-NL's
`realtime_trip_id`), on every day of the window both cover, by its times at
every call one may board or alight at — GTFS-NL leaves out the others, which
the crate keeps with neither allowed.

| Delivery | Window | Trip-days, ours and GTFS-NL | Equal | Of GTFS-NL's, at the ends shorter | Times differ | Only ours | Only GTFS-NL |
|---|---|---|---|---|---|---|---|
| RET | 31 Oct – 12 Dec | 265 238, 265 238 | 245 239 | 19 901 | 98 (85 s at most) | – | – |
| GVB | 1 Nov – 12 Dec | 420 798, 420 798 | 387 426 | – | 33 372 (1 s at most) | – | – |
| Arriva | 31 Oct – 12 Dec | 723 801, 656 497 | 656 107 | 390 | – | 67 304 | – |
| Connexxion | 3 Nov – 12 Dec | 207 311, 1 262 071 | 200 553 | – | – | 6 758 | 1 061 518 |
| Qbuzz | 1 Nov – 12 Dec | 165 658, 862 646 | 164 588 | – | 6 (60 s) | 1 064 | 698 052 |
| HTM | 2 Nov – 27 Nov | 90 131, 97 377 | 47 322 | 11 089 | – | 31 720 | 38 966 |

What differs, each traced to its cause:

- **Calls at the ends.** GTFS-NL begins or ends trips later or earlier than
  their pattern; the times of the calls both have are equal.
- **GVB's single seconds.** Where the sum of run times stated to the tenth
  falls on a half second, GTFS-NL rounds down in some places and up in
  others, by a rule the data do not show: over a sample of 48 299 times,
  rounding the exact sum half up gives GTFS-NL's time for 97 %, half down
  for 92 %, the sum in binary floating point for 95 %, each run time rounded
  first for under half.
- **RET's 98.** The second pass of line 1602's loop: GTFS-NL gives it the run
  times of the first pass, the delivery states its own (§ 38).
- **Qbuzz's 6.** One journey with a wait stated at its first stop, which it
  visits once: GTFS-NL leaves the stop after the wait, taking the delivery's
  departure for the time it arrives there; the crate reads `DepartureTime`
  as NeTEx defines it, the departure. One journey of the 294 754 read.
- **Only GTFS-NL.** Connexxion's and Qbuzz's folders hold one document per
  concession; one of each was read, and GTFS-NL's trips of the others have
  lines the document has not. HTM's are its buses, of a document not read.
- **Only ours.** Arriva's: 59 525 trip-days of its trains, whose lines
  GTFS-NL has under no owner — it takes the trains from the railway's IFF
  timetable, keyed otherwise —, and 7 779 of buses of lines GTFS-NL carries,
  not traced further. Qbuzz's 1 064: journeys stating `Print` false and
  `Dynamic` `onlyIfSignedOn`, run only when a passenger signs on, which
  GTFS-NL leaves out and the crate keeps. HTM's: trams under line planning
  numbers GTFS-NL has not (53 and 54, which carry the public numbers 3 and 4
  beside lines 3 and 4; the event lines 89, 90 and 97; a test line 99).
  Connexxion's 6 758: journeys on lines GTFS-NL runs that day, under journey
  numbers it has not (line A027's journey 1001, from Monday to Friday in the
  delivery, is in no trip of GTFS-NL); not traced further.

Before the rule of § 39, Connexxion's 1 544 journeys naming only a condition
not available ran on every day of their day type but the one the condition
names: on 14 433 trip-days a journey was there twice or three times. Before
the rule of § 38 on waits, RET's line 29 waited 35 minutes in the middle of
its trips rather than at their end.

## What the deliveries do not give

- **Quays the register does not hold.** Every operator but HTM names some:
  quays the register's own export marks `expired` (RET 4, GVB 31, Arriva 55,
  Qbuzz 11), stopped being published in the NeTEx register while the
  timetables still serve them; Arriva's trains' platforms, named
  `NL:CHB:Quay:<station>_<track>` (`NL:CHB:Quay:gn_1`), which the stop
  register does not hold (333); and Connexxion's 31 references written
  `NL:CHB:quay:…`, with a small letter — an identifier is matched as written.
  Each is an `unresolved` finding; the stop point stays a stop of its own at
  its own position.
- **Names of quays.** The register's quays state no name; a converted stop is
  named as its stop place.
- **Positions of rail platforms.** The register holds 731 platforms of
  railway stations stated by their outline alone (`gml:Polygon`, no
  centroid): stops without a position in the index. Their identifiers
  (`NL:CHB:Quay:8400059_7n8`, code `NL:Q:ass_7n8`) are not those Arriva's
  trains name.
- **Rail.** The national railway publishes no NeTEx.
- **The register's own export** (`ExportCHB_<date>.xml`, 53 222 quays with
  their status, accessibility and facilities) is a format of its own, not
  NeTEx: not read. Its positions are those of the NeTEx register.

## Time and memory

`netex stats` with the register read along, 8 threads, base and new
alternately (base, new, new, base) under the shared measurement lock; load
14 to 18 on 64 cores. Wall time and peak resident memory, the mean of each
pair:

| Delivery | Before | After |
|---|---|---|
| RET | 1.5 s, 0.26 GiB | 1.6 s, 0.27 GiB |
| GVB | 1.9 s, 0.34 GiB | 1.8 s, 0.34 GiB |
| Arriva | 3.5 s, 0.58 GiB | 4.0 s, 0.59 GiB |
| Connexxion | 1.2 s, 0.20 GiB | 1.2 s, 0.21 GiB |
| Qbuzz | 1.2 s, 0.18 GiB | 1.2 s, 0.19 GiB |
| HTM | 1.2 s, 0.17 GiB | 1.2 s, 0.17 GiB |
| The register alone | 1.1 s, 0.12 GiB | 1.1 s, 0.12 GiB |
| Arriva, `netex validate` | 3.8 s, 0.57 GiB | 16.5 s, 0.58 GiB |

Before, the index held no journey; the time added is the journeys' — laying
their times along the time demand types, and for `validate` checking each.

Every delivery of the other profiles the crate is checked on — the German
national delivery, the nine Austrian ones, five French, Norway's, Luxembourg's,
Denmark's, the four Belgian, three Czech, and the crate's own examples — gives
the same index as before, entity for entity (a summary hash over every stop
and journey, its calls, times and days).

## Not read

The timing links and timing points themselves (named by identifier, carried
through), dead runs and the vehicle schedule frame, tariff zones, stop areas,
notices of the register, the profile's shared lists.
