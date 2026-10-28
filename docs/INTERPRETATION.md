# Reading NeTEx: the questions a reader has to answer

NeTEx says what a timetable may state; it leaves open how a reader turns what a
delivery *does* state into something a journey planner can ride. Every reader
answers the same questions — what a reference to nothing means, which days a
journey runs on, what to do with a stop place defined in three hundred
documents — and the answers decide what the passenger is shown.

This catalogue lists those questions as our deliveries pose them. For each:
the problem, with evidence from **our** data (delivery, document, identifier);
what the standard says; what an importer built on this crate does today; what
`netex-rs` does; and what we propose for both, with an estimate of the effort.

**Sources.**

- The deliveries: the Austrian Verbünde's of 2026-09-23 to 2026-10-08
  (`*_netex_<verbund>_2026.zip`, nine of them), DELFI's of 2026-10-12
  (`20261012_fahrplaene_gesamtdeutschland.zip`; where a figure is for a part,
  the part is the 276 line documents of the GVH and BSAG folders), the Swiss of
  2026-10-10 (`prod_netex_tt_1.10_che_ski_2026_oev-schweiz…zip`; the part is its
  five shared documents and the timetable documents 1 to 20), and South Tyrol's
  in the Italian profile from the Italian national access point, fetched
  2026-10-20 (`IT-ITH1_L1.xml.gz`, one document, published 2025-08-14;
  `docs/IT-ITH1-PROFILE.md`).
- *The importer*: the NeTEx importer of a journey planner, built on this crate
  and not published; it is cited for what it does with the same deliveries.
- The standard: the NeTEx XML schema (CEN/TS 16614, the `xsd/` of the NeTEx
  repository, branch `v2.0`), cited by schema file and element. The schema's own
  documentation is short; where it says nothing, this says so.
- The figures come from reading the deliveries with `netex-rs` and the
  importer and from counting their elements.

**Proposal:** *adopt* (a reader should answer it and we do not yet), *better*
(we answer it and should answer it otherwise), *keep* (our answer is right),
*not needed* (no delivery of ours poses it).
**Effort:** S up to a day, M up to a week, L more.

---

## 1. References that do not match what they point at

**Problem.** A reference names an identifier, optionally a version, and — by
its element name or `nameOfRefClass` — the class of what it means. Deliveries
get all three wrong in their own ways.

- *The class.* Every day type assignment of the Austrian and German deliveries
  refers to a `UicOperatingPeriod` through an `OperatingPeriodRef`: 1 194 of
  1 194 in the Linz AG delivery (`at:esg:UicOperatingPeriod:05i00:3:` from
  `NX-PI_01_at_esg_LINE_esg_1-2-E-e25_20260923.xml`), 2 533 of 2 533 in the
  DELFI part (`DE::UicOperatingPeriod:1020::`). In the schema a
  `UicOperatingPeriod` is a kind of `OperatingPeriod`, so the reference is
  valid — but a reader that looks up "an `OperatingPeriod` called X" finds
  nothing.
- *The version.* All three deliveries mostly write `version="any"` on
  references (the Swiss `versionRef="any"`); not one reference in the three
  parts names a version other than the one defined.
- *Nothing.* A reference to an identifier nothing defines: 30
  `ScheduledStopPointRef`s in the Swiss part (`8509083` in timetable document
  4 — a bare number, not an identifier of the delivery); 64 100
  `ServiceJourneyRef`s in the Swiss shared document's meetings and rules to
  journeys of the 495 timetable documents the part leaves out; 443
  `TrainNumberRef`s of the Salzburg delivery to train numbers no document
  defines. A DELFI line file's interchanges name the feeder in another line's
  file (148 in the RE19 file of the crate's fixtures).

**Standard.** A reference is `ref` plus optional `version` or `versionRef`
(mutually exclusive: `ynotation/netex_subThing_support.xsd`, `versionRef`);
`nameOfRefClass` is "fixed for each entity type" and allows reflection. The
schema's key references (`xsd:keyref`) check identifiers within one document
only.

**The importer.** Resolves by identifier within the document (the German and
Austrian deliveries) or within the document and the delivery's shared documents
(the Swiss), ignoring version and class; a day type assignment's period is
found whatever kind it is. A reference to nothing is a counted warning
(`UnresolvedReference`), and what referred is left out — for a call, the call
alone.

**netex-rs.** The index resolves by identifier, at the highest version or the
one valid on a day (`IndexBuilder::at`); a reference to nothing is a finding
(`Index::unresolved`) with the document, the referrer and the element. Since
0.3.0 it counts what resolved only because version or class were set aside
(`Concern::ResolvedIgnoringVersion`, `Concern::ResolvedAsOtherClass`, the
version or class found named in the finding); a reference naming `any` resolves
as one naming none. On the nine Austrian deliveries every class finding is an
`OperatingPeriodRef` to a `UicOperatingPeriod` (1 194 in Linz, as above); no
reference resolved past its version.

**Proposal: keep, and better where it costs nothing.** On our deliveries the
only reference resolved past its class is the `OperatingPeriodRef` to a
`UicOperatingPeriod`, which a reader that resolves by identifier never needs to
repair. Do not repair references in the data: resolving by identifier and
recording the class found (`Unresolved` already names element and referrer) is
cheap and loses nothing. What is worth having is the *accounting*: count
references resolved only because version or class were ignored, beside the ones
not resolved at all. Effort **S**.

## 2. Identifiers that mean one thing per document

**Problem.** NeTEx identifiers are meant to be unique; the Linz AG delivery
reuses them per document with other contents. 32 of its 292
`UicOperatingPeriod`s are defined in two line documents with other day bits —
`at:esg:UicOperatingPeriod:34i00:3:` in `…_esg_1-2-E-e25_…` and in
`…_esg_3-46-E-f26_…` — and with them the 32 day types assigned to them; 94 `ServiceJourney`s are defined in two
documents with other content under one identifier and version
(`at:esg:ServiceJourney:02011E-Linz_Linien_GmbH-1-1-15840:dS:` in the `e25`
and the `f26` document of line 2-11). The Linz AG's is the only delivery with
many: Oberösterreich, Salzburg and Steiermark define one period each so, the
other five Austrian deliveries and the DELFI and Swiss parts none.

**Standard.** Silent. An identifier is unique within its codespace by
convention; the schema checks uniqueness per document.

**The importer.** Resolves each document's calendar within the document; reads
twins (§ 12) across documents.

**netex-rs.** The index merges all documents and keeps the definition it reads
*first* — and because documents are read on every core, largest first, which
one that is varies from run to run: three runs over the Linz AG delivery gave
14 089, 14 153 and 14 128 runs on 2026-10-15. **This is a defect of netex-rs**
(the stop places were made order-independent in 0.2.0, the calendars and
journeys were not); read per document, such a delivery gives the same every
time.

**Proposal: better.** netex-rs needs a scope for resolution: per document with
the delivery's shared documents beside it (the German, Austrian and Swiss
layouts all fit), and, where the delivery is one namespace, across it; and a
redefinition with other content must be a finding naming both documents, never
a silent pick. Effort **M**.

**Built in 0.3.0.** What a document defines of its own — lines, journey
patterns, journeys, day types, operating periods and days, availability
conditions — is gathered per document; a reference resolves in its own document
first and only then in the delivery, where of several documents defining an
identifier the one whose name sorts first answers. Stop places, quays, stop
points and levels stay gathered once, as before. A definition repeated alike
is merged; repeated with other content it is the document's own and a finding
(`Concern::RedefinedOtherwise`, both documents named) — for stops, stop points
and levels too, where the document named first keeps deciding. A journey stated
otherwise in two documents is two journeys. The index is numbered in the order
of the documents' names, so it is the same whatever order and threads read it:
the Linz AG delivery gives 14 163 runs on 2026-10-01 on 1, 3 and 8 threads with
one fingerprint (before: 14 107, 14 037, 14 121 in three runs), and the same
41 945 journeys with the same days and calls as the document-by-document
reading; 194 definitions repeated with other content are findings — 50 of the
periods and 50 of the day types (the 32 above, some in more than two documents)
and the 94 journeys.

## 3. Operating days

**Problem.** A journey's days come from day types, which get days from
assignments to operating periods (plain, from date to date), UIC operating
periods (a bit per day), operating days or single dates, each assignment
possibly *removing* days (`isAvailable=false`), and a day type possibly
restricting to weekdays (`PropertyOfDay/DaysOfWeek`). The Swiss delivery uses
availability conditions on the journey instead (86 127 in the part, every one
of its 102 387 journeys names one), plus 18 day types with properties assigned
to single dates (`ch:1:DayType:Allerheiligen`). The Austrian and German
deliveries use day type → assignment → UIC period throughout (1 194 in Linz,
2 533 in the DELFI part); no plain operating period, no operating day.

**Standard.** `DayTypeAssignment` associates a day type with an operating day
or period (`netex_framework/netex_reusableComponents/netex_serviceCalendar_version.xsd`);
`ValidDayBits` holds one bit per day of the period and a missing bit means
available (`netex_availabilityCondition_version.xsd`); an `AvailabilityCondition`
is a validity condition stated in day types and properties of days. How several
assignments of one day type combine, and how a journey's own validity
conditions combine with its day types, the schema does not say.

**The importer.** Assignments in order of their `order`, later ones overriding,
removal by `isAvailable`, UIC bits, plain periods, operating days and dates; a
journey runs on the union of its day types' days, within its availability
conditions where it names any (the Swiss form); bounded by the version of its
line (§ 4).

**netex-rs.** The index reads day types, assignments, plain and UIC periods and
operating days into one bit set per journey (`Index::runs_on`); it does not
read availability conditions or properties of days. Since 0.3.0 it applies
the importer's rules, which are the crate's now (`calendar`): assignments in the
order of their `order`, those stating none last, later ones overriding; a
journey's day types within its availability conditions, or the conditions'
days alone where it names no day type, a condition not available taking its
days away; a period without a first day or with other bits than days a finding
(`Concern::PeriodWithoutFirstDay`, `Concern::DayBitsLengthDiffers`), a journey
without a day kept and counted (`Concern::JourneyWithoutDays`). Properties of
days stay unread.

**Proposal: keep the importer's rules, move them into netex-rs.** What a
journey's days are is a question every reader asks; the answer belongs into the
crate, with the open cases (several properties of day, an assignment outside
its period, a journey's conditions beside its day types) as findings rather
than silent drops. *Writing* a calendar as UIC bits is what a writer of EPIP
needs (`PROFILE-UEBERFUEHRUNG.md`). Effort **M**.

## 4. Versions of a line, and what they govern

**Problem.** The Austrian deliveries write a line in versions with a validity
each, and the day bits of the journeys over the whole timetable year. Two forms:

- one identifier per timetable, the next one in another document:
  `at:esg:Line:01001E.e25:` valid 2026-01-02 to 2026-08-30, its successor in
  an `f26` document; a journey of the `e25` line runs, by its day type, on
  2026-10-15 too (`at:esg:ServiceJourney:01001E-Linz_Linien_GmbH-1018-1-41040:ag:`);
- one identifier in several versions: `at:ktn:Line:0100A_.j26:` version 1
  valid 2025-12-14 to 2026-12-12, version 2 from 2026-05-30, version 3 for four
  days in August; a journey naming version 1 runs, by its day type, on
  2026-10-15 (`at:ktn:ServiceJourney:01046-KMG-152-1-71700:T4:`).

Every Austrian line carries a `ValidBetween`; lines defined in more than one
version: 4 in Linz, 226 in Kärnten, 317 in Tirol. In the Swiss part every one of
the 4 189 lines carries a validity too.

**Standard.** `ValidBetween` is a simple validity condition of the entity
(`netex_framework/netex_responsibility/netex_validityCondition_version.xsd`).
Whether an entity's validity limits the journeys that *refer* to it the schema
does not say. Read by their day bits alone, 3 670 journeys of the Linz AG run on
2026-10-15 though their line's version is not valid that day, and 3 138 of
them repeat the stations and times of another run of the day exactly — the
successor's: the passenger would see the departure twice.

**The importer.** A version governs the days it is valid on, but those a later
version takes over; a journey runs on the days of its day types its line's
version governs (`line_versions`; counted, `journey on days its line version does
not govern`: 35 499 journeys in the Linz AG delivery, 70 561 in Kärnten).

**Proposal: keep, and move into netex-rs** together with § 3. Effort **S**.

**Built in 0.3.0** (`calendar::governed`, the index): the versions of a line are
the definitions of its identifier in one document; a journey runs on the days of
its calendar the version it names governs, and is left out where that leaves
none. Counted as the importer counts them, to the journey: 35 499 journeys of the
Linz AG and 70 561 of Kärnten with days taken away
(`Concern::DaysOutsideLineVersion`), 5 127 of Kärnten left out
(`Concern::JourneyOutsideLineVersion`).

## 5. Times after midnight

**Problem.** A journey leaving at 23:50 calls at 00:10 on the next calendar
day. NeTEx states the time of day and a day offset; some feeds write `24:10:00`
instead. None of our deliveries does: no time from 24 on in the Austrian, German
or Swiss parts; day offsets above zero 10 813 times in Linz, 70 566 in the
DELFI part, 123 943 in the Swiss part.

**Standard.** `ArrivalDayOffset`/`DepartureDayOffset`: "number of days after
the starting time of the journey if not same calendar day"
(`netex_part_2/part2_journeyTimes/netex_passingTimes_version.xsd`). `xs:time`
allows `24:00:00` only.

**The importer and netex-rs.** Both turn time and offset into seconds since the
operating day's start, past 86 400 after midnight. netex-rs reads an hour of 24
so as well: `24:10:00` without an offset is the 87 000 s that `00:10:00` with an
offset of one day is (`tests/own_fragments.rs`); a later hour is no time of day,
and the passing time is left without one.

**Proposal: not needed** for our deliveries; **adopt as a check** if a delivery
with hours past 23 ever arrives: a time stated both ways, an hour past 23 and a
day offset, is a finding. Effort **S**.

## 6. Calls instead of passing times

**Problem.** The Swiss delivery states every journey as a list of calls, each
with its stop point and times, instead of a journey pattern plus passing times
(102 387 of 102 387 in the part); the German and Austrian deliveries never do.

**Standard.** `calls` is the "complete sequence of stops along the route path,
in calling order" (`netex_part_2/part2_journeyTimes/netex_serviceJourney_version.xsd`);
a `Call` is "a view of a point in journey pattern that adds in derived data"
(`netex_call_version.xsd`).

**The importer.** Reads calls in place; journeys of a document calling alike
share a pattern (`calls`, `document`). Template journeys (`TemplateServiceJourney`)
are expanded into their runs.

**netex-rs.** Carried `calls` through untyped before 0.3.0; the index saw a
Swiss journey without stops.

**Proposal: better — type `Call` in netex-rs** and let the index take a
journey's stops from its calls where it has no pattern. Effort **M**.

**Built in 0.3.0.** `Call`, `AvailabilityCondition` (in a frame's or a journey's
`validityConditions`) and `TemplateServiceJourney` with its
`HeadwayJourneyGroup`s are typed, in the model and in the lean stream; the index
takes a journey's calls where it states them, journeys calling alike sharing one
list, and keeps a template's groups beside it (`Journey::headways`). The Swiss
part through the index: 102 413 journeys with their calls (102 387 journeys and
36 templates, less 10 left with fewer than two timed calls), 715 091 calls with a
time.

## 7. Planned interchanges

**Problem.** Three forms in our deliveries:

- DELFI: `ServiceJourneyInterchange`, every one stay-seated, none guaranteed
  (21 651 in the GVH/BSAG part), written into the *distributor's* document
  and naming the feeder, often in another; 17 764 of the 21 651 name one stop
  point on both sides, 3 887 two different ones.
- Austria: `JourneyMeeting` (a meeting of two journeys at a stop point, most
  of them from the feeder's last call into the distributor's first — a vehicle
  running on) and `InterchangeRule` (guaranteed connections from a journey to a
  line).
- Switzerland: both, in a shared document — 325 690 meetings, 60 984 rules in
  the part.

**Standard.** A `ServiceJourneyInterchange` is the "scheduled possibility for
transfer of passengers between two service journeys at the same or different
stop points"; `StaySeated` and `Guaranteed` default to false; a `JourneyMeeting`
is a time constraint fixing interchanges between journeys
(`netex_part_2/part2_journeyTimes/netex_interchange_version.xsd`); an
`InterchangeRule` states them indirectly, by mode, direction or line
(`netex_interchangeRule_version.xsd`).

**The importer.** Reads all three forms (`interchanges`): interchanges resolved
across documents, the feeder's point through whichever document assigns it, a
point the journey does not call at taken at the journey's end and counted;
meetings from last into first call as through-runs; rules into the first
journey of the line leaving within the window each day.

**Proposal: keep.** The importer reads every form our deliveries state —
interchanges, meetings and rules —, and counts a point off the journey's
pattern where it takes it at the journey's end. Moving such a point to the stop
of the pattern at the same station is worth building for a delivery that needs
it, not now. Effort none.

## 8. Levels

**Problem.** A quay, entrance or path link refers to the level it is on
(`LevelRef`). Every one in our deliveries refers to a level of its own stop
place (15 102 in Linz, 14 169 in the DELFI part, 37 in the Swiss part); none is
orphaned.

**Standard.** `LevelRef`: the level a site component is on
(`netex_part_1/part1_ifopt/netex_ifopt_path_version.xsd`).

**The importer and netex-rs.** The index resolves a quay's level to the level's
short name (`Stop::level`); the importer does not use levels.

**Proposal: not needed** for reading. A writer that puts sites and levels into
documents of their own has to keep every level reference resolvable there
(`PROFILE-UEBERFUEHRUNG.md`). Effort none.

## 9. Train numbers

**Problem.** The Swiss delivery gives every journey a `TrainNumberRef` (102 387
of 102 387, 7 637 `TrainNumber`s in the part); the Austrian deliveries write
`TrainNumber`s too (22 693 in the ÖBB/EVU delivery, 3 220 in Kärnten), the
Salzburg one 443 references to numbers it does not define; the German
delivery's journeys carry their number only in the identifier
(`DE::ServiceJourney:<number>_<part>::`).

**Standard.** `TrainNumber`: codes assigned to vehicle journeys operated by
trains (`netex_part_2/part2_journeyTimes/netex_vehicleJourney_version.xsd`).

**The importer.** Reads the train numbers, names a train on a line without public
code by its product and number (`RJX 265`), folds coupled parts (§ 12).

**Proposal: not needed.** None of our deliveries needs a number made up from a
name: those that state train numbers state them as `TrainNumber`s, the German
delivery in its identifiers. Effort none.

**netex-rs.** The South Tyrolean delivery states 558 `TrainNumber`s in its
timetable frame, named by 6 967 journeys. Since 0.6.0 a timetable frame's
`trainNumbers` are typed (`TrainNumber`: its number for advertisement and for
production; `Kind::TrainNumber` in the lean stream); before, they were carried
through untyped.

## 10. Who is responsible for a line

**Problem.** A line names the organisation responsible for it (`AuthorityRef`)
and the one running it (`OperatorRef`), and a journey planner shows one of them
as the line's agency. Our deliveries state one or the other, never both: every
DELFI line an authority and no operator (276 of 276 in the GVH/BSAG part);
every Austrian line an operator and no authority (167 of 167 in Linz,
`at:esg:Line:01001E.e25:` → `at:esg:Operator:1:`; 1 418 in Tirol); every Swiss
line likewise (4 189).

**Standard.** `AuthorityRef` and `OperatorRef` are both optional on a line;
`additionalOperators` lists further operators
(`netex_part_1/part1_networkDescription/netex_line_version.xsd`).

**The importer.** The line's agency is its operator, else its first additional
operator, else its authority.

**Proposal: not needed** for reading — the importer's order answers it. Giving
every line an authority, its operator among the additional operators, is what an
EPIP *writer* needs (`PROFILE-UEBERFUEHRUNG.md`). Effort none.

## 11. Stop points and where they are

**Problem.** A journey calls at scheduled stop points; a passenger stop
assignment says which stop place and quay a point is. Our deliveries:

- assign to a stop place and a quay (Linz 6 899 of 6 899; DELFI part 5 388),
  to a stop place only (DELFI part 7 512 — GVH has no quays; Swiss part
  21 167), or not at all (GVH/BSAG part 2: `DE::ScheduledStopPoint:10384201_GVH_::`;
  DB part 8 677, `DE::ScheduledStopPoint:8089126_DBDB_::`; Kärnten 4 118);
- define a stop place in every line document that serves it, each copy with
  only the quays that line uses: 147 Linz stop places differ between documents
  (`at:44:40002` in `…_esg_1-2-E-e25_…` and `…_esg_2-19-A-f26_…`), and the 598
  assignments with them.

**Standard.** A `PassengerStopAssignment` is "the default allocation of a
scheduled stop point to a specific stop place, and also possibly a quay and
boarding position" (`netex_part_1/part1_tacticalPlanning/netex_stopAssignment_version.xsd`).

**The importer.** Merges stop places by identifier *and* DHID, keeping the
definition of the document first by name and the quays of all; an unassigned
stop point with a position is a stop of its own (`StopPointAsStop`), one without
has no place and its calls are dropped (`CallWithoutPlace`); a quay without a
centroid is placed at its stop place.

**netex-rs.** The index merges a redefined stop place's quays into the kept one
(0.2.0) and resolves a stop point to its quay, else its stop place.

**Proposal: keep the merge, count near-duplicate stop places.** Two stop places
of one name within a few metres are a data defect worth counting, and joining
them a decision a dialect may take. Effort **S**.

## 12. Runs stated more than once

**Problem.** The Linz AG writes a line in up to three documents with the same
journeys (`…_1-1-A`, `-D`, `-E`); the Austrian railway states a train twice and
the parts of a coupled train as two journeys. On 2026-10-15 the Linz AG
delivery states 14 163 runs, of which 9 987 repeat the stations and times of
another.

**Standard.** Silent; a journey is an identifier, and these are different
identifiers.

**The importer.** Folds twins: a journey with the calls and times of a trip of
the same line or train and class added before runs only on the days no such trip
runs (`twins`, on where the build profile says `fold_twins`; 15 674 journeys of
the Linz AG delivery folded on every day, 3 249 on some).

**Proposal: keep.** Effort none.

## 13. Coordinates: axis order and reference systems

**Problem.** A `gml:posList` in WGS 84 (EPSG:4326) is latitude first. The Linz
AG delivery writes its service link geometries longitude first (`14.296652
48.334134 …`, `at:esg:ServiceLink:01_at-44-40044-0-0_at-44-41329-0-0:`): 1 391
of its links, and 116 910 over the nine Austrian deliveries. Every location of our deliveries is
`Longitude`/`Latitude` in WGS 84; no `gml:pos`, no other reference system.

**Standard.** `DefaultLocationSystem` is the frame's default `srsName`
(`netex_framework/netex_responsibility/netex_versionFrame_version.xsd`); the
axis order is the reference system's (EPSG:4326: latitude, longitude).

**The importer.** Reads `Latitude`/`Longitude` only; service link geometries are
not read (the courses come from the GTFS shapes).

**Proposal: adopt a per-link axis check** the day netex-rs reads geometries:
read a link's first position both ways and take the reading that lies near the
stop point it starts at. Reprojection: **not needed** for our deliveries.
Effort **S**.

**Built in 0.3.0** (`geometry::axis_order`): where service links are read — the
routing profile does not read them —, the index judges each link's first
position against the stop point it starts at, its own position else its stop's,
and counts a geometry written longitude first (`Concern::AxisOrderSwapped`) and
one whose order cannot be told (`Concern::AxisOrderUndecided`); nothing is
swapped, the index holds no geometry. On the nine Austrian deliveries 1 391
links in Linz up to 35 383 in VOR, 116 910 in all, are written longitude first;
Kärnten has 15 links whose order cannot be told, all starting at a stop point
no document defines.

**Built in 0.5.0, behind the feature `crs`** (`netex_rs::crs`): the systems the
deliveries around ours write — CH1903+/LV95 and CH1903/LV03, MGI/Austria Lambert
and GK M31, Amersfoort/RD New, RGF93/Lambert-93, ETRS89/UTM 32N and 33N — turned
into WGS 84 in pure Rust: each projection, the EPSG Helmert parameters, and for
the old networks MGI and CH1903, which are distorted against any Helmert
transformation by up to a metre and a half, a polynomial fitted to the official
grids (`tools/crs_corrections.py`), without PROJ. A position states its system by
`srsName`, else its frame's `DefaultLocationSystem`; a system the crate does
not know is a finding (`Concern::UnknownReferenceSystem`), the position left
out. Against 356 positions of the Swiss and Austrian registers and of PROJ with
the national grids every system lands within a metre — 0.35 m at worst (MGI),
0.25 m (LV03), 0.18 m (RD), the rest within a millimetre —; at the registers'
stops held back from the fit within 0.61 m (MGI) and 0.74 m (LV03), where the
Helmert transformations alone are 1.37 m and 1.79 m off. Without the feature
the index reads positions as before.

## 14. What a nested element is, and what it inherits

**Problem.** A quay is written inside its stop place, a stop point in journey
pattern inside its pattern, a passing time inside its journey; frame defaults
(time zone, reference system, data source) apply to everything in the frame.
NeTEx lets many nested elements go without an identifier or a version. Ours do
not: every quay, level, point in a pattern, passing time, call and stop
assignment carries an identifier in the Linz AG delivery, the DELFI DB part and
the Swiss part (927 405 passing times in Linz, 2 252 775 in the DB part, 768 928
calls in the Swiss part, none without).

**Standard.** Identifiers are optional on many nested elements; frame defaults
apply to the frame's contents.

**The importer and netex-rs.** The typed model keeps nesting as it is written;
identifiers are what the document states; the time zone of the frame defaults is
read (by the importer: of the first document by name that names one).

**Proposal: not needed.** Our deliveries give every nested element its
identifier, and a reader that keeps nesting as written needs none invented.
Effort none.

## 15. Refuse or tolerate

**Problem.** Every question above has a cheap answer (drop it) and a costly one
(understand it); a reader that tolerates without saying so hides defects, one
that refuses breaks on real deliveries.

**The importer.** Every decision is a `Warning` with its count and the documents
and entities of its first occurrences; the read stops only when no timetable can
be built at all or a document cannot be read. On Linz its report names 15 kinds
of decision, from `journey on days its line version does not govern` to `quay
under its own identifier, its GlobalID no quay's or shared`.

**Proposal: adopt the policy object, not the list.** A dialect-independent
`Policy` in netex-rs naming each decision a reader takes, counted always,
fatal by choice — so that a build can refuse a delivery whose counts jump.
Effort **M**.

**Built in 0.3.0** for the index: every decision is a `Finding` of a `Concern`,
counted (`Index::finding_count`) and listed in a fixed order; `index::Policy`
tolerates each concern up to a count of the caller's and refuses the index
beyond it, naming every concern exceeded (`Refusal`).

## 16. Several deliveries into one

**Problem.** A timetable is built from several deliveries — the German national
one plus the Austrian Verbünde, the Austrian railway beside the Verbünde — that
overlap in stops (the border stations), in lines (a regional train in two
deliveries) and in runs.

**The journey planner** (its importer and its build). A delivery is the base,
others are added as guests: a stop of the guest is the base's where the base has
it under the same identifier, an operator a region's own source replaces is left
out of the base, the export of the same delivery read beside it pairs runs and
fills in what the NeTEx lacks.

**Proposal: keep the journey planner's.** A stop is paired by its identifier, a
run stated in two deliveries is the base's; nothing is decided by which delivery
happens to come first. Effort none.

**Built in netex-cli 0.1.0** (`netex merge`), **since netex-rs 0.7.0 the
library's** (`netex_rs::merge`, the command on it): the journey planner's rules,
each decision a finding. Stop places pair by identifier, else by `GlobalID` or
`SLOID`, each quay by its own identifier or global identifier; a stop place
whose every quay pairs is the earlier delivery's, its references pointed
there, a name or a position more than 50 m away that differs found; one with a
quay pairing with none is kept apart. Another place under an earlier
identifier and global identifier — a park and ride — is that place stated
otherwise. An object is left out as alike only where it is written alike,
refers to objects alike and is assigned alike (a day type with the same day
type assignments), so that a calendar the Linz AG reuses per document (§ 2) is
not merged into another's; an identifier taken with other content is renamed
into the delivery's codespace, `<label>:<identifier>`, with its references. A
journey calling at the same stations at the same times as one of an earlier
delivery, on days that one runs on too, is left out. Each delivery is written
under its rank and label, so that netex-rs's resolution in the document whose
name sorts first answers as the deliveries are ranked. ÖBB/EVU and VOR (45 s,
13.6 GB, every document held in memory): the 439 stop places they share — 413 paired, 26 kept apart because VOR states a quay the
ÖBB delivery lacks —, 1 764 stop places and quays paired by identifier and 566
quays by GlobalID under another identifier; 555 paired stops named otherwise
and 50 placed more than 50 m away — among the 1 829 of the same identifier 16
named otherwise (`at:46:3040:11` is "zug1a" in the ÖBB delivery, "Bahn" in
VOR's), the other 539 quays paired by GlobalID under another identifier and
named otherwise in the two (VOR's "Bahngasse 2", the ÖBB's "Bus") —; the 302
park and rides of both stated otherwise (2 128 statements); no journey in
both.

## 17. Lexical values

**Problem.** An enumeration value with spaces (`bus `), a time zone on a time, a
line break in an address.

**Standard.** XML Schema compares an enumeration derived from `xs:NMTOKEN` —
the transport mode — after collapsing its white space, and keeps the white space
of an `xs:string` as written (XML Schema Part 2, 4.3.6).

**netex-rs.** Keeps the lexical form of every value (`Time`, `Date`, `Duration`
are text; an enumeration is its token as written, white space included) and
round-trips it. The crate's own fragments (`tests/fixtures/fragments/`) pin it:
padded enumerations, zone offsets and fractions, hours of 24 and past, negative,
short and malformed durations, line breaks and non-ASCII letters in an address
(`tests/own_fragments.rs`).

**Proposal: keep.** Effort none.

## 18. A point the vehicle passes through

**Problem.** A journey pattern may list a stop point the vehicle passes without
stopping. The ÖBB/EVU delivery writes every halt a long-distance train passes as
a point of the pattern with `StopUse` `passthrough` — 122 452 of them. In the
example checked the journey gives such points no passing time, and the pattern
writes `ForBoarding` and `ForAlighting` `true` on them
(`at:obb:StopPointInJourneyPattern:03499M.j26-1-0010102:` in
`NX-PI_01_at_obb_LINE_obb_3-499-M-j26_20261008.xml`: the journey
`at:obb:ServiceJourney:03499M-OEBB-1-1-72060:TA:` has times at points 1 and 4
only). The Swiss delivery writes the same as calls with `StopUse`
`passthrough` — 53 817 in the part —, *with* times: the
`ch:1:ServiceJourney:ch:1:sjyid:100001:108-001_9100NY.j26_475` passes Zürich HB
from 09:24 to 09:58.

**Standard.** `StopUse` says how a point is used (`access`, `interchangeOnly`,
`passthrough`, `noBoardingOrAlighting`:
`netex_part_1/part1_tacticalPlanning/netex_servicePattern_support.xsd`);
boarding and alighting are flags of their own. The schema does not say which wins where they contradict.

**The importer.** Leaves a passed point out and counts the pattern it is in
(`call passed through without stopping, left out`: 7 214 patterns of the ÖBB/EVU
delivery), and a passed call of a Swiss journey likewise; the destination
display a passed point names is shown from the next call on.

**netex-rs.** The index keeps every point of the pattern and does not read
`StopUse`, boarding or alighting: a reader on the index sees a call at
`si:63:103:0:1` without a time.

**Proposal: keep the importer's reading; better in netex-rs:** carry `StopUse`,
boarding and alighting per point of a pattern into the index, and leave a passed
point out of a journey's calls there. Effort **S**.

**Built in 0.3.0.** A passed point — of a pattern or a journey's calls — is no
call and a finding (`Concern::PassedThrough`): 122 452 points in the ÖBB/EVU
delivery, 53 817 calls in the Swiss part. Each call carries whether one may
board and alight (`Journey::uses`), neither at a point used for neither.

## 19. A journey shorter than its pattern

**Problem.** A journey names a pattern and states passing times for only part
of it: `at:oov:ServiceJourney:1212001-WeLi-214-1-41040:Mo-Fr:` has 19 passing
times on the 22 points of its pattern (`NX-PI_01_at_oov_LINE_oov_12-1-j26_20261007.xml`),
the last at point 19, arrival 11:51 — a short run on the pattern of the whole
line. Journeys without a time at their first or last point: 90 in
Oberösterreich, 65 in Tirol, 33 in Kärnten, 28 in Steiermark, 16 in Salzburg.

**Standard.** A journey follows its pattern; passing times are stated at its
timing points. Whether points after the last passing time are served the
schema does not say.

**The importer.** Leaves the journey out (`journey without a time at its ends`,
the 232 journeys above): a deliberate rule — only intermediate times may be
open —, but it loses runs that are there. **This is a fault of ours.**

**netex-rs.** The index keeps every point, the last three without a time.

**Proposal: better.** A journey ends at its last passing time and
begins at its first: points of the pattern before and after are not called at,
and that is counted; a journey left without two timed calls is then left out.
Effort **S** (the importer, and the same in the netex-rs index).

**Built in the index in 0.3.0** (`Concern::UntimedEnds`,
`Concern::TooFewTimedCalls`): the journeys above call short of their pattern's
ends — 90 in Oberösterreich, 65 in Tirol, 33 in Kärnten, 28 in Steiermark, 16 in
Salzburg, the same counts as above. The importer still leaves them out.

## 20. On-demand services

**Problem.** The Austrian Verbünde publish their on-demand services — call
buses, shared taxis, shuttles — as `FlexibleLine`s whose journeys name no day
type but an availability condition, and a time demand type instead of passing
times: `at:ktn:ServiceJourney:Berg-Bad-Bus_504_TA_159_22500_72000:`
(`NX-PI_01_at_ktn_LINE_ktn_Berg-Bad-Bus_84_20261008.xml`), Vienna's
`Baustellen-ASTKatzelsdorf`, Salzburg's `EFA-Shuttle`. Journeys of flexible
lines: Vienna 660, Kärnten 402, Salzburg 170, Steiermark 37, Tirol 4 — in these
deliveries exactly the journeys dated only by an availability condition. Vienna
also writes 45 flexible stop places and 45 flexible areas.

**Standard.** A `FlexibleLine` is a line with booking arrangements
(`netex_part_1/part1_networkDescription/netex_flexibleNetwork_version.xsd`;
deprecated in 2.0 in favour of `Line` with flexible properties).

**The importer.** Leaves the journeys of flexible lines out and counts them
(`FlexibleLine not read`, the same 660, 402, 170, 37 and 4); the journey
planner plans on-demand services apart, from GTFS-Flex.

**netex-rs.** The index reads neither flexible lines nor availability
conditions: such journeys run on no day. Since 0.3.0 it leaves a journey of a
flexible line out and counts it (`Concern::FlexibleLineJourney`; the 402 of
Kärnten), as the importer does.

**Proposal: better** — read a flexible line as what it is, its area or stops,
its windows and booking, into the model of a planner of on-demand services,
not as fixed journeys: read as fixed journeys, an on-demand service would be
shown as a bus at fixed times. Effort **M**.

## 21. Who refers to an object

**Problem.** A reference resolves forwards: a journey names its pattern, the
pattern its stop points, a stop assignment the quay. A report on a delivery
needs the other direction — which journeys a stop assignment naming a quay
nothing defines affects, which day types nothing uses —, and so does cutting a
region out of a delivery. On the German delivery of 2026-10-12 the index
resolves 32 666 991 references between 11 452 389 objects.

**Standard.** Silent.

**The importer.** None; it reads forwards.

**Proposal: adopt** the adjacency over the index's objects, built only when
asked for, each reference as the index resolved it — in its own document first.
Effort **S**.

**Built in 0.5.0** (`IndexBuilder::with_referrers`, `Index::referrers`): the
objects the index keeps — stop places, quays, stop points, levels, lines,
journey patterns, journeys, day types, operating periods and days,
availability conditions — numbered per kind, every reference the index resolves
recorded as it resolves it, and a forward and an inverse adjacency in
compressed rows over them; an object is found by element, identifier and
document; `unreferenced` lists the definitions nothing refers to (a stop place
counts as used through its quays, a journey is never unreferenced). Without
`with_referrers` the index is built as before. On the German delivery, lean,
on 8 threads, alternately without and with (`examples/index_delivery
--referrers`, under the shared measuring lock): 110 s and 131 s at a load of
25 to 16, 124 s and 153 s at 83 to 97; peak memory 11.95 GB and 12.60 GB both
times — 0.65 GB for the 32.7 million references, about 20 bytes each —, the
index's fingerprint the same. `netex validate` reads backwards with it: the
journeys a finding affects, the definitions nothing uses (147 285 on that
delivery).

## 22. A line named through a route

**Problem.** A journey pattern states its route by reference or as a view
(`RouteRef` or `RouteView`), and a view may name the line; the route names its
line too. The South Tyrolean delivery names the line in a route view for
11 625 of its 12 256 patterns; 631 name only a route
(`IT:ITH1:ServiceJourneyPattern:01110_.25a1200830:` →
`IT:ITH1:Route:1-110-25a-12-8/R:` → line `IT:ITH1:Line:01110_.25a:` version 12),
and 929 of its 54 939 journeys follow those, naming no line themselves. Read by
route views alone, those 929 journeys run in no line — a converter to GTFS has
no route for them — and on every day of their day types, though the line
versions their routes name govern only some of those days (§ 4).

**Standard.** A journey pattern names its route by `RouteRef` or `RouteView`,
at most one of them (`netex_part_1/part1_tacticalPlanning/netex_journeyPattern_version.xsd`,
`JourneyPatternProperties`); a route view, an annotated reference to a route,
may hold the route's reference and the line's, and a route names its line
(`netex_part_1/part1_networkDescription/netex_route_version.xsd`, `RouteView`,
`Route`); a vehicle journey may name its route and its line
(`netex_part_2/part2_journeyTimes/netex_vehicleJourney_version.xsd`). Which
comes first where they disagree the schema does not say.

**The importer.** Reads a pattern's `RouteRef` and does not follow it: a
pattern's line is the one its route view names.

**Proposal: adopt** the route between a pattern or journey and its line, the
line stated nearest the journey first. Effort **S**.

**Built in 0.6.0** (the index): routes are a document's own, as lines are
(§ 2): a reference to one resolves in its own document first, and a route
another document states with another line is a finding. A journey runs in the
line it names, else in the line of the route it names, else in its pattern's:
the line the pattern's route view names, else the line of the route the
pattern names, by reference or in its view. The route's `LineRef` is a
reference like any other — unresolved where no document defines the line —,
and the graph of references (§ 21) holds routes, referred to by the patterns
and journeys naming them. On the South Tyrolean delivery the 929 journeys run in
their lines; 871 of them are left out, none of their days being one their line
version governs (`Concern::JourneyOutsideLineVersion`) — the routes these
patterns name are mostly of short versions, 272 of the 600 of a version valid
on one day —, and 58 run on the days their version governs. Checked against a
reading of the document of its own (`IT-ITH1-PROFILE.md`).

---

## Summary

| § | Question | Proposal | Effort | Built (0.3.0 unless named) |
|---|---|---|---|---|
| 1 | References that do not match | keep; count what resolved only by ignoring version or class | S | counted |
| 2 | Identifiers per document | better: a resolution scope in netex-rs, a redefinition a finding — **fixes a netex-rs defect** | M | ✓ |
| 3 | Operating days | keep the importer's rules, move them into netex-rs; write UIC bits for EPIP | M | ✓ (`calendar`) |
| 4 | Line versions | keep, move into netex-rs | S | ✓ |
| 5 | Times after midnight | not needed; a check for a time stated both ways | S | – |
| 6 | Calls | better: type `Call` in netex-rs | M | ✓ |
| 7 | Interchanges | keep; move a point off its journey's pattern only for a delivery that needs it | – | – |
| 8 | Levels | not needed for reading | – | – |
| 9 | Train numbers | not needed | – | – |
| 10 | Who is responsible for a line | not needed for reading (an authority per line for an EPIP writer) | – | – |
| 11 | Stop points and stop places | keep the merge; count near-duplicate stop places | S | – |
| 12 | Runs stated twice | keep | – | – |
| 13 | Axis order, reference systems | adopt a per-link check when geometries are read; reproject the national systems | S | ✓ (a finding; `crs` in 0.5.0) |
| 14 | Nested elements | not needed | – | – |
| 15 | Refuse or tolerate | adopt a policy object | M | ✓ (`Policy`) |
| 16 | Several deliveries | keep the journey planner's rules | – | ✓ (`netex merge`) |
| 17 | Lexical values | keep | – | – |
| 18 | Passed points | keep the importer's reading; netex-rs's index carries `StopUse` and the boarding flags | S | ✓ |
| 19 | A journey shorter than its pattern | better: end it at its last passing time — **fixes a fault of the importer** | S | ✓ (index) |
| 20 | On-demand services | better: flexible lines into a planner of on-demand services | M | left out, counted |
| 21 | Who refers to an object | adopt the reverse adjacency, only when asked for | S | ✓ (0.5.0) |
| 22 | A line named through a route | adopt: the line stated nearest the journey first | S | ✓ (0.6.0) |
