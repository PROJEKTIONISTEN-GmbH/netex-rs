# The Nordic NeTEx Profile

Norway, Sweden and Finland publish their timetables in the Nordic NeTEx
Profile, drawn up by Entur and the Nordic public transport authorities (ODIN)
on NeTEx 1.x ([Nordic NeTEx Profile](https://enturas.atlassian.net/wiki/spaces/PUBLIC/pages/728891481/Nordic+NeTEx+Profile),
examples at [github.com/entur/profile-examples](https://github.com/entur/profile-examples)).
A delivery is a flat archive: one document per line, two shared documents per
codespace, and no site frame in any of them — the stops are the national stop
register's, published apart. Identifiers have the form
`<codespace>:<type>:<id>` (`RUT:Line:100`, `NSR:Quay:9011`; Sweden's
`SE:050:Quay:…`). This page describes Norway's national delivery as Entur
published it on 2026-10-31 (`rb_norway-aggregated-netex.zip`, under the
Norwegian Licence for Open Government Data, NLOD 2.0): what it states, how
`netex-rs` reads it, what was read before and after the crate learned the
profile, how the result compares with the GTFS feed Entur publishes from the
same data, and its inventory with `examples/inventory`. Nothing of the
delivery is in this repository: the crate's own example of the profile is
`tests/fixtures/own/nordic/`, with its stop register in
`tests/fixtures/own/nordic-stops/`. Sweden's delivery (Trafiklab, which asks
for a key) was not read.

## The delivery

| | Norway, national (Entur) |
|---|---|
| Licence | NLOD 2.0 |
| Archive | 289 MB; 4 819 documents, 5.7 GB inflated |
| `version` | `1.15:NO-NeTEx-networktimetable:1.5`; the stop register `1.15:NO-NeTEx-stops:1.5` |
| Layout | per codespace — 60 of them — one document per line (`RUT_RUT-Line-100_100_Kjeller---Oslo-bussterminal.xml`, 4 051), one per flexible line (698), the shared data (`_RUT_shared_data.xml`, 35) and the shared data of flexible lines (`_RUT_flexible_shared_data.xml`, 34); the stop register as one more document, `_stops.xml` (413 MB) |
| Frames | a composite frame of a resource, service, service calendar and timetable frame; site frames only in the stop register and, for flexible stop places, in seven shared documents of flexible lines |
| Days of the journeys | 2019-04-15 to 2029-10-30 |

The national archive holds the stop register; the archives Entur publishes per
codespace (`rb_<codespace>-aggregated-netex.zip`) do not, and the register is
published apart, by the national stop register (NSR) as
`tiamat/Current_latest.zip`.

## What it states, and how the crate reads it

**Stops.** The line documents define no site: the shared document of each
codespace assigns its stop points to quays by `QuayRef` (272 991 assignments),
the quays the stop register defines — 58 007 stop places with 101 828 quays,
and a stop point of its own for each (159 835, in its service frame). A stop
place may stand within a parent stop place (`ParentSiteRef`, 3 479 of them),
which groups the stop places of one interchange. Read with its register, every
assignment of the delivery finds its quay but two: the register of the day —
inside the archive and published apart alike — has no `NSR:Quay:62109` and no
`NSR:Quay:62110`. A per-codespace archive is read with the register along:
`Source::with` reads several sources as one, the documents of each source
after the first named under its file name; `netex validate`, `stats`,
`convert` and `extract` take it as `--with PATH`. Ruter's 394 documents
(`RUT`) read alone leave 7 249 references to quays resolving to nothing; read
with the register NSR published the same day, none.

**Identifiers.** Every identifier names its codespace (`RUT:ServiceJourney:…`,
`NSR:Quay:…`), and each document declares the codespaces it uses
(`codespaces/Codespace`). No two codespaces share an identifier; a reference
from a line document into the codespace's shared document resolves in the
delivery, as any reference does that its own document does not answer
(`docs/INTERPRETATION.md` § 2).

**Calendar.** Day types name days of the week (`PropertyOfDay/DaysOfWeek`) and
are assigned operating periods — by dates, or, in 25 codespaces and 383
periods, by their first and last operating day (`FromOperatingDayRef`,
`ToOperatingDayRef`) — and dates, some not available (`isAvailable` false). An
operating period stated by its operating days takes their dates since this
change (§ 30); before, it gave no day.

**Dated journeys.** 17 codespaces — `RUT` in 391 documents, `VYG`, `SJN`,
`NWY` and more — give their journeys no day type: 2 158 080
`DatedServiceJourney`s in 507 documents each name a journey
(`ServiceJourneyRef`) and the operating day it runs on (`OperatingDayRef`),
and nothing else. 3 359 are cancelled and 6 681 replaced
(`ServiceAlteration`); the replacing ones name the dated journey they replace
by a `DatedServiceJourneyRef` (17 613), where NeTEx 2.0 has `replacedJourneys`.
Since this change a dated journey stating no journey of its own dates the one
it names, a cancelled or replaced one giving no day (§ 28). 106 259 journeys
run on dated journeys alone; 1 989 of them on none, every dated journey naming
them cancelled or replaced.

**Interchanges.** 72 798 `ServiceJourneyInterchange`s in 1 080 documents,
64 943 of them from a journey another line document defines: the index
resolves them across the delivery, and keeps all 72 798.

**Flexible lines.** 698 `FlexibleLine`s, among a service frame's lines; their
routes name them by `FlexibleLineRef`. Of their 2 390 journeys 1 158 name the
line so too, the others only their pattern; they state the window a ride may
be booked for (`EarliestDepartureTime`, `LatestArrivalTime`) and their booking
(`BookingArrangements`, `FlexibleServiceProperties`); flexible stop places
with their areas (973) are assigned by `FlexibleStopAssignment`. Since this
change a flexible line is a line of the index, and a journey of one — by its
line, its route or its pattern — is left out and counted as the Austrian ones
are (§ 29, § 20); the booking and the flexible stop places are carried
through.

**A condition to the year 9999.** Flytoget's documents (`FLT`) state the
validity of their frames as an availability condition from 2026-10-28 to
9999-12-31. No journey names it; before this change every calendar of the
delivery spanned the first to the last day of any condition, 2.9 million days
(§ 31).

## Read before and after

The index of the delivery read with netex-rs at the base of this change (after
0.7.2, the French profile) and with this change (`netex validate`, `netex
stats`); untyped, the share of the delivery's element occurrences that are
fragments the typed model carries through (`examples/inventory`).

| | Before | After |
|---|---|---|
| Untyped | 0.890 % | 0.858 % |
| Stops (stop places and quays) | 159 835 | 159 835 |
| Stop points | 273 963 | 273 963 |
| Lines | 4 051 | 4 749, the 698 flexible ones among them |
| Journeys in the index | 377 191 | 377 150 |
| Journeys running on a day (`netex stats`) | 269 361 | 375 161 |
| Journey days (`netex stats`) | 6 232 756 | 8 536 762 |
| Stops called at, lines with a journey (`netex stats`) | 82 110, 3 520 | 90 205, 4 050 |
| References that do not resolve | 826 | 2 |
| Journeys left with fewer than two timed calls | 2 159 271 | 0 |
| Journeys without a day | 2 266 214 | 1 989 |
| Periods without a first day | 383 | 0 |
| Journeys of flexible lines left out | 1 158 | 2 390 |
| Definitions nothing refers to | 7 786 | 4 677 |

Before, every dated journey was read as a journey of its own, with no pattern
and no times — 2 158 080 of the 2 159 271 journeys left with fewer than two
timed calls —, and the journeys they date as journeys without a day; the
routes of flexible lines named lines that, untyped, no document defined (824
of the 826 references). 1 191 journeys of flexible lines stating only a booking
window were left with fewer than two timed calls, 41 stating fixed times were
in the index: both are left out now as journeys of flexible lines.

## Time and memory

`netex stats` and `netex validate` of the national archive, netex-rs at the
base of this change and with it alternately (base, new, new, base), on a
shared host of 64 cores under a load of 2.5 to 5, the process limited to 8:

| | Before | After |
|---|---|---|
| `netex stats` | 355.5 s, 357.6 s; 8.6 GiB | 9.2 s, 9.1 s; 1.8 GiB |
| `netex validate` | 736.8 s, 798.4 s; 8.5 GiB | 97.5 s, 102.1 s; 1.8 GiB |

Before, most of `netex stats`'s time went to the calendars, 2.9 million days
each, from the first day of the delivery to the 9999 of Flytoget's frames
(`docs/INTERPRETATION.md` § 31). In runs apart from these measurements, of
this change before and after the calendars were narrowed to the days that can
give a journey a day, the index was built in 36 s and 12 s, and asking it for
the days of each of its journeys took 293 s and under 0.1 s.

## Checked against Entur's GTFS

Entur publishes a GTFS feed of the same delivery (`rb_norway-aggregated-gtfs.zip`,
published the same evening as the NeTEx). `netex convert --to gtfs` of the
NeTEx against it, trip for trip by identifier:

| | Trips | Days the same | Stops and stop times the same |
|---|---|---|---|
| In both feeds | 374 797 | 374 538 | 374 793 |
| netex-rs's feed only | 364: 353, and 11 under other identifiers | | |
| Entur's feed only | 11, the same 11 | | |

Every difference has its reason:

- **259 trips, other days.** Entur's feed gives each of them every day
  netex-rs gives it, and besides only days that an assignment with
  `isAvailable` false naming an operating period takes away from one of its
  day types (`BFO` 196, `BEF` 52, `OIS` 8, `FJT` 3 trips; checked trip by
  trip against the assignments of the delivery). The crate takes the days
  away, as the standard says and as OpenTripPlanner's reader of NeTEx does.
- **4 trips, other stops and times.** One stop assignment of `SOF` each names a
  quay the register does not hold (`NSR:Quay:62109`, `NSR:Quay:62110`): the
  call is left out of the trip, with its finding. Entur's feed, written from
  another state of the register, has the quays.
- **353 trips in netex-rs's feed only**, all of Voss Gondol (`VOG`): they run in
  May 2021, as the delivery still states; Entur's feed leaves expired
  services out.
- **11 trips under other identifiers.** `GOF`'s journeys, `GOF:ServiceJourney:3`
  and on, are `GOF:VehicleJourney:3` and on in Entur's feed; under those
  identifiers their days, stops and times are the same.

Neither feed has the 2 390 journeys of flexible lines, nor the 1 989 journeys
every dated journey of which is cancelled or replaced. Journey days: 8 530 169
in netex-rs's feed, 8 532 191 in Entur's.

## Converting to GTFS

`netex convert --to gtfs` takes the time zone from the frames
(`Europe/Oslo`) and writes 375 161 trips with 8 990 058 stop times — the
4 calls at the two missing quays left out — and 72 714 transfers from the
72 798 interchanges, 84 of them stated twice. A quay stands under its stop
place as a station or, where that stop place is within a parent stop place,
under the parent, as GTFS stations do not nest
(`crates/netex2gtfs/README.md`); Entur's feed puts it under its own stop
place. The stop register's quays have no name of their own: each is named as
its stop place. Not carried into the feed, and said so in the report: notices,
networks, flexible lines, flexible stop places and their assignments, route
points and service links, the register's fares, topographic places, tariff
zones, groups of stop places and parkings.

## Several deliveries into one

The identifiers of the profile name their codespace, which is the
publisher's: a Nordic delivery and a delivery of another profile share no
identifier, and `netex merge` (`merge::merge`) writes both as they are,
nothing paired, left out or renamed — what `tests/merge.rs` pins for the
crate's own Nordic example and its EPIP one, in either rank. A stop that two
deliveries both state pairs only by an identifier or a global identifier both
give it (§ 16): Norway's stop register states no `GlobalID` and no `SLOID`;
its `keyList` names the identifiers it took a stop over from
(`imported-id`), which the merge does not pair by.

## What is not read

- A dated journey stating a pattern and passing times of its own is a journey
  of its own, as before; its operating day is not read as its day.
- `replacedJourneys` (NeTEx 2.0's form of the replaced dated journeys) is
  carried through; the index needs neither form, the replaced dated journey
  stating its own `ServiceAlteration`.
- Flexible lines are left out, with their booking, flexible stop places and
  areas (§ 20).
- Sweden's delivery and its stop register (`_stops.xml`, codespace `SE:050`)
  were not read; the own example writes Norway's form.

## The inventory of the delivery

4 819 files, 5.7 GB inflated, inventoried with `examples/inventory`.

309 distinct elements, 50272353 occurrences; 479 distinct (parent, child) pairs; 295 distinct (element, attribute) pairs.

### Carried through untyped after the typed read

431293 fragments in all, 0.858 % of the 50272353 element occurrences (a fragment counts once, whatever it holds).

| Parent | Untyped child | Occurrences |
|---|---|---|
| `ServiceLink` | `projections` | 138132 |
| `RoutePoint` | `projections` | 108398 |
| `StopPlace` | `Weighting` | 57885 |
| `StopPlace` | `tariffZones` | 57280 |
| `JourneyPattern` | `linksInSequence` | 27840 |
| `Quay` | `placeEquipments` | 22308 |
| `StopPointInJourneyPattern` | `BookingArrangements` | 9932 |
| `StopPlace` | `placeEquipments` | 1543 |
| `ServiceJourney` | `FlexibleServiceProperties` | 1339 |
| `DestinationDisplay` | `vias` | 1062 |
| `stopAssignments` | `FlexibleStopAssignment` | 973 |
| `Line` | `Presentation` | 696 |
| `TopographicPlace` | `Polygon` | 539 |
| `DestinationDisplay` | `variants` | 436 |
| `FlexibleLine` | `BookingContact` | 364 |
| `FlexibleLine` | `BookingMethods` | 364 |
| `FlexibleLine` | `BookingNote` | 348 |
| `FlexibleLine` | `MinimumBookingPeriod` | 333 |
| `Quay` | `boardingPositions` | 279 |
| `FlexibleLine` | `BookingAccess` | 266 |
| `FlexibleLine` | `BuyWhen` | 266 |
| `Operator` | `CustomerServiceContactDetails` | 240 |
| `GroupOfStopPlaces` | `PurposeOfGroupingRef` | 153 |
| `StopPlace` | `adjacentSites` | 126 |
| `ServiceFrame` | `Network` | 69 |
| `FlexibleLine` | `BookWhen` | 31 |
| `FlexibleLine` | `LatestBookingTime` | 29 |
| `Operator` | `Address` | 29 |
| `ResourceFrame` | `typesOfValue` | 11 |
| `ServiceFrame` | `additionalNetworks` | 8 |
| `SiteFrame` | `flexibleStopPlaces` | 7 |
| `CompositeFrame` | `versions` | 2 |
| `Notice` | `alternativeTexts` | 1 |
| `SiteFrame` | `groupsOfTariffZones` | 1 |
| `SiteFrame` | `parkings` | 1 |
| `SiteFrame` | `tariffZones` | 1 |
| `dataObjects` | `FareFrame` | 1 |

### Elements

| Element | Occurrences | Files |
|---|---|---|
| `StopPointInJourneyPatternRef` | 9039716 | 4749 |
| `TimetabledPassingTime` | 9039716 | 4749 |
| `DepartureTime` | 8561911 | 4167 |
| `OperatingDayRef` | 2184035 | 509 |
| `DatedServiceJourney` | 2158080 | 507 |
| `ServiceJourneyRef` | 2158080 | 507 |
| `ScheduledStopPointRef` | 974053 | 4819 |
| `ArrivalTime` | 726867 | 4167 |
| `StopPointInJourneyPattern` | 699082 | 4749 |
| `ServiceLinkInJourneyPattern` | 664902 | 4014 |
| `ServiceLinkRef` | 664902 | 4014 |
| `Name` | 578547 | 4819 |
| `PointOnRoute` | 569297 | 4749 |
| `RoutePointRef` | 569297 | 4749 |
| `PrivateCode` | 519923 | 4618 |
| `DayTypeRef` | 445187 | 4294 |
| `pos` | 425588 | 3 |
| `JourneyPatternRef` | 379540 | 4749 |
| `ServiceJourney` | 379540 | 4749 |
| `passingTimes` | 379540 | 4749 |
| `FromDate` | 377393 | 4819 |
| `ValidBetween` | 377093 | 254 |
| `Key` | 339176 | 66 |
| `KeyValue` | 339176 | 66 |
| `Value` | 339176 | 66 |
| `ScheduledStopPoint` | 273963 | 70 |
| `dayTypes` | 273431 | 4294 |
| `PassengerStopAssignment` | 272991 | 64 |
| `DepartureDayOffset` | 252453 | 522 |
| `projections` | 246530 | 200 |
| `LineRef` | 235708 | 4051 |
| `FromPointRef` | 215307 | 1117 |
| `ToPointRef` | 215307 | 1117 |
| `QuayRef` | 214984 | 64 |
| `OperatorRef` | 206145 | 4749 |
| `keyList` | 173579 | 66 |
| `Centroid` | 163038 | 1 |
| `Latitude` | 163038 | 1 |
| `Location` | 163038 | 1 |
| `Longitude` | 163038 | 1 |
| `ServiceLink` | 142509 | 37 |
| `Distance` | 142219 | 36 |
| `LineString` | 138132 | 36 |
| `LinkSequenceProjection` | 138132 | 36 |
| `TariffZoneRef` | 130999 | 1 |
| `posList` | 125999 | 41 |
| `PointProjection` | 108398 | 197 |
| `RoutePoint` | 108398 | 197 |
| `ProjectedPointRef` | 102680 | 39 |
| `Quay` | 101828 | 1 |
| `TransportMode` | 99455 | 4759 |
| `DayTypeAssignment` | 80756 | 150 |
| `PublicCode` | 73889 | 4738 |
| `FromJourneyRef` | 72798 | 1080 |
| `ServiceJourneyInterchange` | 72798 | 1080 |
| `ToJourneyRef` | 72798 | 1080 |
| `Guaranteed` | 71779 | 1063 |
| `IsWaitPoint` | 64952 | 380 |
| `AccessibilityAssessment` | 60577 | 1 |
| `AccessibilityLimitation` | 60577 | 1 |
| `MobilityImpairedAccess` | 60577 | 1 |
| `StepFreeAccess` | 60577 | 1 |
| `WheelchairAccess` | 60577 | 1 |
| `limitations` | 60577 | 1 |
| `MaximumWaitTime` | 59982 | 426 |
| `StopPlaceRef` | 58430 | 1 |
| `StopPlace` | 58007 | 1 |
| `TopographicPlaceRef` | 57936 | 1 |
| `Weighting` | 57885 | 1 |
| `tariffZones` | 57281 | 1 |
| `StopPlaceType` | 56907 | 1 |
| `quays` | 56906 | 1 |
| `CompassBearing` | 56788 | 1 |
| `ForBoarding` | 55725 | 4749 |
| `pointsInSequence` | 52894 | 4749 |
| `NoticeAssignment` | 50371 | 1363 |
| `NoticeRef` | 50371 | 1363 |
| `NoticedObjectRef` | 50371 | 1363 |
| `ForAlighting` | 45149 | 4749 |
| `Date` | 43933 | 116 |
| `AudibleSignalsAvailable` | 42055 | 1 |
| `EscalatorFreeAccess` | 42055 | 1 |
| `LiftFreeAccess` | 42055 | 1 |
| `TransportSubmode` | 37975 | 4748 |
| `DestinationDisplayRef` | 37859 | 4755 |
| `RailSubmode` | 30970 | 51 |
| `ArrivalDayOffset` | 29895 | 561 |
| `Description` | 29124 | 4069 |
| `JourneyPattern` | 29062 | 4749 |
| `RouteRef` | 29062 | 4749 |
| `linksInSequence` | 27840 | 4014 |
| `StaySeated` | 25545 | 696 |
| `placeEquipments` | 23851 | 1 |
| `Route` | 23832 | 4749 |
| `DirectionType` | 22582 | 3950 |
| `isAvailable` | 21517 | 45 |
| `GeneralSign` | 21120 | 1 |
| `TimingPointStatus` | 20741 | 4 |
| `SignContentType` | 19788 | 1 |
| `DatedServiceJourneyRef` | 17613 | 24 |
| `DayType` | 17534 | 150 |
| `FrontText` | 15372 | 166 |
| `DestinationDisplay` | 14936 | 166 |
| `ToDate` | 13785 | 4437 |
| `ShortName` | 13439 | 2888 |
| `OperatingPeriodRef` | 10868 | 45 |
| `Url` | 10851 | 466 |
| `BookingContact` | 10743 | 761 |
| `BookingMethods` | 10654 | 761 |
| `Phone` | 10647 | 556 |
| `ServiceAlteration` | 10040 | 33 |
| `BookingArrangements` | 9932 | 365 |
| `OperatingPeriod` | 9730 | 45 |
| `Codespace` | 9673 | 4818 |
| `Xmlns` | 9673 | 4818 |
| `XmlnsUrl` | 9673 | 4818 |
| `MinimumBookingPeriod` | 9392 | 388 |
| `BookWhen` | 9256 | 398 |
| `DaysOfWeek` | 8897 | 46 |
| `PropertyOfDay` | 8897 | 46 |
| `ShelterEquipment` | 8874 | 1 |
| `Enclosed` | 8843 | 1 |
| `PublicityChannel` | 7893 | 51 |
| `BusSubmode` | 7123 | 4207 |
| `Seats` | 6472 | 1 |
| `StepFree` | 6471 | 1 |
| `properties` | 6191 | 46 |
| `ProjectToPointRef` | 5718 | 158 |
| `DefaultLanguage` | 4819 | 4819 |
| `DefaultLocale` | 4819 | 4819 |
| `FrameDefaults` | 4819 | 4819 |
| `ParticipantRef` | 4819 | 4819 |
| `PublicationDelivery` | 4819 | 4819 |
| `PublicationTimestamp` | 4819 | 4819 |
| `ServiceFrame` | 4819 | 4819 |
| `TimeZone` | 4819 | 4819 |
| `dataObjects` | 4819 | 4819 |
| `CompositeFrame` | 4818 | 4818 |
| `codespaces` | 4818 | 4818 |
| `frames` | 4818 | 4818 |
| `validityConditions` | 4818 | 4818 |
| `RepresentedByGroupRef` | 4749 | 4749 |
| `TimetableFrame` | 4749 | 4749 |
| `journeyPatterns` | 4749 | 4749 |
| `lines` | 4749 | 4749 |
| `routes` | 4749 | 4749 |
| `vehicleJourneys` | 4749 | 4749 |
| `VisualSignsAvailable` | 4730 | 1 |
| `AvailabilityCondition` | 4565 | 4565 |
| `Advertised` | 4153 | 118 |
| `Planned` | 4153 | 118 |
| `Line` | 4051 | 4051 |
| `RequestStop` | 3544 | 18 |
| `ParentSiteRef` | 3479 | 1 |
| `Text` | 3406 | 29 |
| `Notice` | 3405 | 29 |
| `EarliestDepartureTime` | 3276 | 587 |
| `LatestArrivalTime` | 3276 | 587 |
| `CalendarDate` | 3267 | 44 |
| `OperatingDay` | 3267 | 44 |
| `SideText` | 3106 | 5 |
| `LinearRing` | 2610 | 8 |
| `Polygon` | 2604 | 8 |
| `exterior` | 2604 | 8 |
| `AlternativeName` | 2197 | 1 |
| `NameType` | 2197 | 1 |
| `BoardingPosition` | 1999 | 1 |
| `FlexibleLineRef` | 1982 | 698 |
| `noticeAssignments` | 1451 | 1363 |
| `BookingNote` | 1345 | 381 |
| `FlexibleServiceProperties` | 1339 | 366 |
| `ContactPerson` | 1314 | 151 |
| `FareZoneRef` | 1182 | 1 |
| `FurtherDetails` | 1176 | 48 |
| `BookingAccess` | 1153 | 594 |
| `alternativeNames` | 1151 | 1 |
| `Email` | 1143 | 368 |
| `Via` | 1099 | 31 |
| `LatestBookingTime` | 1094 | 382 |
| `FlexibleArea` | 1089 | 7 |
| `journeyInterchanges` | 1080 | 1080 |
| `vias` | 1062 | 31 |
| `Parking` | 1049 | 1 |
| `ParkingPaymentProcess` | 1049 | 1 |
| `ParkingVehicleTypes` | 1049 | 1 |
| `ParkingCapacity` | 1039 | 1 |
| `ParkingUserType` | 1039 | 1 |
| `FlexibleStopAssignment` | 973 | 7 |
| `FlexibleStopPlace` | 973 | 7 |
| `FlexibleStopPlaceRef` | 973 | 7 |
| `areas` | 973 | 7 |
| `NumberOfSpaces` | 895 | 1 |
| `Content` | 894 | 1 |
| `TotalCapacity` | 772 | 1 |
| `FlexibleLine` | 698 | 698 |
| `FlexibleLineType` | 698 | 698 |
| `Colour` | 696 | 696 |
| `Presentation` | 696 | 696 |
| `TextColour` | 696 | 696 |
| `ParkingProperties` | 648 | 1 |
| `ParkingUserTypes` | 648 | 1 |
| `parkingProperties` | 648 | 1 |
| `spaces` | 648 | 1 |
| `AuthorityRef` | 590 | 70 |
| `CountryRef` | 539 | 1 |
| `Descriptor` | 539 | 1 |
| `TopographicPlace` | 539 | 1 |
| `TopographicPlaceType` | 539 | 1 |
| `PaymentMethods` | 520 | 1 |
| `ParentTopographicPlaceRef` | 519 | 1 |
| `ViaType` | 515 | 1 |
| `TariffZone` | 491 | 1 |
| `FareZone` | 485 | 1 |
| `ScopingMethod` | 485 | 1 |
| `ZoneTopology` | 485 | 1 |
| `neighbours` | 480 | 1 |
| `Monitored` | 437 | 437 |
| `DestinationDisplayVariant` | 436 | 2 |
| `DestinationDisplayVariantMediaType` | 436 | 2 |
| `variants` | 436 | 2 |
| `Heated` | 404 | 1 |
| `Sanitary` | 404 | 1 |
| `WaitingRoomEquipment` | 404 | 1 |
| `FromOperatingDayRef` | 383 | 25 |
| `ToOperatingDayRef` | 383 | 25 |
| `WaterSubmode` | 366 | 257 |
| `BuyWhen` | 336 | 274 |
| `SanitaryEquipment` | 336 | 1 |
| `SanitaryFacilityList` | 336 | 1 |
| `Gender` | 333 | 1 |
| `ContactDetails` | 321 | 69 |
| `LegalName` | 310 | 63 |
| `NumberOfSpacesWithRechargePoint` | 306 | 1 |
| `NumberOfToilets` | 290 | 1 |
| `Operator` | 279 | 69 |
| `boardingPositions` | 279 | 1 |
| `ParkingLayout` | 271 | 1 |
| `members` | 248 | 1 |
| `CustomerServiceContactDetails` | 240 | 57 |
| `CompanyNumber` | 224 | 30 |
| `OrganisationType` | 207 | 33 |
| `Covered` | 201 | 1 |
| `routePoints` | 197 | 197 |
| `ScopeOfTicketsAvailable` | 184 | 1 |
| `TicketMachines` | 184 | 1 |
| `TicketOffice` | 184 | 1 |
| `TicketTypesAvailable` | 184 | 1 |
| `TicketingEquipment` | 184 | 1 |
| `TicketingFacilityList` | 184 | 1 |
| `VehicleModes` | 184 | 1 |
| `NumberOfMachines` | 183 | 1 |
| `ServiceCalendarFrame` | 166 | 166 |
| `destinationDisplays` | 166 | 166 |
| `GroupOfStopPlaces` | 155 | 1 |
| `PurposeOfGroupingRef` | 153 | 1 |
| `dayTypeAssignments` | 150 | 150 |
| `SiteRef` | 126 | 1 |
| `adjacentSites` | 126 | 1 |
| `TaxiSubmode` | 109 | 98 |
| `Network` | 105 | 69 |
| `AirSubmode` | 101 | 99 |
| `Authority` | 82 | 69 |
| `InverseRouteRef` | 78 | 39 |
| `ResourceFrame` | 70 | 70 |
| `scheduledStopPoints` | 70 | 70 |
| `stopAssignments` | 70 | 70 |
| `organisations` | 69 | 69 |
| `BrandingRef` | 53 | 49 |
| `TramSubmode` | 51 | 10 |
| `Branding` | 47 | 10 |
| `CoachSubmode` | 45 | 45 |
| `operatingPeriods` | 45 | 45 |
| `operatingDays` | 44 | 44 |
| `IsoCode` | 40 | 1 |
| `serviceLinks` | 37 | 37 |
| `CancellationPossible` | 36 | 1 |
| `ChangeOfTimePossible` | 36 | 1 |
| `GroupOfTariffZones` | 30 | 1 |
| `Address` | 29 | 2 |
| `AddressLine1` | 29 | 2 |
| `Town` | 29 | 2 |
| `notices` | 29 | 29 |
| `GroupOfLines` | 20 | 7 |
| `RechargingAvailable` | 15 | 1 |
| `MetroSubmode` | 14 | 6 |
| `groupsOfLines` | 13 | 7 |
| `typesOfValue` | 11 | 11 |
| `SiteFrame` | 8 | 8 |
| `additionalNetworks` | 8 | 8 |
| `TelecabinSubmode` | 7 | 4 |
| `flexibleStopPlaces` | 7 | 7 |
| `interior` | 6 | 1 |
| `Direction` | 4 | 2 |
| `EndDate` | 2 | 2 |
| `Image` | 2 | 2 |
| `StartDate` | 2 | 2 |
| `Version` | 2 | 2 |
| `directions` | 2 | 2 |
| `versions` | 2 | 2 |
| `AlternativeText` | 1 | 1 |
| `FareFrame` | 1 | 1 |
| `PurposeOfGrouping` | 1 | 1 |
| `alternativeTexts` | 1 | 1 |
| `fareZones` | 1 | 1 |
| `groupsOfStopPlaces` | 1 | 1 |
| `groupsOfTariffZones` | 1 | 1 |
| `parkings` | 1 | 1 |
| `stopPlaces` | 1 | 1 |
| `topographicPlaces` | 1 | 1 |

### Attributes

| Element | Attribute | Occurrences |
|---|---|---|
| `StopPointInJourneyPatternRef` | `ref` | 9039716 |
| `StopPointInJourneyPatternRef` | `version` | 9039716 |
| `TimetabledPassingTime` | `id` | 9039716 |
| `TimetabledPassingTime` | `version` | 9039716 |
| `OperatingDayRef` | `ref` | 2184035 |
| `DatedServiceJourney` | `id` | 2158080 |
| `DatedServiceJourney` | `version` | 2158080 |
| `ServiceJourneyRef` | `ref` | 2158080 |
| `ServiceJourneyRef` | `version` | 2158080 |
| `ScheduledStopPointRef` | `ref` | 974053 |
| `StopPointInJourneyPattern` | `id` | 699082 |
| `StopPointInJourneyPattern` | `order` | 699082 |
| `StopPointInJourneyPattern` | `version` | 699082 |
| `ServiceLinkInJourneyPattern` | `id` | 664902 |
| `ServiceLinkInJourneyPattern` | `order` | 664902 |
| `ServiceLinkInJourneyPattern` | `version` | 664902 |
| `ServiceLinkRef` | `ref` | 664902 |
| `PointOnRoute` | `id` | 569297 |
| `PointOnRoute` | `order` | 569297 |
| `PointOnRoute` | `version` | 569297 |
| `RoutePointRef` | `ref` | 569297 |
| `DayTypeRef` | `ref` | 445187 |
| `JourneyPatternRef` | `ref` | 379540 |
| `JourneyPatternRef` | `version` | 379540 |
| `ServiceJourney` | `id` | 379540 |
| `ServiceJourney` | `version` | 379540 |
| `ScheduledStopPoint` | `id` | 273963 |
| `ScheduledStopPoint` | `version` | 273963 |
| `PassengerStopAssignment` | `id` | 272991 |
| `PassengerStopAssignment` | `order` | 272991 |
| `PassengerStopAssignment` | `version` | 272991 |
| `LineRef` | `ref` | 235708 |
| `LineRef` | `version` | 235708 |
| `FromPointRef` | `ref` | 215307 |
| `ToPointRef` | `ref` | 215307 |
| `QuayRef` | `ref` | 214984 |
| `OperatorRef` | `ref` | 206145 |
| `ToPointRef` | `version` | 186608 |
| `FromPointRef` | `version` | 186145 |
| `ScheduledStopPointRef` | `versionRef` | 159835 |
| `ServiceLink` | `id` | 142509 |
| `ServiceLink` | `version` | 142509 |
| `DayTypeRef` | `version` | 139098 |
| `LinkSequenceProjection` | `id` | 138132 |
| `LinkSequenceProjection` | `version` | 138132 |
| `LineString` | `srsName` | 137824 |
| `TariffZoneRef` | `ref` | 130999 |
| `TariffZoneRef` | `version` | 130353 |
| `ScheduledStopPointRef` | `version` | 114129 |
| `PointProjection` | `id` | 108398 |
| `PointProjection` | `version` | 108398 |
| `RoutePoint` | `id` | 108398 |
| `RoutePoint` | `version` | 108398 |
| `posList` | `srsDimension` | 108213 |
| `ProjectedPointRef` | `ref` | 102680 |
| `ProjectedPointRef` | `version` | 102680 |
| `Quay` | `id` | 101828 |
| `Quay` | `modification` | 101828 |
| `Quay` | `version` | 101828 |
| `QuayRef` | `version` | 101828 |
| `LineString` | `gis:id` | 96314 |
| `posList` | `count` | 89650 |
| `LineString` | `srsDimension` | 89342 |
| `DayTypeAssignment` | `id` | 80756 |
| `DayTypeAssignment` | `order` | 80756 |
| `DayTypeAssignment` | `version` | 80756 |
| `Quay` | `changed` | 74684 |
| `FromJourneyRef` | `ref` | 72798 |
| `ServiceJourneyInterchange` | `id` | 72798 |
| `ServiceJourneyInterchange` | `version` | 72798 |
| `ToJourneyRef` | `ref` | 72798 |
| `ToJourneyRef` | `version` | 72798 |
| `Name` | `lang` | 62970 |
| `AccessibilityAssessment` | `id` | 60577 |
| `AccessibilityAssessment` | `modification` | 60577 |
| `AccessibilityAssessment` | `version` | 60577 |
| `AccessibilityLimitation` | `id` | 60577 |
| `AccessibilityLimitation` | `modification` | 60577 |
| `AccessibilityLimitation` | `version` | 60577 |
| `StopPlaceRef` | `ref` | 58430 |
| `StopPlace` | `created` | 58007 |
| `StopPlace` | `id` | 58007 |
| `StopPlace` | `modification` | 58007 |
| `StopPlace` | `version` | 58007 |
| `StopPlaceRef` | `version` | 58007 |
| `StopPlace` | `changed` | 57982 |
| `TopographicPlaceRef` | `created` | 57936 |
| `TopographicPlaceRef` | `ref` | 57936 |
| `TopographicPlaceRef` | `version` | 57936 |
| `NoticeAssignment` | `id` | 50371 |
| `NoticeAssignment` | `order` | 50371 |
| `NoticeAssignment` | `version` | 50371 |
| `NoticeRef` | `ref` | 50371 |
| `NoticedObjectRef` | `ref` | 50371 |
| `NoticedObjectRef` | `version` | 50371 |
| `DestinationDisplayRef` | `ref` | 37859 |
| `LineString` | `gml:id` | 33306 |
| `JourneyPattern` | `id` | 29062 |
| `JourneyPattern` | `version` | 29062 |
| `RouteRef` | `ref` | 29062 |
| `RouteRef` | `version` | 29062 |
| `OperatingDayRef` | `version` | 25955 |
| `Description` | `lang` | 24737 |
| `placeEquipments` | `id` | 23851 |
| `Route` | `id` | 23832 |
| `Route` | `version` | 23832 |
| `GeneralSign` | `id` | 21120 |
| `GeneralSign` | `modification` | 21120 |
| `GeneralSign` | `version` | 21120 |
| `PrivateCode` | `type` | 18576 |
| `DatedServiceJourneyRef` | `ref` | 17613 |
| `DatedServiceJourneyRef` | `version` | 17613 |
| `DayType` | `id` | 17534 |
| `DayType` | `version` | 17534 |
| `DestinationDisplay` | `id` | 14936 |
| `DestinationDisplay` | `version` | 14936 |
| `ServiceJourney` | `publication` | 12551 |
| `RoutePointRef` | `version` | 11485 |
| `OperatingPeriodRef` | `ref` | 10868 |
| `OperatingPeriodRef` | `version` | 10868 |
| `OperatingPeriod` | `id` | 9730 |
| `OperatingPeriod` | `version` | 9730 |
| `Codespace` | `id` | 9673 |
| `ShelterEquipment` | `id` | 8874 |
| `ShelterEquipment` | `modification` | 8874 |
| `ShelterEquipment` | `version` | 8874 |
| `LineString` | `AA:id` | 8204 |
| `NoticeAssignment` | `modification` | 7893 |
| `NoticeAssignment` | `status` | 7893 |
| `FromJourneyRef` | `version` | 7855 |
| `ProjectToPointRef` | `ref` | 5718 |
| `PublicationDelivery` | `version` | 4819 |
| `PublicationDelivery` | `xmlns` | 4819 |
| `ServiceFrame` | `id` | 4819 |
| `ServiceFrame` | `version` | 4819 |
| `CompositeFrame` | `created` | 4818 |
| `CompositeFrame` | `id` | 4818 |
| `CompositeFrame` | `version` | 4818 |
| `RepresentedByGroupRef` | `ref` | 4749 |
| `TimetableFrame` | `id` | 4749 |
| `TimetableFrame` | `version` | 4749 |
| `AvailabilityCondition` | `id` | 4565 |
| `AvailabilityCondition` | `version` | 4565 |
| `PublicationDelivery` | `xmlns:gis` | 4476 |
| `PublicationDelivery` | `xmlns:siri` | 4476 |
| `Line` | `id` | 4051 |
| `Line` | `version` | 4051 |
| `ParentSiteRef` | `ref` | 3479 |
| `Notice` | `id` | 3405 |
| `Notice` | `version` | 3405 |
| `OperatingDay` | `id` | 3267 |
| `OperatingDay` | `version` | 3267 |
| `ParentSiteRef` | `version` | 2433 |
| `AlternativeName` | `id` | 2197 |
| `AlternativeName` | `modification` | 2197 |
| `AlternativeName` | `version` | 2197 |
| `Polygon` | `ns2:id` | 2129 |
| `BoardingPosition` | `id` | 1999 |
| `BoardingPosition` | `modification` | 1999 |
| `BoardingPosition` | `version` | 1999 |
| `FlexibleLineRef` | `ref` | 1982 |
| `FlexibleLineRef` | `version` | 1982 |
| `Quay` | `created` | 1538 |
| `DestinationDisplayRef` | `version` | 1340 |
| `FlexibleServiceProperties` | `id` | 1339 |
| `FlexibleServiceProperties` | `version` | 1339 |
| `FareZoneRef` | `ref` | 1182 |
| `FlexibleArea` | `id` | 1089 |
| `FlexibleArea` | `version` | 1089 |
| `Polygon` | `srsDimension` | 1089 |
| `Polygon` | `srsName` | 1089 |
| `Parking` | `created` | 1049 |
| `Parking` | `id` | 1049 |
| `Parking` | `modification` | 1049 |
| `Parking` | `version` | 1049 |
| `ParkingCapacity` | `id` | 1039 |
| `ParkingCapacity` | `version` | 1039 |
| `ProjectToPointRef` | `version` | 1002 |
| `FlexibleStopAssignment` | `id` | 973 |
| `FlexibleStopAssignment` | `version` | 973 |
| `FlexibleStopPlace` | `id` | 973 |
| `FlexibleStopPlace` | `version` | 973 |
| `FlexibleStopPlaceRef` | `ref` | 973 |
| `FlexibleStopPlaceRef` | `version` | 973 |
| `PublicationDelivery` | `xmlns:xsi` | 930 |
| `Content` | `lang` | 894 |
| `Parking` | `changed` | 868 |
| `PublicationDelivery` | `xmlns:gml` | 851 |
| `FlexibleLine` | `id` | 698 |
| `FlexibleLine` | `version` | 698 |
| `ParkingProperties` | `id` | 648 |
| `ParkingProperties` | `version` | 648 |
| `AuthorityRef` | `ref` | 590 |
| `CountryRef` | `ref` | 539 |
| `TopographicPlace` | `id` | 539 |
| `TopographicPlace` | `modification` | 539 |
| `TopographicPlace` | `version` | 539 |
| `ParentTopographicPlaceRef` | `ref` | 519 |
| `ParentTopographicPlaceRef` | `version` | 519 |
| `TariffZone` | `id` | 491 |
| `TariffZone` | `modification` | 491 |
| `TariffZone` | `version` | 491 |
| `FareZone` | `changed` | 485 |
| `FareZone` | `id` | 485 |
| `FareZone` | `modification` | 485 |
| `FareZone` | `version` | 485 |
| `TariffZone` | `changed` | 479 |
| `Polygon` | `gis:id` | 475 |
| `FareZone` | `created` | 470 |
| `PublicationDelivery` | `xmlns:ns2` | 442 |
| `PublicationDelivery` | `xmlns:ns3` | 442 |
| `CompositeFrame` | `modification` | 439 |
| `WaitingRoomEquipment` | `id` | 404 |
| `WaitingRoomEquipment` | `modification` | 404 |
| `WaitingRoomEquipment` | `version` | 404 |
| `FromOperatingDayRef` | `ref` | 383 |
| `FromOperatingDayRef` | `version` | 383 |
| `ToOperatingDayRef` | `ref` | 383 |
| `ToOperatingDayRef` | `version` | 383 |
| `TopographicPlace` | `changed` | 373 |
| `NoticedObjectRef` | `xmlns:xsi` | 358 |
| `NoticedObjectRef` | `xsi:type` | 358 |
| `SanitaryEquipment` | `id` | 336 |
| `SanitaryEquipment` | `modification` | 336 |
| `SanitaryEquipment` | `version` | 336 |
| `LineString` | `ns2:id` | 308 |
| `Operator` | `id` | 279 |
| `Operator` | `version` | 279 |
| `ValidBetween` | `id` | 253 |
| `ValidBetween` | `version` | 253 |
| `PublicationDelivery` | `xmlns:AA` | 209 |
| `PublicationDelivery` | `xmlns:netex` | 209 |
| `PublicationDelivery` | `xmlns:xs` | 209 |
| `TicketingEquipment` | `id` | 184 |
| `TicketingEquipment` | `modification` | 184 |
| `TicketingEquipment` | `version` | 184 |
| `ServiceCalendarFrame` | `id` | 166 |
| `ServiceCalendarFrame` | `version` | 166 |
| `TopographicPlace` | `created` | 166 |
| `GroupOfStopPlaces` | `created` | 155 |
| `GroupOfStopPlaces` | `id` | 155 |
| `GroupOfStopPlaces` | `modification` | 155 |
| `GroupOfStopPlaces` | `version` | 155 |
| `PurposeOfGroupingRef` | `ref` | 153 |
| `SiteRef` | `ref` | 126 |
| `AuthorityRef` | `version` | 105 |
| `Network` | `id` | 105 |
| `Network` | `version` | 105 |
| `PublicationDelivery` | `xmlns:xsd` | 105 |
| `journeyInterchanges` | `modificationSet` | 83 |
| `Authority` | `id` | 82 |
| `Authority` | `version` | 82 |
| `GroupOfStopPlaces` | `changed` | 78 |
| `InverseRouteRef` | `ref` | 78 |
| `InverseRouteRef` | `version` | 78 |
| `ResourceFrame` | `id` | 70 |
| `ResourceFrame` | `version` | 70 |
| `BrandingRef` | `ref` | 53 |
| `noticeAssignments` | `modificationSet` | 51 |
| `Branding` | `id` | 47 |
| `Branding` | `version` | 47 |
| `Notice` | `modification` | 46 |
| `Notice` | `status` | 46 |
| `GroupOfTariffZones` | `changed` | 30 |
| `GroupOfTariffZones` | `id` | 30 |
| `GroupOfTariffZones` | `modification` | 30 |
| `GroupOfTariffZones` | `version` | 30 |
| `Network` | `changed` | 30 |
| `TariffZone` | `created` | 25 |
| `PublicationDelivery` | `xmlns:p2` | 22 |
| `GroupOfLines` | `id` | 20 |
| `GroupOfLines` | `version` | 20 |
| `SiteFrame` | `id` | 8 |
| `SiteFrame` | `version` | 8 |
| `BrandingRef` | `version` | 5 |
| `Direction` | `id` | 4 |
| `Direction` | `version` | 4 |
| `Version` | `id` | 2 |
| `Version` | `version` | 2 |
| `AlternativeText` | `id` | 1 |
| `AlternativeText` | `version` | 1 |
| `FareFrame` | `id` | 1 |
| `FareFrame` | `modification` | 1 |
| `FareFrame` | `version` | 1 |
| `PublicationDelivery` | `xsi:schemaLocation` | 1 |
| `PurposeOfGrouping` | `created` | 1 |
| `PurposeOfGrouping` | `id` | 1 |
| `PurposeOfGrouping` | `modification` | 1 |
| `PurposeOfGrouping` | `version` | 1 |
| `RepresentedByGroupRef` | `version` | 1 |
| `ResourceFrame` | `modification` | 1 |
| `ServiceFrame` | `modification` | 1 |
| `SiteFrame` | `modification` | 1 |
| `Text` | `lang` | 1 |
| `notices` | `modificationSet` | 1 |

### Parents and children

| Parent | Child | Occurrences |
|---|---|---|
| `TimetabledPassingTime` | `StopPointInJourneyPatternRef` | 9039716 |
| `passingTimes` | `TimetabledPassingTime` | 9039716 |
| `TimetabledPassingTime` | `DepartureTime` | 8561166 |
| `DatedServiceJourney` | `OperatingDayRef` | 2158080 |
| `DatedServiceJourney` | `ServiceJourneyRef` | 2158080 |
| `vehicleJourneys` | `DatedServiceJourney` | 2158080 |
| `TimetabledPassingTime` | `ArrivalTime` | 726867 |
| `StopPointInJourneyPattern` | `ScheduledStopPointRef` | 699082 |
| `pointsInSequence` | `StopPointInJourneyPattern` | 699082 |
| `ServiceLinkInJourneyPattern` | `ServiceLinkRef` | 664902 |
| `linksInSequence` | `ServiceLinkInJourneyPattern` | 664902 |
| `PointOnRoute` | `RoutePointRef` | 569297 |
| `pointsInSequence` | `PointOnRoute` | 569297 |
| `LineString` | `pos` | 425588 |
| `ServiceJourney` | `JourneyPatternRef` | 379540 |
| `ServiceJourney` | `passingTimes` | 379540 |
| `vehicleJourneys` | `ServiceJourney` | 379540 |
| `ServiceJourney` | `PrivateCode` | 375108 |
| `dayTypes` | `DayTypeRef` | 364431 |
| `ValidBetween` | `FromDate` | 363481 |
| `KeyValue` | `Key` | 339176 |
| `KeyValue` | `Value` | 339176 |
| `keyList` | `KeyValue` | 339176 |
| `scheduledStopPoints` | `ScheduledStopPoint` | 273963 |
| `ServiceJourney` | `dayTypes` | 273281 |
| `PassengerStopAssignment` | `ScheduledStopPointRef` | 272991 |
| `stopAssignments` | `PassengerStopAssignment` | 272991 |
| `TimetabledPassingTime` | `DepartureDayOffset` | 252453 |
| `ScheduledStopPoint` | `Name` | 246587 |
| `PassengerStopAssignment` | `QuayRef` | 214984 |
| `ServiceJourney` | `LineRef` | 212700 |
| `ServiceJourney` | `Name` | 203611 |
| `ServiceJourney` | `OperatorRef` | 201470 |
| `Centroid` | `Location` | 163038 |
| `Location` | `Latitude` | 163038 |
| `Location` | `Longitude` | 163038 |
| `PassengerStopAssignment` | `ValidBetween` | 159835 |
| `ScheduledStopPoint` | `ValidBetween` | 159835 |
| `ServiceLink` | `FromPointRef` | 142509 |
| `ServiceLink` | `ToPointRef` | 142509 |
| `serviceLinks` | `ServiceLink` | 142509 |
| `ServiceLink` | `Distance` | 142219 |
| `LinkSequenceProjection` | `LineString` | 138132 |
| `ServiceLink` | `projections` | 138132 |
| `projections` | `LinkSequenceProjection` | 138132 |
| `tariffZones` | `TariffZoneRef` | 130353 |
| `LineString` | `posList` | 123389 |
| `RoutePoint` | `projections` | 108398 |
| `projections` | `PointProjection` | 108398 |
| `routePoints` | `RoutePoint` | 108398 |
| `PointProjection` | `ProjectedPointRef` | 102680 |
| `Quay` | `Centroid` | 101828 |
| `Quay` | `keyList` | 101828 |
| `quays` | `Quay` | 101828 |
| `Quay` | `PrivateCode` | 88209 |
| `DayTypeAssignment` | `DayTypeRef` | 80756 |
| `dayTypeAssignments` | `DayTypeAssignment` | 80756 |
| `ServiceJourneyInterchange` | `FromJourneyRef` | 72798 |
| `ServiceJourneyInterchange` | `FromPointRef` | 72798 |
| `ServiceJourneyInterchange` | `ToJourneyRef` | 72798 |
| `ServiceJourneyInterchange` | `ToPointRef` | 72798 |
| `journeyInterchanges` | `ServiceJourneyInterchange` | 72798 |
| `ServiceJourneyInterchange` | `Guaranteed` | 71779 |
| `StopPointInJourneyPattern` | `IsWaitPoint` | 64952 |
| `Quay` | `PublicCode` | 64037 |
| `AccessibilityAssessment` | `MobilityImpairedAccess` | 60577 |
| `AccessibilityAssessment` | `limitations` | 60577 |
| `AccessibilityLimitation` | `StepFreeAccess` | 60577 |
| `AccessibilityLimitation` | `WheelchairAccess` | 60577 |
| `limitations` | `AccessibilityLimitation` | 60577 |
| `ServiceJourneyInterchange` | `MaximumWaitTime` | 59982 |
| `PassengerStopAssignment` | `StopPlaceRef` | 58007 |
| `StopPlace` | `Centroid` | 58007 |
| `StopPlace` | `Name` | 58007 |
| `StopPlace` | `keyList` | 58007 |
| `stopPlaces` | `StopPlace` | 58007 |
| `StopPlace` | `TopographicPlaceRef` | 57936 |
| `StopPlace` | `Weighting` | 57885 |
| `StopPlace` | `tariffZones` | 57280 |
| `StopPlace` | `StopPlaceType` | 56907 |
| `StopPlace` | `TransportMode` | 56906 |
| `StopPlace` | `quays` | 56906 |
| `Quay` | `CompassBearing` | 56788 |
| `StopPlace` | `AccessibilityAssessment` | 56745 |
| `StopPointInJourneyPattern` | `ForBoarding` | 55725 |
| `StopPlace` | `ValidBetween` | 55577 |
| `NoticeAssignment` | `NoticeRef` | 50371 |
| `NoticeAssignment` | `NoticedObjectRef` | 50371 |
| `noticeAssignments` | `NoticeAssignment` | 50371 |
| `StopPointInJourneyPattern` | `ForAlighting` | 45149 |
| `DayTypeAssignment` | `Date` | 43933 |
| `AccessibilityLimitation` | `AudibleSignalsAvailable` | 42055 |
| `AccessibilityLimitation` | `EscalatorFreeAccess` | 42055 |
| `AccessibilityLimitation` | `LiftFreeAccess` | 42055 |
| `ServiceJourney` | `TransportMode` | 36822 |
| `StopPointInJourneyPattern` | `DestinationDisplayRef` | 36760 |
| `ServiceJourney` | `TransportSubmode` | 33267 |
| `StopPlace` | `PrivateCode` | 32892 |
| `TransportSubmode` | `RailSubmode` | 30945 |
| `TimetabledPassingTime` | `ArrivalDayOffset` | 29895 |
| `JourneyPattern` | `Name` | 29062 |
| `JourneyPattern` | `RouteRef` | 29062 |
| `JourneyPattern` | `pointsInSequence` | 29062 |
| `journeyPatterns` | `JourneyPattern` | 29062 |
| `JourneyPattern` | `linksInSequence` | 27840 |
| `DayTypeAssignment` | `OperatingDayRef` | 25955 |
| `ServiceJourneyInterchange` | `StaySeated` | 25545 |
| `Route` | `Name` | 23832 |
| `Route` | `pointsInSequence` | 23832 |
| `routes` | `Route` | 23832 |
| `Route` | `LineRef` | 23008 |
| `Route` | `DirectionType` | 22573 |
| `Quay` | `placeEquipments` | 22308 |
| `StopPlace` | `Description` | 21718 |
| `DayTypeAssignment` | `isAvailable` | 21517 |
| `placeEquipments` | `GeneralSign` | 21120 |
| `ScheduledStopPoint` | `TimingPointStatus` | 20741 |
| `GeneralSign` | `PrivateCode` | 19788 |
| `GeneralSign` | `SignContentType` | 19788 |
| `DatedServiceJourney` | `DatedServiceJourneyRef` | 17613 |
| `dayTypes` | `DayType` | 17534 |
| `DestinationDisplay` | `FrontText` | 14936 |
| `destinationDisplays` | `DestinationDisplay` | 14936 |
| `Route` | `ShortName` | 13348 |
| `DayTypeAssignment` | `OperatingPeriodRef` | 10868 |
| `ServiceJourney` | `keyList` | 10498 |
| `BookingContact` | `Phone` | 10370 |
| `BookingContact` | `Url` | 10297 |
| `DatedServiceJourney` | `ServiceAlteration` | 10040 |
| `StopPointInJourneyPattern` | `BookingArrangements` | 9932 |
| `BookingArrangements` | `BookingContact` | 9930 |
| `operatingPeriods` | `OperatingPeriod` | 9730 |
| `Codespace` | `Xmlns` | 9673 |
| `Codespace` | `XmlnsUrl` | 9673 |
| `codespaces` | `Codespace` | 9673 |
| `OperatingPeriod` | `FromDate` | 9347 |
| `OperatingPeriod` | `ToDate` | 9347 |
| `BookingArrangements` | `BookingMethods` | 8951 |
| `PropertyOfDay` | `DaysOfWeek` | 8897 |
| `properties` | `PropertyOfDay` | 8897 |
| `placeEquipments` | `ShelterEquipment` | 8874 |
| `ShelterEquipment` | `Enclosed` | 8843 |
| `BookingArrangements` | `MinimumBookingPeriod` | 8701 |
| `BookingArrangements` | `BookWhen` | 8246 |
| `NoticeAssignment` | `PublicityChannel` | 7893 |
| `TransportSubmode` | `BusSubmode` | 6503 |
| `DayType` | `properties` | 6191 |
| `ShelterEquipment` | `Seats` | 6068 |
| `ShelterEquipment` | `StepFree` | 6067 |
| `DestinationDisplay` | `Name` | 5770 |
| `PointProjection` | `ProjectToPointRef` | 5718 |
| `` | `PublicationDelivery` | 4819 |
| `DefaultLocale` | `DefaultLanguage` | 4819 |
| `DefaultLocale` | `TimeZone` | 4819 |
| `FrameDefaults` | `DefaultLocale` | 4819 |
| `PublicationDelivery` | `ParticipantRef` | 4819 |
| `PublicationDelivery` | `PublicationTimestamp` | 4819 |
| `PublicationDelivery` | `dataObjects` | 4819 |
| `CompositeFrame` | `FrameDefaults` | 4818 |
| `CompositeFrame` | `codespaces` | 4818 |
| `CompositeFrame` | `frames` | 4818 |
| `CompositeFrame` | `validityConditions` | 4818 |
| `dataObjects` | `CompositeFrame` | 4818 |
| `frames` | `ServiceFrame` | 4818 |
| `ServiceFrame` | `journeyPatterns` | 4749 |
| `ServiceFrame` | `lines` | 4749 |
| `ServiceFrame` | `routes` | 4749 |
| `TimetableFrame` | `vehicleJourneys` | 4749 |
| `frames` | `TimetableFrame` | 4749 |
| `AccessibilityLimitation` | `VisualSignsAvailable` | 4730 |
| `AvailabilityCondition` | `FromDate` | 4565 |
| `validityConditions` | `AvailabilityCondition` | 4565 |
| `AvailabilityCondition` | `ToDate` | 4183 |
| `ServiceJourneyInterchange` | `Advertised` | 4153 |
| `ServiceJourneyInterchange` | `Planned` | 4153 |
| `Line` | `Name` | 4051 |
| `Line` | `RepresentedByGroupRef` | 4051 |
| `Line` | `TransportMode` | 4051 |
| `lines` | `Line` | 4051 |
| `PublicationDelivery` | `Description` | 4048 |
| `Line` | `TransportSubmode` | 4010 |
| `Line` | `OperatorRef` | 3977 |
| `Line` | `PublicCode` | 3936 |
| `Quay` | `AccessibilityAssessment` | 3832 |
| `StopPointInJourneyPattern` | `RequestStop` | 3544 |
| `Notice` | `Text` | 3405 |
| `notices` | `Notice` | 3405 |
| `TimetabledPassingTime` | `EarliestDepartureTime` | 3276 |
| `TimetabledPassingTime` | `LatestArrivalTime` | 3276 |
| `OperatingDay` | `CalendarDate` | 3267 |
| `operatingDays` | `OperatingDay` | 3267 |
| `DestinationDisplay` | `SideText` | 3106 |
| `Line` | `PrivateCode` | 2904 |
| `Quay` | `Description` | 2864 |
| `LinearRing` | `posList` | 2610 |
| `Polygon` | `exterior` | 2604 |
| `exterior` | `LinearRing` | 2604 |
| `StopPlace` | `ParentSiteRef` | 2430 |
| `ServiceJourney` | `PublicCode` | 2400 |
| `AlternativeName` | `Name` | 2197 |
| `AlternativeName` | `NameType` | 2197 |
| `alternativeNames` | `AlternativeName` | 2197 |
| `BoardingPosition` | `Centroid` | 1999 |
| `BoardingPosition` | `PublicCode` | 1999 |
| `boardingPositions` | `BoardingPosition` | 1999 |
| `StopPlace` | `placeEquipments` | 1543 |
| `FlexibleServiceProperties` | `BookingMethods` | 1339 |
| `ServiceJourney` | `FlexibleServiceProperties` | 1339 |
| `BookingContact` | `ContactPerson` | 1314 |
| `TimetableFrame` | `noticeAssignments` | 1202 |
| `neighbours` | `FareZoneRef` | 1182 |
| `BookingContact` | `FurtherDetails` | 1176 |
| `ServiceJourney` | `FlexibleLineRef` | 1158 |
| `StopPlace` | `alternativeNames` | 1151 |
| `Via` | `DestinationDisplayRef` | 1099 |
| `vias` | `Via` | 1099 |
| `FlexibleArea` | `Polygon` | 1089 |
| `areas` | `FlexibleArea` | 1089 |
| `TimetableFrame` | `journeyInterchanges` | 1080 |
| `DestinationDisplay` | `vias` | 1062 |
| `Parking` | `Centroid` | 1049 |
| `Parking` | `Name` | 1049 |
| `Parking` | `ParentSiteRef` | 1049 |
| `Parking` | `ParkingPaymentProcess` | 1049 |
| `Parking` | `ParkingVehicleTypes` | 1049 |
| `parkings` | `Parking` | 1049 |
| `ParkingCapacity` | `ParkingUserType` | 1039 |
| `spaces` | `ParkingCapacity` | 1039 |
| `members` | `ScheduledStopPointRef` | 1007 |
| `BookingContact` | `Email` | 982 |
| `FlexibleServiceProperties` | `BookWhen` | 979 |
| `FlexibleServiceProperties` | `LatestBookingTime` | 979 |
| `FlexibleStopAssignment` | `FlexibleStopPlaceRef` | 973 |
| `FlexibleStopAssignment` | `ScheduledStopPointRef` | 973 |
| `FlexibleStopPlace` | `Name` | 973 |
| `FlexibleStopPlace` | `TransportMode` | 973 |
| `FlexibleStopPlace` | `areas` | 973 |
| `flexibleStopPlaces` | `FlexibleStopPlace` | 973 |
| `stopAssignments` | `FlexibleStopAssignment` | 973 |
| `FlexibleStopPlace` | `keyList` | 972 |
| `Parking` | `keyList` | 929 |
| `ParkingCapacity` | `NumberOfSpaces` | 895 |
| `GeneralSign` | `Content` | 894 |
| `FlexibleServiceProperties` | `BookingAccess` | 885 |
| `Route` | `FlexibleLineRef` | 824 |
| `Notice` | `PublicCode` | 820 |
| `Parking` | `TotalCapacity` | 772 |
| `ServiceJourney` | `DepartureTime` | 745 |
| `BookingArrangements` | `BookingNote` | 732 |
| `FlexibleLine` | `FlexibleLineType` | 698 |
| `FlexibleLine` | `Name` | 698 |
| `FlexibleLine` | `OperatorRef` | 698 |
| `FlexibleLine` | `RepresentedByGroupRef` | 698 |
| `FlexibleLine` | `TransportMode` | 698 |
| `FlexibleLine` | `TransportSubmode` | 698 |
| `lines` | `FlexibleLine` | 698 |
| `FlexibleLine` | `PublicCode` | 696 |
| `Line` | `Presentation` | 696 |
| `Presentation` | `Colour` | 696 |
| `Presentation` | `TextColour` | 696 |
| `Parking` | `parkingProperties` | 648 |
| `ParkingProperties` | `ParkingUserTypes` | 648 |
| `ParkingProperties` | `spaces` | 648 |
| `parkingProperties` | `ParkingProperties` | 648 |
| `members` | `TariffZoneRef` | 646 |
| `StopPlace` | `BusSubmode` | 620 |
| `FlexibleArea` | `keyList` | 601 |
| `Descriptor` | `Name` | 539 |
| `TopographicPlace` | `CountryRef` | 539 |
| `TopographicPlace` | `Descriptor` | 539 |
| `TopographicPlace` | `Polygon` | 539 |
| `TopographicPlace` | `TopographicPlaceType` | 539 |
| `TopographicPlace` | `ValidBetween` | 539 |
| `topographicPlaces` | `TopographicPlace` | 539 |
| `TopographicPlace` | `ParentTopographicPlaceRef` | 519 |
| `Via` | `ViaType` | 515 |
| `TariffZone` | `Name` | 491 |
| `TariffZone` | `Polygon` | 491 |
| `TariffZone` | `ValidBetween` | 491 |
| `tariffZones` | `TariffZone` | 491 |
| `FareZone` | `AuthorityRef` | 485 |
| `FareZone` | `Name` | 485 |
| `FareZone` | `Polygon` | 485 |
| `FareZone` | `PrivateCode` | 485 |
| `FareZone` | `ScopingMethod` | 485 |
| `FareZone` | `ValidBetween` | 485 |
| `FareZone` | `ZoneTopology` | 485 |
| `FareZone` | `keyList` | 485 |
| `fareZones` | `FareZone` | 485 |
| `FareZone` | `neighbours` | 480 |
| `RoutePoint` | `Name` | 455 |
| `FlexibleServiceProperties` | `BookingContact` | 449 |
| `Line` | `Monitored` | 437 |
| `DestinationDisplay` | `variants` | 436 |
| `DestinationDisplayVariant` | `DestinationDisplayVariantMediaType` | 436 |
| `DestinationDisplayVariant` | `FrontText` | 436 |
| `variants` | `DestinationDisplayVariant` | 436 |
| `FlexibleLine` | `PrivateCode` | 433 |
| `members` | `StopPlaceRef` | 423 |
| `WaitingRoomEquipment` | `Heated` | 404 |
| `WaitingRoomEquipment` | `Sanitary` | 404 |
| `WaitingRoomEquipment` | `Seats` | 404 |
| `WaitingRoomEquipment` | `StepFree` | 404 |
| `placeEquipments` | `WaitingRoomEquipment` | 404 |
| `OperatingPeriod` | `FromOperatingDayRef` | 383 |
| `OperatingPeriod` | `ToOperatingDayRef` | 383 |
| `FlexibleLine` | `BookingContact` | 364 |
| `FlexibleLine` | `BookingMethods` | 364 |
| `FlexibleServiceProperties` | `MinimumBookingPeriod` | 358 |
| `FlexibleLine` | `BookingNote` | 348 |
| `SanitaryEquipment` | `PaymentMethods` | 336 |
| `SanitaryEquipment` | `SanitaryFacilityList` | 336 |
| `placeEquipments` | `SanitaryEquipment` | 336 |
| `FlexibleLine` | `MinimumBookingPeriod` | 333 |
| `SanitaryEquipment` | `Gender` | 333 |
| `ParkingCapacity` | `NumberOfSpacesWithRechargePoint` | 306 |
| `ContactDetails` | `Url` | 292 |
| `SanitaryEquipment` | `NumberOfToilets` | 290 |
| `Operator` | `Name` | 279 |
| `Quay` | `boardingPositions` | 279 |
| `organisations` | `Operator` | 279 |
| `Parking` | `ParkingLayout` | 271 |
| `FlexibleLine` | `BookingAccess` | 266 |
| `FlexibleLine` | `BuyWhen` | 266 |
| `FlexibleServiceProperties` | `BookingNote` | 265 |
| `TransportSubmode` | `WaterSubmode` | 258 |
| `ValidBetween` | `ToDate` | 255 |
| `validityConditions` | `ValidBetween` | 253 |
| `ServiceFrame` | `noticeAssignments` | 249 |
| `CustomerServiceContactDetails` | `Url` | 240 |
| `Operator` | `CustomerServiceContactDetails` | 240 |
| `Operator` | `ContactDetails` | 239 |
| `Operator` | `LegalName` | 235 |
| `ServiceJourney` | `Description` | 220 |
| `Parking` | `Covered` | 201 |
| `ServiceFrame` | `routePoints` | 197 |
| `TicketingEquipment` | `PaymentMethods` | 184 |
| `TicketingEquipment` | `ScopeOfTicketsAvailable` | 184 |
| `TicketingEquipment` | `TicketMachines` | 184 |
| `TicketingEquipment` | `TicketOffice` | 184 |
| `TicketingEquipment` | `TicketTypesAvailable` | 184 |
| `TicketingEquipment` | `TicketingFacilityList` | 184 |
| `TicketingEquipment` | `VehicleModes` | 184 |
| `placeEquipments` | `TicketingEquipment` | 184 |
| `TicketingEquipment` | `NumberOfMachines` | 183 |
| `Operator` | `CompanyNumber` | 182 |
| `ServiceFrame` | `destinationDisplays` | 166 |
| `frames` | `ServiceCalendarFrame` | 166 |
| `ContactDetails` | `Phone` | 164 |
| `Operator` | `OrganisationType` | 161 |
| `GroupOfStopPlaces` | `Centroid` | 155 |
| `GroupOfStopPlaces` | `Description` | 155 |
| `GroupOfStopPlaces` | `Name` | 155 |
| `groupsOfStopPlaces` | `GroupOfStopPlaces` | 155 |
| `GroupOfStopPlaces` | `members` | 154 |
| `GroupOfStopPlaces` | `PurposeOfGroupingRef` | 153 |
| `ServiceCalendarFrame` | `dayTypeAssignments` | 150 |
| `ServiceCalendarFrame` | `dayTypes` | 150 |
| `StopPlace` | `adjacentSites` | 126 |
| `adjacentSites` | `SiteRef` | 126 |
| `CustomerServiceContactDetails` | `Phone` | 113 |
| `TransportSubmode` | `TaxiSubmode` | 109 |
| `StopPlace` | `WaterSubmode` | 108 |
| `CustomerServiceContactDetails` | `Email` | 105 |
| `Network` | `AuthorityRef` | 105 |
| `Network` | `Name` | 105 |
| `TransportSubmode` | `AirSubmode` | 98 |
| `FlexibleStopPlace` | `PrivateCode` | 94 |
| `BoardingPosition` | `keyList` | 92 |
| `BookingArrangements` | `LatestBookingTime` | 86 |
| `TariffZone` | `keyList` | 86 |
| `Parking` | `Description` | 84 |
| `Authority` | `ContactDetails` | 82 |
| `Authority` | `Name` | 82 |
| `organisations` | `Authority` | 82 |
| `GroupOfStopPlaces` | `keyList` | 78 |
| `Route` | `InverseRouteRef` | 78 |
| `Authority` | `LegalName` | 75 |
| `ServiceFrame` | `scheduledStopPoints` | 70 |
| `ServiceFrame` | `stopAssignments` | 70 |
| `ResourceFrame` | `organisations` | 69 |
| `ServiceFrame` | `Network` | 69 |
| `frames` | `ResourceFrame` | 69 |
| `FareZone` | `members` | 64 |
| `ContactDetails` | `Email` | 56 |
| `Parking` | `ValidBetween` | 48 |
| `Branding` | `Name` | 47 |
| `ScheduledStopPoint` | `ShortName` | 47 |
| `typesOfValue` | `Branding` | 47 |
| `Authority` | `OrganisationType` | 46 |
| `Line` | `BrandingRef` | 45 |
| `ServiceCalendarFrame` | `operatingPeriods` | 45 |
| `TransportSubmode` | `CoachSubmode` | 45 |
| `FlexibleServiceProperties` | `BuyWhen` | 44 |
| `ServiceCalendarFrame` | `operatingDays` | 44 |
| `Line` | `ShortName` | 43 |
| `Authority` | `CompanyNumber` | 42 |
| `StopPlace` | `TramSubmode` | 42 |
| `TopographicPlace` | `IsoCode` | 40 |
| `ServiceFrame` | `serviceLinks` | 37 |
| `FlexibleServiceProperties` | `CancellationPossible` | 36 |
| `FlexibleServiceProperties` | `ChangeOfTimePossible` | 36 |
| `additionalNetworks` | `Network` | 36 |
| `FlexibleLine` | `BookWhen` | 31 |
| `GroupOfTariffZones` | `Name` | 30 |
| `GroupOfTariffZones` | `ValidBetween` | 30 |
| `GroupOfTariffZones` | `members` | 30 |
| `groupsOfTariffZones` | `GroupOfTariffZones` | 30 |
| `Address` | `AddressLine1` | 29 |
| `Address` | `Town` | 29 |
| `FlexibleLine` | `LatestBookingTime` | 29 |
| `Operator` | `Address` | 29 |
| `ServiceFrame` | `notices` | 29 |
| `BookingArrangements` | `BuyWhen` | 26 |
| `StopPlace` | `RailSubmode` | 25 |
| `Branding` | `Url` | 22 |
| `GroupOfLines` | `Name` | 20 |
| `groupsOfLines` | `GroupOfLines` | 20 |
| `Quay` | `Name` | 17 |
| `Parking` | `RechargingAvailable` | 15 |
| `Network` | `groupsOfLines` | 13 |
| `ResourceFrame` | `typesOfValue` | 11 |
| `Line` | `Description` | 10 |
| `JourneyPattern` | `DirectionType` | 9 |
| `StopPlace` | `MetroSubmode` | 9 |
| `TransportSubmode` | `TramSubmode` | 9 |
| `JourneyPattern` | `Description` | 8 |
| `JourneyPattern` | `PrivateCode` | 8 |
| `ServiceFrame` | `additionalNetworks` | 8 |
| `SiteFrame` | `flexibleStopPlaces` | 7 |
| `frames` | `SiteFrame` | 7 |
| `Polygon` | `interior` | 6 |
| `interior` | `LinearRing` | 6 |
| `FlexibleStopPlace` | `Description` | 5 |
| `GroupOfLines` | `TransportMode` | 5 |
| `Operator` | `BrandingRef` | 5 |
| `TransportSubmode` | `MetroSubmode` | 5 |
| `Direction` | `Description` | 4 |
| `Direction` | `Name` | 4 |
| `StopPlace` | `TelecabinSubmode` | 4 |
| `directions` | `Direction` | 4 |
| `FlexibleLine` | `BrandingRef` | 3 |
| `Network` | `Description` | 3 |
| `StopPlace` | `AirSubmode` | 3 |
| `TransportSubmode` | `TelecabinSubmode` | 3 |
| `BookingArrangements` | `BookingAccess` | 2 |
| `Branding` | `Image` | 2 |
| `CompositeFrame` | `versions` | 2 |
| `FlexibleLine` | `Description` | 2 |
| `GroupOfTariffZones` | `PrivateCode` | 2 |
| `ServiceFrame` | `directions` | 2 |
| `Version` | `EndDate` | 2 |
| `Version` | `StartDate` | 2 |
| `versions` | `Version` | 2 |
| `AlternativeText` | `Text` | 1 |
| `Authority` | `keyList` | 1 |
| `Branding` | `Description` | 1 |
| `Branding` | `ShortName` | 1 |
| `FareFrame` | `fareZones` | 1 |
| `Notice` | `alternativeTexts` | 1 |
| `Operator` | `keyList` | 1 |
| `PurposeOfGrouping` | `Description` | 1 |
| `PurposeOfGrouping` | `Name` | 1 |
| `PurposeOfGrouping` | `keyList` | 1 |
| `SiteFrame` | `Description` | 1 |
| `SiteFrame` | `FrameDefaults` | 1 |
| `SiteFrame` | `groupsOfStopPlaces` | 1 |
| `SiteFrame` | `groupsOfTariffZones` | 1 |
| `SiteFrame` | `parkings` | 1 |
| `SiteFrame` | `stopPlaces` | 1 |
| `SiteFrame` | `tariffZones` | 1 |
| `SiteFrame` | `topographicPlaces` | 1 |
| `StopPlace` | `PublicCode` | 1 |
| `alternativeTexts` | `AlternativeText` | 1 |
| `dataObjects` | `FareFrame` | 1 |
| `dataObjects` | `ResourceFrame` | 1 |
| `dataObjects` | `ServiceFrame` | 1 |
| `dataObjects` | `SiteFrame` | 1 |
| `typesOfValue` | `PurposeOfGrouping` | 1 |
