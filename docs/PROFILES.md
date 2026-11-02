# Profiles: how each country's deliveries are laid out and what the crate reads for them

NeTEx is used through national and European profiles. The crate reads the
documents of any of them into the same model; the index applies the rules each
profile relies on. This page lists, for every profile the crate has been read
against, how its deliveries are laid out, what the crate reads for it, and how
much of it was read; the README has the short form, the pages named below the
detail.

| Profile | How its deliveries are laid out | What the crate reads for it | Read in full |
|---|---|---|---|
| **DELFI** (Germany, `ntx:1.1`) | one document per line, 28 176 in one zip archive; each repeats the stops it serves | stop places merged across documents with all their quays, `GlobalID` (DHID) keys, site connections, interchanges | yes, every file of the national delivery |
| **Austria** | one document per line, each with its own calendar; lines in versions | identifiers that are each document's own, the days each version of a line governs, `OperatingPeriodRef`s naming UIC periods, flexible lines left out and counted | yes, all nine deliveries of the national access point |
| **Switzerland** | shared documents (stop points, lines, calendar) and timetable documents naming them | journeys stating their `Call`s, `StopUse` pass-throughs, availability conditions as the calendar, template journeys with headway groups | yes, the national timetable of 2026 (521 documents, 39.3 GB of XML) |
| **EPIP** (European Passenger Information Profile) | composite frames of typed frames (`epip:EU_PI_*`) | read as any NeTEx; the index as for the others | the official EPIP example and the crate's own |
| **IT-ITH1** (Italy, South Tyrol; the Italian national access point) | one document of EPIP frames (`IT:ITH1:…` identifiers), lines in versions | every name with the language it states; patterns naming their line only through a route; stop places stating a submode beside their mode; train numbers in the timetable frame; `OperatingPeriodRef`s naming UIC periods | yes, levels 1 and 3 of the delivery |
| **NeTEx France** (the French national access point) | per network: stops, calendar, network and one document per line, everything in general frames typed by the part (`FR:TypeOfFrame:NETEX_ARRET`, …); or specific frames typed so; SNCF's trains in one document of 647 MB | general frames; quays defined beside their stop place, which names them by `QuayRef`; passing times naming no point, by position; day types given their days by a validity and days of the week, the assignments their exceptions; plain periods by the days of the week of their day type | yes, five deliveries: Fluo Grand Est (Bas-Rhin), Naolib, TBM, Normandy's aggregate, SNCF |
| **Nordic NeTEx Profile** (Norway, Sweden, Finland) | per codespace, one document per line, a shared and a flexible shared document; no site in any of them, the stops being the national stop register's, published apart | journeys dated by the dated journeys naming them (`DatedServiceJourney`), cancelled and replaced ones giving no day; flexible lines as lines, their journeys left out and counted; operating periods stated by operating days; the stop register read along with the delivery (`Source::with`, `--with`); interchanges across line documents | yes, the national deliveries of Norway (with its stop register), Sweden and Finland |
| **Luxembourg** (EPIP, `ntx:1.1`) | one document per line, 713 in one archive, in a folder per operator | as the German delivery: line offers of typed frames, walks between quays, a line naming its authority and an additional operator | yes, the national delivery |
| **Denmark** (EPIP, `ntx:1.1`, Rejseplanen) | one document per line, 1 560 in one archive | as the German delivery; stop points with positions of their own, link geometries in GML; a link starting farther from its stop than the axis order can be told by is a finding | yes, the national delivery |
| **Belgium** (EPIP, converted from GTFS; SNCB, De Lijn, TEC, STIB) | per operator a `common.xml` with the calendar (SNCB's 1.08 GB), `stops.xml` and one document per line; no `TypeOfFrameRef`, no version | the bounds of operating periods written as instants in UTC, each the midnight beginning a day in Brussels; stop assignments naming a quay by a `StopPlaceRef`, counted; lines naming no operator and no mode, left so | yes, the four operators' deliveries |
| **Czechia** (EPIP, CIS JŘ) | one document per batch of the national formats: a bus or city line, a line on a detour, a train; `version="1"`; no position anywhere | stop places and stop points each document numbers anew: a stop per name, found from its own document (`local-identifier`); stops without a position, journeys past midnight without a day offset, both reported | yes, buses, city transport and trains |
| **Netherlands** (BISON, TMI9 NeTEx-NL, `ntx:1.1` with NeTEx 2.0's `privateCodes`) | one document per operator or concession, at the national access point NDOV Loket; no site in any, the stops being the national stop register's (CHB), published apart; positions in RD New | journeys stating a departure and a time demand type rather than passing times — run times to the tenth of a second along the timing links the pattern names, through timing points that are no stop, loops passing a link twice —; availability conditions in `contentValidityConditions`, day types beside them; private codes; the register's codes `NL:Q:…`, `NL:S:…` as global identifiers, the keys the German delivery names Dutch stops by | yes, six operators' deliveries (RET, GVB, Arriva, Connexxion, Qbuzz, HTM) with the stop register |

`tests/fixtures/own/` has a small delivery of each kind but DELFI, written for
the tests; `tests/fixtures/fragments/` has small sets of frames and entities,
each pinning what the reader makes of one kind of thing a delivery states
(times, white space, references, calendars, levels, axis order …);
`tests/fixtures/delfi/` has five files of the German delivery.
`docs/DELFI-PROFILE.md` lists what the German delivery uses of NeTEx: 193
distinct elements, of which the typed model covers all but four, which are
carried through. `docs/IT-ITH1-PROFILE.md` does the same for South Tyrol's,
with what it states and how the crate reads it, `docs/FR-PROFILE.md` for
five French deliveries, checked against the GTFS feeds their producers publish,
`docs/NORDIC-PROFILE.md` for Norway's, checked against Entur's GTFS feed,
`docs/EPIP-NEIGHBOURS.md` for the EPIP deliveries of Luxembourg, Denmark,
Belgium and Czechia, with what each does not give, and `docs/NL-BISON-PROFILE.md`
for six Dutch operators' deliveries and the stop register, checked against the
national GTFS feed.
