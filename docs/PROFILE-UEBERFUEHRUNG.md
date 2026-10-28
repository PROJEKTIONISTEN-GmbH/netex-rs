# From one NeTEx profile into another

The deliveries we read are written in four profiles: the German national one
(DELFI, `NX-PI-01_DE_NAP_LINE`), the Austrian Verbünde's, the Swiss one, and the
European passenger information profile EPIP (CEN/TS 16614-4), which the other
three claim to follow and none writes as it is. This document says what turning
a delivery of one into another has to decide — into EPIP, and into a timetable
as the importer of a journey planner reads every profile — and what is gained
and lost on the way, counted on our deliveries.

The questions behind each step, with what the standard says and what we propose,
are in [`INTERPRETATION.md`](INTERPRETATION.md); this document names them by
their section there (§ 1 to § 21). Figures are for the deliveries named there.

## The four profiles side by side

| | DELFI (DE) | Austria | Switzerland | EPIP |
|---|---|---|---|---|
| Documents | one per line, every frame a line needs in it (28 286 in the delivery of 2026-10-12) | one per line, likewise (164 to 952 per Verbund) | five shared documents (resources, sites, service, calendar, common) and 515 timetable documents naming them | the profile prescribes frames, not documents |
| Identifiers | unique in the delivery but for some stop points (155 in the GVH/BSAG part) | the Linz AG reuses calendar and journey identifiers per document with other contents (§ 2) | unique; journeys `ch:1:sjyid:…` | unique |
| Stop places | repeated in every line document, each copy with the quays that line uses; GVH writes none (§ 11) | likewise; quays named by their DHID, areas written as quays | in the site document, once | once, with every quay |
| Stop points | assigned to stop place and quay, or to the stop place alone; some to nothing | assigned to stop place and quay; some to nothing (Kärntner Linien 4 118) | assigned, a third to the stop place alone; none with a location of its own (68 794 in the part) | every one with a location |
| Journeys | pattern + passing times, day offsets | pattern + passing times, day offsets | calls with day offsets; template journeys at a headway (§ 6) | pattern + passing times |
| Days | day type → assignment → UIC period | day type → assignment → UIC period; the line's version limits them (§ 4) | availability condition per journey (bits) | day type → assignment → UIC period |
| Lines | authority, no operator | operator, no authority; in versions | operator, no authority; with validity | one authority; operators as additional operators |
| Interchanges | stay-seated `ServiceJourneyInterchange` in the distributor's document | `JourneyMeeting`, `InterchangeRule` | `JourneyMeeting`, `InterchangeRule` in the common document | `ServiceJourneyInterchange` |
| Train numbers | in the journey's identifier | `TrainNumber` (ÖBB/EVU: 22 693) | `TrainNumber` on every journey | `TrainNumber` |
| Geometry | service links without geometry (13 517 in the GVH/BSAG part) | service links with geometry, longitude first (§ 13) | none | service links, latitude first |
| Time zone | frame defaults, `Europe/Berlin` | none stated | frame defaults, offset +1 / summer +2 | — |

## Into EPIP

A writer of EPIP closes, for every delivery, the differences the table shows:

1. Every line gets one authority: the codespace's one authority, else one made
   from the line's operator, with the address EPIP requires; the operator goes
   among the line's additional operators (§ 10).
2. Every calendar becomes a day type, one assignment and one UIC period of bits
   (§ 3); a journey's availability conditions become a day type of their own.
3. Meetings of two journeys become interchanges between them (§ 7).
4. Every stop point gets a location: its quay's, else its stop place's (§ 11).
5. Every journey gets a pattern and passing times: kept where it has them, made
   from its calls, journeys calling alike sharing one pattern, or from a time
   demand type (§ 6); what the profile does not hold — a journey's validity
   conditions, `ValidBetween`, calls — is not written.
6. Service link geometries are written latitude first, every position in WGS 84
   (§ 13); a pattern gets its direction, a journey its line's operator.
7. A level reference stays only where it points at a level of its own site, so
   that every reference resolves in the document the sites are written to (§ 8).

## What each profile has to give up or decide on the way

Counted on our deliveries as `netex-rs` reads them. A *run* is a journey on one
of four reference days (2026-10-15, -17, -18, 2026-12-08).

**Austria → EPIP** (all nine Verbünde). Gained: one authority per line, every
stop point with a location, service link geometry latitude first (116 910
links written longitude first, § 13), a direction per pattern, meetings as
interchanges. To be decided, because EPIP states it once where the deliveries
state it many times or not at all:

- *quays* — a stop place is defined in every line document that serves it,
  each copy with the quays that line uses (Vienna defines 22 183 quays): the
  stop place written once must hold the quays of every definition (§ 11);
- *line versions* — a journey runs only on the days its line's version
  governs: on 2026-10-15 the importer's timetables, which apply them, run
  123 162 trips of the nine deliveries (§ 4);
- *runs stated twice* — 9 987 of the Linz AG's 14 163 runs on 2026-10-15
  repeat another; written as they are, each departure is shown twice (§ 12);
- *calendars reused per document* — the Linz AG's 32 periods defined twice with
  other day bits must stay each document's own (§ 2);
- *on-demand services* — 2 123 runs of flexible lines on the four reference
  days in Kärnten, Salzburg, Steiermark, Vienna and Tirol have no form in EPIP
  but fixed journeys (§ 20);
- *passed points* — a point passed through is no call (§ 18);
- *interchange rules* (Vienna 164, ÖBB/EVU 1 596) and Vienna's 45 flexible stop
  places, 45 flexible areas and 46 flexible stop assignments have no EPIP form;
- *notices on journeys* — Vienna states 8 333 notice assignments.

**DELFI → EPIP** (the GVH/BSAG part, 1.3 GB, and the DB part, 3.5 GB). Every
journey, interchange and stop place has its EPIP form; lost are the service
links (7 886 in the GVH/BSAG part — DELFI's carry no geometry) and the groups
of stop places. To be decided: the *flat* journeys — every call at one time —,
6.1 % of the whole delivery (283 961 of 4 661 032 journeys; all of GVH's and
BSAG's, 86 248 in the part): written as they are, they reach every stop in the
minute they leave; the importer leaves them out.

**Switzerland → EPIP** (the five shared documents and timetable documents 1 to
20, 1.4 GB). Gained: a journey pattern for each set of calls alike, made of
102 387 journeys' calls; the 86 127 availability conditions as day types with
UIC periods; interchanges made of meetings. Without an EPIP form: the template
journeys (36, run at a headway — 1 106 runs on 2026-10-15), the interchange
rules (60 984), 69 operating periods without a single day (journeys on them run
on none), the time bands and check constraints. Passed calls (`StopUse`
`passthrough`, 53 817) must not become passing times a reader stops at (§ 18);
the stops keep their SLOID in the key list (§ 11).

**EPIP → a timetable.** Reading an export in EPIP instead of the source loses
whatever the conversion gave up; what it decided on the delivery's behalf a
reader of the export cannot undo. For routing we read the source.

## Into a timetable, as an importer does it

The journey planner's importer that `INTERPRETATION.md` cites, built on this
crate, reads every profile into the same model — stops with their DHIDs,
lines, trips with their calls and days, footpaths, interchanges and
through-runs — and what it cannot hold, or decides on the delivery's behalf, it
counts as a warning. It decides: the versions of a line bound its journeys
(§ 4), runs stated twice are folded (§ 12), the quays of every definition of a
stop place are kept (§ 11), each document's calendar is its own (§ 2),
interchange rules and the Swiss meetings are read (§ 7), the Swiss template
journeys run, flat and passed calls are left out (§ 18). It leaves out the
journeys shorter than their pattern (§ 19) — 232 in the Austrian deliveries —
and what a timetable does not keep at all — notices beyond their texts, the
geometry of service links, levels, vehicle types, authorities.

## What to build where

The transformation is a write path: reading a delivery into the crate's model
is what netex-rs does already, and the step into EPIP is a writer over that
model plus the decisions above. A writer of EPIP belongs into netex-rs only if
we want to *deliver* EPIP — to OpenTripPlanner, or to a partner; for routing we
read every profile directly. The proposals of INTERPRETATION are reader
decisions and belong into netex-rs (what a delivery states) or the importer
(what a timetable makes of it), whichever profile they come from.
