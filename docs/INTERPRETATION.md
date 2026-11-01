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
  `docs/IT-ITH1-PROFILE.md`); and five deliveries in the French profile from
  the French national access point, fetched 2026-10-31: Fluo Grand Est in the
  Bas-Rhin, Naolib (Nantes), TBM (Bordeaux), the aggregate of Normandy's
  networks and SNCF's trains (`docs/FR-PROFILE.md`); and Norway's national
  delivery in the Nordic NeTEx Profile, Entur's of 2026-10-31
  (`rb_norway-aggregated-netex.zip`, with its stop register;
  `docs/NORDIC-PROFILE.md`); and the deliveries of four neighbours in the
  European Passenger Information Profile (`docs/EPIP-NEIGHBOURS.md`):
  Luxembourg's of 2026-10-21, Denmark's of 2026-10-30, the Belgian
  operators' of 2026-10-22 (SNCB) and 2026-10-31 (De Lijn, TEC, STIB) and the
  Czech national ones of 2026-10-30.
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

**Built in 0.8.0**, as far as § 29: a flexible line is a line
of the index; a journey of one is left out and counted, whether it names the
line or reaches it through its route or pattern.

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

## 23. Frames that hold everything

**Problem.** The French profile writes every part of a delivery — stops,
network, calendar, timetable — in `GeneralFrame`s, naming the part by the
frame's `TypeOfFrameRef` (`FR:TypeOfFrame:NETEX_ARRET`, `…NETEX_HORAIRE`, …).
Fluo Grand Est in the Bas-Rhin states 103 general frames in its 53 documents,
Naolib 198, Normandy's aggregate 1 276; each holds its entities side by side in
`members`.

**Standard.** A general frame is "a General purpose frame that can be used to
exchange any NeTEx element. Does not impose any structure."
(`netex_framework/netex_frames/netex_generalFrame_version.xsd`, `GeneralFrame`);
its `members` hold the entities. The profile chooses it for every part, typed by
the part ("Éléments communs", on version frames: "un GeneralFrame qui peut
contenir n'importe quel type d'ENTITY IN VERSION").

**The importer.** Not run on a French delivery.

**netex-rs.** Up to 0.7.2 a general frame was carried through untyped, as
anything the model does not name: 99.66 % of Fluo's elements, 99.99 % of
Naolib's and 99.91 % of Normandy's, and the index held no stop, line or journey
of them.

**Proposal: adopt.** Effort **S**.

**Built in 0.8.0.** `GeneralFrame` is typed, its `members` hold
every kind of entity the specific frames hold that the model types, and a quay
standing on its own (§ 24); the lean stream reads them in document order
(`FrameKind::General`), and the index takes each as it takes the same entity in a
specific frame. Untyped after the read: 7.23 % of Fluo's elements, 0.76 % of
Naolib's, 3.51 % of Normandy's — postal addresses, kinds of place,
suitabilities, route links, networks (`docs/FR-PROFILE.md`).

## 24. Quays defined beside their stop place

**Problem.** The French profile defines a quay (*zone d'embarquement*) on its
own, among the members of the stops' frame, and the stop place names it among
its quays by reference: "le profil fait le choix de définir les ZONEs
D'EMBARQUEMENT indépendamment et de les référencer" ("Description des arrêts",
`StopPlace`, `quays`). The quay names its stop place by its `SiteRef` as well.
Fluo states 1 035 quays so, Naolib 2 939, Normandy's aggregate 12 604; each is
named by exactly one stop place and names the same one by its `SiteRef`.

**Standard.** A stop place's `quays` hold a `Quay` or a `QuayRef`
(`netex_part_1/part1_ifopt/netex_ifopt_stopPlace_version.xsd`,
`quays_RelStructure`); a quay's `SiteRef` names the site it is part of. Which
wins where the two disagree the schema does not say.

**The importer.** Not run on a French delivery.

**netex-rs.** Up to 0.7.2 a `QuayRef` among a stop place's quays was carried
through, and a quay outside a stop place was not read.

**Proposal: adopt** a quay's stop place from both references. Effort **S**.

**Built in 0.8.0.** A quay standing on its own is an entity of
its own (`stream::Kind::Quay`), a quay of the stop place naming it among its
quays, else of the one its `SiteRef` names; each reference resolves as any other,
a `QuayRef` to nothing is a reference to nothing. A quay two stop places claim
stays with the first and is a finding (`Concern::QuayOfTwoStopPlaces`; none in
the five deliveries). The stop place naming its quay and the quay naming its
stop place are references of the graph (§ 21), but no use of either: a stop
nothing places a stop point at is still listed as unreferenced.

## 25. Passing times naming no point

**Problem.** Fluo, Naolib and Normandy write a journey's passing times without
the point of the pattern each is at — 1 771, 124 908 and 85 356 journeys, every
one of theirs —, in the order of the pattern's points: each journey has as many
passing times as its pattern has points. TBM and SNCF name the point.

**Standard.** In NeTEx 1.x a passing time's `PointInJourneyPatternRef` is
optional (`netex_part_2/part2_journeyTimes/netex_passingTimes_version.xsd`,
`PassingTimeGroup`); 2.0 requires it, and an identifier of the passing time.
The French profile writes 1.x and keeps the reference optional ("Horaires",
`PassingTime`). That passing times then stand in the order of the points the
schema does not say; it is the only reading under which these deliveries are
timetables.

**The importer.** Not run on a French delivery.

**netex-rs.** Up to 0.7.2 a passing time naming no point was left out, and a
journey with none left was left out with it.

**Proposal: adopt** the reading by position. Effort **S**.

**Built in 0.8.0.** A passing time naming no point is at the
point of its position, in the order the pattern states its points; one naming a
point is read as before. Times other in number than the points are read as far
as both go, and are a finding (`Concern::PassingTimesOtherThanPoints`; none in
the five deliveries).

## 26. A day type's properties

**Problem.** Three forms of day type in the French deliveries:

- a validity, days of the week (`PropertyOfDay/DaysOfWeek`) and an assignment
  of each day it runs on: Fluo's 30 day types, every one of Normandy's — the
  properties summarise the days, they do not decide them: Fluo's school days are
  Monday to Friday from 2 November 2026 to 2 July 2027 by their properties,
  without the holidays by their dates;
- a validity and days of the week, the assignments its exceptions — days taken
  away (`isAvailable` false), days added on other days of the week: TBM's
  (Mecatran's export), 147 of its 283 day types with no assignment at all, 72
  with removals only, 62 adding a day on another day of the week;
- days of the week and an assignment to an operating period without day bits:
  Naolib's 139 day types, the period giving its days only on those days of the
  week.

Read by their assignments alone, 32 331 of TBM's 39 749 journeys run on no day,
and a Naolib journey on every day of its periods, whatever its days of the
week.

**Standard.** A property of day names what makes a day one of the type, a
day type assignment gives the type days (`netex_framework/netex_reusableComponents/netex_dayType_version.xsd`,
`netex_serviceCalendar_version.xsd`). The profile makes the validity a limit
of the day type ("si le TYPE DE JOUR n'est valable que pour une période de
temps limitée, on le précisera grâce au ValidBetween"), the properties of day
alternatives ("sous un même PropertyOfDay les caractéristiques s'associent par
un ET, sinon … par un OU"), the assignments exceptions where the properties do
not suffice ("Éléments communs", day types). How a day type's properties and its
assignments combine the schema does not say.

**The importer.** Not run on a French delivery; it gives day types the days of
their assignments (§ 3).

**netex-rs.** Up to 0.7.2 the index read assignments alone (§ 3).

**Proposal: adopt**, as a rule every reader of the crate applies. Effort **S**.

**Built in 0.8.0** (`calendar`, the index). Properties are read
where they name days of the week and nothing else: an assignment to a period
without day bits gives the period's days on those days of the week; a day type
with a validity of a first and a last day, none of whose assignments gives it a
day of its own — a period, or a date within its validity on one of its days of
the week —, runs on the days of its validity on its days of the week, its
assignments adding and taking away in their order. A day type whose assignments
do give it such a day runs on their days alone, as before — every day type an
available period is assigned to among them, UIC day bits or not. The two French forms are told apart by that alone:
each of Fluo's and Normandy's day types adds a day of its own, none of TBM's
does. Properties naming more than days of the week (holidays, seasons) are not
read; where they would decide the days, that is a finding
(`Concern::PropertiesOfDayUnread`). Checked against the GTFS feeds the same
producers publish: TBM's 555 768 journey days and Naolib's 682 559, each
journey's days and stop times one for one; Fluo's 209 362 within the period of
its NeTEx (`docs/FR-PROFILE.md`).

## 27. Walks stating a distance alone

**Problem.** Fluo states 826 walks between quays (`SiteConnection`), Normandy's
aggregate 16 600; all of Fluo's and 16 565 of Normandy's state a distance and
no duration.

**Standard.** A site connection states its `Distance` and its durations
(`WalkTransferDuration`, `TransferDuration`) independently
(`netex_part_1/part1_tacticalPlanning/netex_siteConnection_version.xsd`);
neither is required.

**The importer.** Not run on a French delivery.

**netex-rs.** The index holds a walk by the time it takes and left a walk
without a duration out without a word.

**Proposal: count it**; turning a distance into a time needs a walking speed,
which is the reader's choice, not the delivery's. Effort **S**.

**Built in 0.8.0.** A walk without a duration is a finding
(`Concern::WalkWithoutDuration`), and still no transfer of the index.

## 28. Journeys dated by dated journeys

**Problem.** Norway's delivery gives 106 259 journeys no day type: 2 158 080
`DatedServiceJourney`s each name a journey (`ServiceJourneyRef`) and an
operating day (`OperatingDayRef`) and state nothing else — `RUT` in 391
documents, `VYG`, `SJN` and 14 more codespaces. 3 359 of them are cancelled and
6 681 replaced (`ServiceAlteration`); a journey running on a day a dated
journey of it is replaced on has a second, plain dated journey for that day
(`VYG:ServiceJourney:384-114_841657-R`, 2027-01-04). Read as journeys of their
own, the dated journeys have no pattern and no times, and the journeys they
date no day.

**Standard.** A `DatedServiceJourney` is "a particular journey of a vehicle on
a particular operating day", and may refer to the journey it is a run of
(`netex_part_2/part2_journeyTimes/netex_datedVehicleJourney_version.xsd`, the
`JourneyRef` of `DatedVehicleJourneyReferencesGroup`, NeTEx 2.0; NeTEx 1.x
gives a dated journey no reference to its journey). The Nordic profile writes
every dated journey so, and names the dated journey a replacing one replaces
by a `DatedServiceJourneyRef` (NeTEx 2.0: `replacedJourneys`).

**The importer.** Not run on a Nordic delivery.

**netex-rs.** Up to 0.7.2 the index read a dated journey as a journey of its
own: 2 158 080 journeys left with fewer than two timed calls, 2 266 214
without a day.

**Proposal: adopt.** Effort **S**.

**Built in 0.8.0** (the index). A dated journey naming a
service journey and an operating day and stating no pattern, passing times
or calls of its own dates the journey it names: the journey runs on the
operating day, beside the days of its day types. A cancelled or replaced
dated journey gives no day and takes none away — the plain dated journey of
the same day gives it, where there is one. Any other dated journey is a
journey of its own, as before. Entur's GTFS of the same delivery has the same
days for every journey a dated journey dates (`docs/NORDIC-PROFILE.md`).

## 29. A flexible line named by a route

**Problem.** Norway's delivery defines 698 `FlexibleLine`s among its service
frames' lines; their routes name them by `FlexibleLineRef`, and of their 2 390
journeys 1 232 name neither line nor route, only their pattern. With flexible
lines untyped, 824 references to them resolved to nothing, and the journeys
reaching them through their pattern were read as fixed journeys — left out
only where they stated no fixed times (1 191), kept where they did (41).

**Standard.** A `FlexibleLine` is a `Line` with booking arrangements (§ 20).

**The importer.** Not run on a Nordic delivery.

**netex-rs.** Up to 0.7.2 the index read neither flexible lines nor the
`FlexibleLineRef` of a journey, which left the journey out (§ 20).

**Proposal: adopt.** Effort **S**.

**Built in 0.8.0.** A flexible line is a line of the index,
its `FlexibleLineType` typed and its booking carried through; a reference to
it resolves as any reference to a line. A journey whose line is flexible — the
one it names, its route's or its pattern's — is left out and counted
(`Concern::FlexibleLineJourney`): 2 390 in Norway. The Austrian deliveries gain
their flexible lines as lines and leave out the same journeys as before.

## 30. Operating periods stated by their operating days

**Problem.** 383 operating periods of 25 Norwegian codespaces state their
first and last day as operating days (`FromOperatingDayRef`,
`ToOperatingDayRef`): read by their dates alone, they have no first day, and
1 539 journeys run on no day.

**Standard.** An operating period starts on an operating day or a date and
ends on one (`netex_framework/netex_reusableComponents/netex_serviceCalendar_version.xsd`,
`OperatingPeriod`, a choice of each).

**netex-rs.** Up to 0.7.2 the index read the dates only
(`Concern::PeriodWithoutFirstDay`).

**Proposal: adopt.** Effort **S**.

**Built in 0.8.0.** A period stated by operating days takes
their dates, each reference resolved in the period's document first; a
reference to nothing is a finding, and leaves the period without that day.

## 31. A condition no journey names, to the year 9999

**Problem.** Flytoget's documents state the validity of their frames as an
availability condition from 2026-10-28 to 9999-12-31. The index gave every
calendar a bit for each day from the first to the last day of any period or
condition of the delivery: in Norway's, 2.9 million days, 360 kB a calendar;
asking for the days of each journey took five minutes.

**Standard.** Silent on how far a reader looks.

**netex-rs.** Up to 0.7.2 the calendar spanned every period and condition.

**Proposal: better.** Effort **S**.

**Built in 0.8.0.** The calendar spans the days that can give
a journey a day: periods an assignment names, conditions a journey names,
operating days, assigned dates and the validities of day types their days of
the week decide. A period or condition nothing names gives no journey a day
however far it reaches; no journey's days change (the German, Austrian and
French deliveries read the same before and after, journey for journey).

## 32. Stops a delivery names and does not define

**Problem.** The Nordic deliveries assign their stop points to quays of the
national stop register and define no site of their own. Norway's national
archive holds the register; the archives of each codespace do not, and the
register is published apart.

**Standard.** A reference may name what another publication defines; how a
reader finds it the schema does not say.

**netex-rs.** Up to 0.7.2 a reader gave the index the documents of one
archive or directory.

**Proposal: adopt.** Effort **S**.

**Built in 0.8.0** (`dataset::Source::with`, `--with` of the
`netex` command). Several sources are read as one: a delivery with its
register, the documents of each source after the first named under its file
or directory name, so that no two documents share a name. The register's
stops are stops of the delivery, merged as the stops of any two documents are.

## 33. Bounds of an operating period written as instants

**Problem.** The Belgian deliveries write the first and last day of every
UIC operating period as an instant in UTC: `2026-10-21T22:00:00Z` for the
period beginning 22 October, `2026-10-29T23:00:00Z` for the one ending
30 October — the midnights beginning those days in Brussels, in summer and
in winter time. Their day bits count from the local day. Read by the date
of the instant, every period began a day early: 74 677 of STIB's 76 043
journeys and 199 557 of TEC's 203 955 ran on other days than the GTFS feeds
the operators publish beside the NeTEx give them.

**Standard.** `FromDate` and `ToDate` of an operating period are
`xsd:dateTime` (`netex_framework/netex_reusableComponents/netex_serviceCalendar_version.xsd`);
a date and time with a zone is an instant. The schema does not say which
day an instant names; the deliveries name no time zone
(`DefaultLocale`) to turn it into a local day.

**netex-rs.** Up to 0.7.2 the day of a bound was the date written.

**Proposal: adopt.** Effort **S**.

**Built in 0.8.0** (`calendar::bound_day`). A bound written
with a zone (`Z`, `+01:00`) at noon or later names the next day: the local
midnight it is lies within twelve hours of UTC — every European zone — and
is nearest the UTC midnight of its own day. A bound without a zone, or
before noon, is the day of its date. Only the Belgian deliveries write such
bounds of all those read; STIB's and TEC's journeys now run, trip for trip,
on the days of their GTFS feeds. De Lijn and SNCB state every day by a
dated assignment and no day bit, and read the same as before.

## 34. Stop identifiers each document numbers anew

**Problem.** The Czech deliveries of buses and of city transport are
converted batch by batch from the national timetable format, each batch
numbering its stop places from one: `CZ:cisjr_jdf:StopPlace:1` is another
stop in each of the 13 347 bus documents, under 2 441 names, and no stop
place states a position. A document of a line on a detour states its stop points
under the identifiers of the regular timetable's, named otherwise (17 121
stop points of the bus delivery). Gathered once by identifier, the bus
delivery had 82 stops: every journey called at the stop places of the
document named first. 513 994 of the 514 212 journeys the index holds now
called at stops of other names before.

**Standard.** An identifier is unique within its codespace; the schema's
keys check it within one document only.

**netex-rs.** Up to 0.7.2 a stop place, quay or stop point was gathered once
by identifier and version; another document's other content was a finding
(`Concern::RedefinedOtherwise`), and the definition of the document named
first was kept.

**Proposal: adopt.** Effort **S**.

**Built in 0.8.0.** A stop place, quay or stop point is
gathered by identifier, version and name: definitions naming it alike are
one stop, as before; a definition naming it otherwise is a stop of its own
(`Concern::LocalIdentifier`, naming the first document stating that name
and the document whose stop the identifier alone names, `Index::stop_by_id`).
A reference in a document defining the identifier resolves to that
document's stop; elsewhere to the one of the document named first. The
German delivery and eight of the nine Austrian ones repeat their stops under
one name and read as before; the ninth (VOR) names two quays otherwise in
another document (`Bahn` and `Zug`, `Bus` and `Bus + SEV`), now two quays
each. The Czech bus delivery has 98 129 stops and 139 087 stop points.
Without a position, a stop is known by its name alone: a reader who needs
places has to find them by name.

## 35. A stop assignment naming a quay by `StopPlaceRef`

**Problem.** The Belgian deliveries assign every stop point to a quay
through a `StopPlaceRef` (`<StopPlaceRef ref="gs:stibmivb:1306F"/>`, a quay
of `stops.xml`): 10 253 assignments of STIB's, 179 487 of TEC's and 924 146
of SNCB's.

**Standard.** A passenger stop assignment names a stop place by
`StopPlaceRef` and a quay by `QuayRef`
(`netex_part_1/part1_tacticalPlanning/netex_stopAssignment_version.xsd`).

**netex-rs.** Up to 0.7.2 the reference resolved to the quay, which is what
the delivery means, and nothing was said.

**Proposal: keep, and count.** Effort **S**.

**Built in 0.8.0.** The stop point is at the stop the
reference names, as before; a `StopPlaceRef` naming a quay, or a `QuayRef`
naming a stop place, is a finding that it resolved to another class
(`Concern::ResolvedAsOtherClass`, § 1), as a `QuayRef` naming a stop place
among a stop place's quays was already.

---

## 36. Passing times from a departure and a time demand type

**Problem.** No journey of the Dutch deliveries lists its passing times.
Each states its departure (`DepartureTime`, `DepartureDayOffset`) and names
a `TimeDemandType`, which states how long its journeys take on each timing
link (`runTimes/JourneyRunTime`) and wait at each timing point
(`waitTimes/JourneyWaitTime`); the journey's pattern names, at each of its
points, the timing link onward (`OnwardTimingLinkRef`), and passes timing
points that are no stop (`TimingPointInJourneyPattern`). The calendar is an
availability condition in the timetable frame's `contentValidityConditions`.
Read as before, all 294 754 journeys of six operators' deliveries were left
out: no passing time, and no day.

**Standard.** A vehicle journey states its passing times, or its departure
and a time demand type from which they follow
(`netex_part_2/part2_journeyTimes/netex_vehicleJourney_version.xsd`:
`DepartureTime`, `TimeDemandTypeRef`; `netex_part_1/part1_tacticalPlanning/netex_timeDemandType_version.xsd`).
`contentValidityConditions` holds the validity conditions a frame's content
shares (`netex_framework/netex_responsibility/netex_versionFrame_version.xsd`).

**netex-rs.** Up to 0.7.2 the model typed neither time demand types nor
timing points in a pattern, and no frame's `contentValidityConditions`.

**Proposal: adopt.** Effort **M**.

**Built in 0.8.0.** The model types them (`TimeDemandType`,
`JourneyRunTime`, `JourneyWaitTime`, `TimingPointInJourneyPattern`, every
frame's `content_validity_conditions`), the lean stream yields time demand
types (`Kind::TimeDemandType`, in `Selection::routing`) and the conditions of
`contentValidityConditions`. A journey listing neither passing times nor
calls but naming a time demand type, resolved in its own document first,
leaves the first point of its pattern at its departure and runs each timing
link onward in the type's run time; at every later point it arrives and
waits as long as the type says, else as long as the pattern's point says
(`WaitTime`), before it leaves — but at the last, where it arrives. A wait
the type states at the first point does not hold the departure back: the
departure is when the journey leaves (one journey of the 294 754 has such a
wait; GTFS-NL takes the departure for its arrival there). A point naming no
timing link onward, or a link the type gives no run time, leaves the calls
after it untimed (`Concern::LinkWithoutRunTime`, then `UntimedEnds`). Checked
against GTFS-NL day by day (`docs/NL-BISON-PROFILE.md`).

## 37. Run times to a tenth of a second

**Problem.** GVB states run times to the tenth of a second (`PT1M3.4S`,
`PT14.9S`). Read in whole seconds, the fractions were lost, and a journey's
times ran up to several seconds early by its end.

**Standard.** `RunTime` and `WaitTime` are `xsd:duration`, whose seconds may
have a fraction.

**netex-rs.** Up to 0.7.2 `Duration::seconds` read whole seconds.

**Proposal: adopt.** Effort **S**.

**Built in 0.8.0** (`Duration::milliseconds`). The times of
a journey add up to the millisecond, and each passing time is rounded to the
nearest second, a half up. GVB's 420 798 trip-days of the comparison window
are, to the second, GTFS-NL's in 92 %, and within one second in all others:
where the exact sum falls on a half second, GTFS-NL rounds down in some
places and up in others.

## 38. A link or a point a pattern passes twice

**Problem.** A pattern may run a link twice — RET's line 612 loops through a
town and back, on the same links —, and its time demand type then states a
run time for each pass, in the order of the pattern (30 s and 46 s). A
pattern may visit a point twice too — the start and end of a loop, a timing
point passed on the way out and at the end —, while its type states one wait
for it. RET's line 29 passes the timing point HA7121 in the middle of its
route and ends there; its type states a wait of 35 minutes there, which the
identifiers of its waits number as the last point's.

**Standard.** A run time names its timing link, a wait time its timing point;
neither names the pass.

**netex-rs.** New.

**Proposal: adopt.** Effort **S**.

**Built in 0.8.0.** The n-th pass of a link takes the n-th
run time stated for it, else the last stated: a type stating one run time
for a link a pattern runs twice means it for both. The waits stated for a
point are those of its last visits, in order: one wait for a point visited
twice is the layover at its end. RET's trips of line 29 wait at their end
rather than 35 minutes in their middle. GTFS-NL gives line 1602's second
pass the run times of the first (98 trip-days); the delivery states its own.

## 39. A journey naming only conditions not available

**Problem.** Connexxion writes a day a journey does not run as the profile's
own example does (TMI9 *Dienstregeling* 9.3.0, § 9.3): the journey's
condition without the day, a condition for the day not available beside it,
and a journey naming only that condition — 1 544 of them —, there to show
the validity the journey no longer has. Each also names the weekday day
type. Read with its day types' days less the condition's, each ran on every
other weekday, beside the journey it was written for: 14 433 trip-days a
journey stated twice or three times.

**Standard.** An availability condition not available (`IsAvailable` false)
states days the entity is not valid on. The Dutch profile: such a condition
gives a journey no validity (§§ 9.1.2, 9.2), and the day types of its
calendar frame are for information, the day bits leading where they differ
(§ 3.2, rule 7).

**netex-rs.** Up to 0.7.2 a journey ran on its day types' days, within the
days of the conditions available it names, less the days of those not
available.

**Proposal: adopt.** Effort **S**.

**Built in 0.8.0.** A journey naming conditions none of
which is available runs on no day, whatever its day types say
(`Concern::JourneyWithoutDays`). Where it names one available, nothing
changed. Every delivery of the other profiles the crate is checked on gives
the same index as before. The day types of the three Dutch deliveries naming them give every
journey each day of its conditions, so the rule of § 3.2.7 — the day bits
lead — holds for them as read; a delivery whose day types gave fewer days
would keep the fewer.

## 40. A stop known by the register's code

**Problem.** The German delivery names the Dutch stops it serves by
`GlobalID` `NL:Q:66150540` and `NL:S:66150540`; the Dutch stop register
names the same stops `NL:CHB:Quay:66150540` and states those keys as private
codes (`privateCodes/PrivateCode type="QuayCode"`, `StopPlaceCode`). By
identifier, the two are other stops.

**Standard.** `privateCodes` (NeTEx 2.0) are codes an entity is known by in
other systems, each with its type; `keyList` holds keys of any meaning.

**netex-rs.** Up to 0.7.2 a stop's global identifier was its `GlobalID` key,
and the model did not type `privateCodes`.

**Proposal: adopt.** Effort **S**.

**Built in 0.8.0.** The model types `privateCodes` beside
`keyList` (`PrivateCodes`), but on a passing time and a stop point in a pattern,
where it stays untyped. A stop place's or quay's global
identifier (`Stop::dhid`, `Index::stop_by_dhid`) is its `GlobalID`, else its
`StopPlaceCode` or `QuayCode`; `netex convert --stop-ids global` writes it,
and `netex merge` pairs a stop of a later delivery with an earlier one by it,
as by `GlobalID` and `SLOID`.

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
| 23 | Frames that hold everything | adopt: general frames typed, read as the specific frames | S | ✓ (0.8.0) |
| 24 | Quays beside their stop place | adopt: the stop place naming the quay, else the quay's `SiteRef` | S | ✓ (0.8.0) |
| 25 | Passing times naming no point | adopt: by position in the pattern | S | ✓ (0.8.0) |
| 26 | A day type's properties | adopt: days of the week within the validity where no assignment gives a day of its own; plain periods by days of the week | S | ✓ (0.8.0) |
| 27 | Walks stating a distance alone | count them; no walking speed chosen | S | counted (0.8.0) |
| 28 | Journeys dated by dated journeys | adopt: a dated journey stating no journey dates the one it names; cancelled or replaced, no day | S | ✓ (0.8.0) |
| 29 | A flexible line named by a route | adopt: a flexible line is a line; its journeys, however they reach it, left out | S | ✓ (0.8.0) |
| 30 | Periods stated by operating days | adopt: their dates | S | ✓ (0.8.0) |
| 31 | A condition no journey names | better: the calendar spans what can give a day | S | ✓ (0.8.0) |
| 32 | Stops a delivery does not define | adopt: several sources read as one | S | ✓ (0.8.0) |
| 33 | Bounds of a period written as instants | adopt: from noon on, the next day | S | ✓ (0.8.0) |
| 34 | Stop identifiers each document numbers anew | adopt: a stop per name, found in its own document | S | ✓ (0.8.0) |
| 35 | An assignment naming a quay by `StopPlaceRef` | keep, and count | S | counted (0.8.0) |
| 36 | Passing times from a departure and a time demand type | adopt: from the departure along the timing links, run and wait times of the type | M | ✓ (0.8.0) |
| 37 | Run times to a tenth of a second | adopt: add up to the millisecond, round each time to the second | S | ✓ (0.8.0) |
| 38 | A link or a point passed twice | adopt: each pass its own run time in order; the waits of a point its last visits' | S | ✓ (0.8.0) |
| 39 | A journey naming only conditions not available | adopt: no day | S | ✓ (0.8.0) |
| 40 | A stop known by the register's code | adopt: the code a global identifier where no `GlobalID` | S | ✓ (0.8.0) |
