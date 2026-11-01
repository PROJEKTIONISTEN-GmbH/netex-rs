# The EPIP deliveries of Luxembourg, Denmark, Belgium and Czechia

Four of Germany's neighbours publish their timetables in the European
Passenger Information Profile (EPIP), each built otherwise: Luxembourg and
Denmark in the form of the German delivery, one line offer per line;
Belgium's operators each in a delivery converted from their GTFS feeds; the
Czech national system in documents converted batch by batch from the
national timetable formats. This page says what each delivery states, how
`netex-rs` reads it, what changed for them, and — what a user of the data has
to know — what they do not give. Nothing of any delivery is in this
repository: the crate's own examples of their forms, invented, are
`tests/fixtures/own/lu/`, `dk/`, `be/` and `cz/`.

## The deliveries

| | Luxembourg | Denmark | Belgium | Czechia |
|---|---|---|---|---|
| Publisher | the national access point (`data.public.lu`) | Rejseplanen, through the national access point | per operator: SNCB, De Lijn, TEC, STIB (Belgian Mobility Company) | CIS JŘ, the national timetable information system |
| Licence | CC0 | CC BY 4.0 | CC BY 4.0 | none stated: read and measured, nothing of it used here |
| Read | of 2026-10-21 | of 2026-10-30 | SNCB of 2026-10-22, the others of 2026-10-31 | of 2026-10-30; the trains of the timetable year 2026 and its monthly changes |
| Archive | 26 MB, 713 documents and two other files, 652 MB inflated | 129 MB, 1 560 documents, 3.9 GB inflated | SNCB 141 MB, 2 184 documents, 7.8 GB; De Lijn 190 MB, 2 023, 11.9 GB; TEC 105 MB, 849, 6.6 GB; STIB 24 MB, 89, 1.2 GB | buses 203 MB, 13 347 documents, 6.0 GB; trams, trolleybuses and cable cars 68 MB, 909, 2.8 GB; trains 64 MB, 13 252, 1.3 GB |
| Layout | one document per line in a folder per operator | one document per line in a folder per operator; Copenhagen's metro line M3 161 MB | `common.xml` (operator, calendar; SNCB's 1.08 GB), `stops.xml`, one document per line (`line-gr:<operator>:<line>.xml`) | one document per batch of the national format: a line, a line on a detour, a train |
| `version`, frames | `ntx:1.1`; composite frames typed `epip:EU_PI_LINE_OFFER`, each frame typed | as Luxembourg | none; no `TypeOfFrameRef`; every `version` `any` | `1`; frames typed as EPIP's |
| Identifiers | `LU::StopPlace:200405025_CdT::` | `DK::ScheduledStopPoint:8600081_DSB_1::` | `gs:nmbssncb:8821006_23` (a quay), `gr:` lines, `gt:` journeys, `gc:` calendar | `CZ:cisjr_jdf:StopPlace:1` (each document its own), `CZ:CISJR_CZPTT:StopPlace:CZ54212` (trains) |

## What the crate reads

`netex stats` and `netex validate` with this change; untyped, the share of
the element occurrences the typed model carries through as fragments
(`examples/inventory`).

| Delivery | Stops, stop points | Lines | Journeys | Journey days (days) | Untyped |
|---|---|---|---|---|---|
| Luxembourg | 8 197, 6 250 | 713 | 40 289 | 732 121 (2026-10-21 to 2026-12-12) | 0.019 % |
| Denmark | 60 808, 36 882 | 1 560 | 196 818 | 2 748 230 (2026-10-30 to 2027-01-13) | 0.185 % |
| SNCB | 2 950, 924 146 | 2 182 | 158 370 | 1 237 567 (2025-12-29 to 2026-12-12) | 2.345 % |
| De Lijn | 30 596, 271 974 | 2 021 | 350 799 | 2 131 061 (2026-10-15 to 2026-12-31) | 0.828 % |
| TEC | 46 563, 179 849 | 847 | 204 552 | 1 371 675 (2026-10-01 to 2027-01-28) | 0.808 % |
| STIB | 3 296, 10 253 | 87 | 76 043 | 450 475 (2026-10-26 to 2026-11-22) | 0.181 % |
| Czechia, buses | 98 129, 139 087 | 12 478 | 514 212 | 97 223 672 (2016-08-05 to 2039-12-31) | 1.160 % |
| Czechia, city transport | 5 976, 7 598 | 888 | 272 920 | 68 384 251 (2022-09-01 to 2035-12-31) | 0.822 % |
| Czechia, trains | 3 386, 3 386 | 13 252 | 13 252 | 2 875 801 (2025-12-14 to 2026-12-12) | 0.149 % |

What is carried through untyped: Denmark's link geometries (`LineString`
directly in the `ServiceLink`), the Belgian route points' projections, the
Czech journeys' facilities and stop points' tariff zones.

## What changed

Luxembourg and Denmark read as they did: they are written as the German
delivery is. Three things the Belgian and Czech deliveries pose are read
since 0.8.0 (`docs/INTERPRETATION.md` §§ 33–35):

- **Bounds of a period written in UTC.** The Belgian operators write the
  first and last day of every operating period as the instant in UTC of the
  midnight beginning it in Brussels (`2026-10-21T22:00:00Z`). Read by the
  date written, every period began a day early. A bound with a zone from noon
  on names the next day now (`calendar::bound_day`).
- **Stop identifiers each document numbers anew.** The Czech buses and city
  transport number their stop places from one in every document, and a line's
  timetable on a detour states its stop points under the regular timetable's
  identifiers with other names. A stop place, quay or stop point another
  document names otherwise is a stop of its own now, found from the document
  defining it, and a finding (`local-identifier`): the bus delivery has
  98 129 stops instead of 82, and 513 994 of its 514 212 journeys call at
  other stops than before — their own document's.
- **Quays assigned by `StopPlaceRef`.** The Belgian deliveries name each
  stop point's quay with a `StopPlaceRef`. It is read as before, the quay, and
  counted as a reference resolved to another class.

| | Before | After |
|---|---|---|
| STIB, journeys on the days of STIB's GTFS feed | 1 366 of 76 043 | 76 043 of 76 043 |
| TEC, journeys on the days of TEC's GTFS feed | 4 398 of 203 955 | 203 693 of 203 693 |
| TEC, journeys without a day | 597 | 859 |
| Czech buses: stops, stop points | 82, 118 944 | 98 129, 139 087 |
| Czech buses: journeys | 514 014 | 514 212 |
| Czech city transport: stops, stop points | 56, 6 062 | 5 976, 7 598 |
| `local-identifier` (buses, city transport) | — | 118 190, 7 456 |
| `resolved-as-other-class` (SNCB, TEC, STIB) | 59 838, 360, 376 | 983 984, 179 847, 10 629 |

Of TEC's journeys, 278 run on no day now and 16 that ran on none before do.
The 278 are those every day of whose bits an assignment takes away: the
bits of `gc:tec:C2026-choi-Sem-Cong-42-0000100` name the Fridays from
6 November, each of which a dated assignment takes away; read a day early,
they were Thursdays, which none took. TEC's GTFS feed gives them no day
either. The 198 more Czech bus journeys are journeys two
documents state under one identifier, with the same stop point identifiers:
alike before, they call at other stops now, each its own document's.

Every other delivery the crate's documents describe reads the same before
and after, index for index (counts, findings, a hash over every stop and
journey): the German national delivery, the nine Austrian ones, the five
French and Norway's. One Austrian quay identifier in each of two places of
VOR's delivery is named otherwise in another document (`Bahn` and `Zug`,
`Bus` and `Bus + SEV`): two quays each now, and two findings.

## Belgium against its GTFS

Each Belgian operator publishes a GTFS feed beside its NeTEx, and the NeTEx
is converted from it. `netex convert --to gtfs`, trip for trip by
identifier:

| | Trips in both | Same days | Same stops and times |
|---|---|---|---|
| STIB | 76 043 | 76 043 | 76 043 |
| TEC | 203 693 | 203 693 | 203 693 |
| De Lijn | 345 482 | 241 931 | 345 482 |
| SNCB | 124 298 | 106 193 | 123 907 |

STIB's and TEC's NeTEx and GTFS are of one export. De Lijn's and SNCB's are
not: the feed of De Lijn holds 9 028 trips the NeTEx does not, and the NeTEx
5 317 the feed does not; SNCB's feed is of 30 October, its NeTEx of
22 October. Both state every day by a dated assignment — no period of
theirs has a day bit —, and their journeys run on the dates the NeTEx
lists: the trips of De Lijn whose days differ differ in those dates
themselves (`gt:delijn:1001_101_177_3ANTRAM-32_…_1493_…` is assigned 9, 10
and 12 November in the NeTEx, 13 November as well in the feed). 310 of
SNCB's 391 trips with other stop times are among those whose passing times
name points their pattern does not define (below); the other 81 were not
traced, the feed being eight days younger than the NeTEx.

## What the deliveries do not give

**Luxembourg.** Nothing missing: every reference resolves; two quays nothing
refers to.

**Denmark.** 183 service links whose geometry starts more than 150 m from the
stop point the link starts at — the border trains to Flensburg among them —:
their axis order is not told (`axis-order-undecided`); the geometries are
written latitude first.

**Belgium.**
- No line names its operator or its mode: `netex stats` counts every journey
  under no operator, `netex convert` writes every route as a bus and names no
  agency but the publisher given. A user knows the operator by the delivery;
  the mode — SNCB's trains, STIB's metro and trams — the NeTEx does not say.
- SNCB: the passing times of 429 journeys name points of their pattern the
  pattern does not define (683 references to nothing). In those looked at,
  the pattern numbers its points from `…-3`, the passing times from `…-1`:
  the first two times name nothing, and each other the point two places
  further on, so that the journey leaves its first stop at its third time.
  Read as written: 142 of them call short of their pattern's last points
  (`untimed-ends`), 17 have fewer than two timed calls left and are left out.
- TEC: 859 journeys run on no day: their periods' bits name none, or only
  days a dated assignment takes away.
- The 1.08 GB `common.xml` of SNCB holds the calendar of every journey as
  dated assignments — 5 065 213 of them —; `netex stats` of it alone takes
  18 s and 2.0 GiB, of which 1.08 GB is the document's text.

**Czechia.**
- No stop place and no stop point states a position, in any of the three
  deliveries: every stop point called at is without one (132 143 of the
  buses', 7 399 of city transport's, 3 386 of the trains'). Converted to
  GTFS, which needs positions, the stops and the journeys calling at them are
  left out, and the report says so. A user has to place the stops: the bus
  and city stops by their names alone, written as the national format writes
  them (`Bernartice,,nám.`: town, part of town, place); the train stations
  by their national number (`CZ54212`).
- Stop identifiers are each document's own (above); a stop named alike in
  two documents is taken as one, though nothing but the name says so.
- 3 674 bus and 2 504 city journeys run past midnight without stating a day
  offset: their times go back (`time-goes-back`) and are kept as stated; the
  GTFS conversion leaves them out. NeTEx reads a missing offset as none.
- 1 631 bus and 7 city journeys run on no day: in those looked at, their
  periods' bits are all zero.
- The trains are published as a timetable for the year and a monthly
  archive of changes (`NeTEx_GVD2026_updates_<month>.zip`), each train
  variant a document. A change is a variant of its own with its days; the
  NeTEx does not say which variant of the timetable for the year it
  replaces. Of the 3 492 train numbers of October's changes, 2 394 are run on
  some day of the change by the timetable for the year as well: read
  together (`--with`), both run.
- Each train is a line of its own (13 252 lines, 13 252 journeys).

## Time and memory

`netex stats` of every delivery and `netex validate` of the two largest,
netex-rs at the base of this change and with it alternately (base, new,
new, base), on eight threads of a shared host of 64 cores under a load of
11 to 24:

| Delivery | Command | Before | After |
|---|---|---|---|
| Luxembourg | `netex stats` | 0.9 s, 0.9 s; 0.13 GiB | 0.9 s, 0.9 s; 0.13 GiB |
| Denmark | `netex stats` | 4.7 s, 4.7 s; 0.78 GiB | 4.7 s, 4.7 s; 0.78 GiB |
| STIB | `netex stats` | 0.9 s, 0.9 s; 0.32 GiB | 0.9 s, 0.9 s; 0.32 GiB |
| TEC | `netex stats` | 4.5 s, 4.5 s; 1.23 GiB | 4.2 s, 4.3 s; 1.23 GiB |
| De Lijn | `netex stats` | 7.7 s, 8.0 s; 2.04 GiB | 7.2 s, 7.2 s; 2.05 GiB |
| SNCB | `netex stats` | 26.2 s, 24.1 s; 6.01 GiB | 24.1 s, 24.1 s; 6.01 GiB |
| Czechia, city transport | `netex stats` | 3.9 s, 3.9 s; 0.46 GiB | 3.9 s, 3.7 s; 0.47 GiB |
| Czechia, trains | `netex stats` | 2.0 s, 2.1 s; 0.22 GiB | 2.1 s, 2.0 s; 0.23 GiB |
| Czechia, buses | `netex stats` | 10.0 s, 9.4 s; 1.37 GiB | 9.0 s, 9.0 s; 1.43 GiB |
| SNCB | `netex validate` | 44.6 s, 41.6 s; 5.48 GiB | 42.8 s, 41.9 s; 5.49 GiB |
| Czechia, buses | `netex validate` | 185.3 s, 184.8 s; 1.43 GiB | 189.4 s, 189.6 s; 1.50 GiB |

Reading the Czech buses' stops apart costs `validate` 2 % more time and
0.07 GiB more memory: 98 047 more stops and 20 143 more stop points, and the
references to them. SNCB's index is large: 924 146 stop points, each under
an identifier naming its line's whole route
(`gr:nmbssncb:184-gs:…_gs:…`; 465 characters on average in line 1147's
document, 926 in line 184's), and 5 million dated assignments. `netex stats`
of SNCB peaks at 4.0 GiB on one thread, 5.0 on two, 5.7 on four and 6.0 on
eight.

## Not read

- The Czech monthly changes of the trains were measured against the timetable
  for the year, not read into one delivery.
- Denmark's real-time data (SIRI-ET) and Belgium's (GTFS-RT) are no NeTEx.
