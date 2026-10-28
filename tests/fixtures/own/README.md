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

All six documents are valid against the schemas of NeTEx 1.x and 2.0, but for
the references of `ch/ch-timetable.xml` into `ch/ch-service.xml`: an XML Schema
key reference does not reach into another document. `tests/own_examples.rs`
reads them, writes them back, validates them where the official schemas are
there, and asserts what the index makes of each.
