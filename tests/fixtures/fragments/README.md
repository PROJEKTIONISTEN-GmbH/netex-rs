# Own fragments

Small pieces of NeTEx written for this crate's tests: under `frames/`, sets of
frames that the tests wrap in a publication of their own; under `entities/`,
entities each standing alone, with their namespace declared on them. Each file
pins what the reader makes of one kind of thing a delivery states, and opens
with a comment saying what it holds and what in the standard it rests on — the
NeTEx schema file, XML Schema Part 2 or GML 3.2. Places, lines, operators and
times are invented. They are part of the crate and licensed as it is: **MIT OR
Apache-2.0**.

| File | What it pins |
|---|---|
| `frames/times.xml` | dates, times and durations in every lexical form, zone offsets and fractions, `24:00:00`, negative and shortest durations |
| `frames/day-offsets.xml` | a time after midnight as a day offset and as hours past 24 |
| `frames/unreadable-values.xml` | values not of their type — a time, a duration, a date for a date-time, a position list, a transport mode — kept as written |
| `frames/whitespace.xml` | white space around an enumeration, in a list of days, in names, notices and addresses |
| `frames/frame-order.xml` | frames in an order of their own, an order stated twice in a pattern, passing times beside calls, a train number, an interchange in another frame |
| `frames/references.xml` | references with and without a version, `versionRef`, `nameOfRefClass`, to another version, another class, to nothing; a comment and a CDATA section |
| `frames/nested-objects.xml` | objects inside others with and without their own identifier, version and attributes; a frame's validity and defaults |
| `frames/reference-systems.xml` | positions in WGS 84, UTM, ETRS89 and Gauss-Krüger, a frame's default system and none |
| `frames/line-responsibilities.xml` | lines with an operator, an authority, further operators, none; authorities by codespace; private codes |
| `frames/train-numbers.xml` | train numbers as `TrainNumber` in a general frame and as a journey's name |
| `frames/calendar-periods.xml` | day types given days by UIC periods, shared and combined, an assignment to a day type no document defines |
| `frames/calendar-weekday.xml` | a day type of one weekday with its bits |
| `frames/calendar-holiday.xml` | a holiday day type assigned a single date |
| `frames/journeys-in-place.xml` | a journey's availability condition in place, a journey stating calls, stop assignments, levels, a journey meeting |
| `frames/journeys-and-blocks.xml` | journeys taking their vehicle type from themselves or their block, and one stating nothing |
| `frames/journeys-left-out.xml` | journeys that cannot be ridden, each for another reason |
| `frames/journey-meetings.xml` | journey meetings, one with a journey no document defines, one naming an object of another class |
| `frames/interchanges.xml` | planned interchanges at one point, between stop places, across a border, turning back, at no stop place; one between patterns |
| `frames/levels.xml` | levels and what refers to them — quays, entrances, access spaces, path links, parkings — defined or not |
| `frames/site-structures.xml` | a site structure with its levels, new in NeTEx 2.0 |
| `frames/submodes.xml` | the submode a stop place, its entrance and its quay state beside their mode, and a stop place stating none |
| `frames/quays-beside-their-stop-place.xml` | quays defined beside their stop place, as the French profile writes them: named by their stop place's `QuayRef`, naming it by their `SiteRef`, claimed by two, by none, a `QuayRef` to nothing and to a stop place |
| `frames/axis-order.xml` | link geometries written longitude first, latitude first, far away, from nothing, without geometry |
| `entities/day-types.xml` | day types with and without the schema defaults of their attributes, a name's language, properties of day |
| `entities/calendar.xml` | a day type, a UIC period and the assignment of the one to the other |
| `entities/stop-point.xml` | a scheduled stop point, with and without its position |
| `entities/journey.xml` | a service journey stating its calls |
| `entities/interchanges.xml` | planned interchanges stated in full |

`tests/own_fragments.rs` reads every fragment, writes it back, writes it for
both releases and indexes it in full and streamed; it validates them against
both releases where the official schemas are there. Every fragment is valid but
for references to objects other fragments define, or no document does, and
for what its comment says is not: hours past 24 (`times.xml`,
`day-offsets.xml`), a point in a journey pattern without an identifier
(`nested-objects.xml`), the values of `unreadable-values.xml`, and the site
structure of `site-structures.xml`, which NeTEx 1.x does not know. Tests
beside their subject assert what the index makes of single fragments.
