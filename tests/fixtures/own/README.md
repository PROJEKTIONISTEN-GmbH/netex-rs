# Own examples

Small NeTEx deliveries written for this crate's tests, one for each profile the
crate is used with most. Places, lines, operators and times are invented. They
are part of the crate and licensed as it is: **MIT OR Apache-2.0**.

| Directory | Profile | What it pins |
|---|---|---|
| `epip/` | European Passenger Information Profile, a line offer (`epip:EU_PI_LINE_OFFER` with its common, stop, network, calendar and timetable frames) | stop places with quays and their `GlobalID`, stop assignments to a quay or a stop place, a journey pattern stating where one may board and alight, a weekday calendar as UIC day bits, an arrival past midnight |
| `it/` | Italian profile of South Tyrol (IT-ITH1), a line offer in one document as the national access point publishes it | names with their language — a stop place named in Italian with German and Ladin translations among its alternative names, one named in Italian only —, the submodes stop places state beside their mode, a train whose pattern names only its route and the route its line, a calendar of UIC day bits named through an `OperatingPeriodRef`, a train number for advertisement in the timetable frame |
| `at/` | Austrian-style: one document per line, each with its own calendar | identifiers that are each document's own (both documents define `at:ex:DayType:1` for other days), a line in two versions with the journeys of each, a stop place stated in both documents with a quay in each |
| `ch/` | Swiss-style: a shared document and a timetable document naming it | journeys stating their calls rather than a pattern with passing times, a point passed through, a call on the next day, availability conditions as the calendar, a template journey run at a headway, references resolved across documents |
| `fr/` | French profile (NeTEx France): the profile's `stop.xml`, `resource.xml`, `network.xml` and a `line_xyz.xml`, every part in general frames | quays defined beside their stop place and named by it, identifiers of the profile's form `FR:…:LOC`, references into another document by `versionRef`, passing times naming no point of the pattern, day types in the three forms the French deliveries write — dates listing each day, a validity and days of the week with the dates as exceptions, a period without day bits —, a network carried through untyped |
| `nordic/` | Nordic NeTEx Profile: the shared data of a codespace, two line files and a flexible line file, no site in any | stop points assigned to quays of a stop register published apart (`nordic-stops/`), day types assigned a period by dates, a period by operating days and a date not available, a train dated by dated journeys — one cancelled, one replaced by a bus naming the one it replaces —, an interchange from a journey of the other line file, a flexible line its route names and whose journey names only its pattern |
| `nordic-stops/` | the stop register the Nordic line files name, as the national registers publish it | stop places with their quays, two of them within a parent stop place |
| `lu/` | Luxembourg's national delivery: one line offer of the European Passenger Information Profile per line, identifiers `LU::<type>:<number>::` | a walk between two quays of one stop place, a line naming its authority and, as an additional operator, the operator that runs it, times to the second |
| `dk/` | Denmark's national delivery (Rejseplanen): one line offer per line, identifiers `DK::<type>:<number>::` | a train calling at stop points that state their own positions, service link geometries written as GML positions with leading zeros, one of them starting 178 m from its stop point |
| `be/` | the Belgian operators' deliveries, converted from GTFS: a shared `common.xml` with the operator and the calendar, `stops.xml`, and one document per line (the deliveries name it `line-gr:<operator>:<line>.xml`, a name with colons, which this repository writes `line-gr-ex-1.xml`) | no `TypeOfFrameRef` and no version of the publication, the bounds of operating periods written as instants in UTC — the midnights beginning the days in Brussels, either side of the night the clocks go back —, a date not available written as a plain date, stop assignments naming a quay by a `StopPlaceRef`, a line naming neither operator nor mode |
| `cz/` | the Czech national delivery of buses (CIS JŘ), converted from the national timetable format batch by batch: one document per batch, `version="1"` | two documents of one line — its regular timetable and its timetable on a detour — each numbering its stop places from one and stating its stop points under the same identifiers, with other names; no position for any stop place or stop point; times after midnight without a day offset |

All twenty-two documents are valid against the schemas of NeTEx 1.x and 2.0, but
for the references of `ch/ch-timetable.xml` into `ch/ch-service.xml` — an XML
Schema key reference does not reach into another document —, for the passing
times of `fr/line_10.xml`, written as NeTEx 1.x allows them and the French
deliveries write them: without the point they are at, which NeTEx 2.0
requires —, and for the dated journeys of the Nordic train's line file, written
as the Nordic deliveries write them: naming their journey, which NeTEx 1.x does
not let a dated journey do, and the dated journey a replacing one replaces as a
plain `DatedServiceJourneyRef`, where NeTEx 2.0 has `replacedJourneys`. `tests/own_examples.rs`
reads them, writes them back, validates them where the official schemas are
there, and asserts what the index makes of each.
