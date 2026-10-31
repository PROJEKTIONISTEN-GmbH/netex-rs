# The French profile (NeTEx France)

France's national access point, [transport.data.gouv.fr](https://transport.data.gouv.fr/),
publishes timetables in the French NeTEx profile, "NeTEx – Profil France"
([normes.transport.data.gouv.fr](https://normes.transport.data.gouv.fr/normes/netex/),
version 2.4): common elements, stops, networks, timetables, accessibility,
fares and parking, each a part of its own. The profile writes every part of a
delivery in general frames typed by the part (`FR:TypeOfFrame:NETEX_ARRET`,
`…NETEX_HORAIRE`, …) and its identifiers in the form
`FR:<INSEE code>:<type>:<code>:<codespace>`, `LOC` for a code the producer's
system assigns. This page describes five deliveries as fetched on 2026-10-31:
what they state, how `netex-rs` reads it, what was read before and after the
crate learned the profile, and the inventory of one of them with
`examples/inventory`. Nothing of the deliveries is in this repository: the
crate's own example of the profile is `tests/fixtures/own/fr/`.

## The deliveries

| | Fluo Grand Est, Bas-Rhin | Naolib (Nantes) | TBM (Bordeaux) | Normandy, aggregate | SNCF (TGV, Intercités, TER) |
|---|---|---|---|---|---|
| Licence | Licence Ouverte 2.0 | Licence Ouverte 2.0 | Licence Ouverte 1.0 | Licence Ouverte 2.0 | ODbL |
| Archive | 0.7 MB | 0.3 MB of stops and 13.9 MB of timetable, two archives | 24.0 MB | 29.6 MB, an archive of 32 archives, one per network | 32.1 MB |
| Documents, inflated | 53, 16.5 MB | 1 + 100, 648 MB | 206, 706 MB | 686, 827 MB | 1, 647 MB (and an 88 MB log of the conversion) |
| `version` | `1.10:FR-NETEX-2.2` | `1.1:FR-NETEX-2.2`, `1.1:FR-NETEX_RESEAU-2.2` | none | `1.10:FR-NETEX-2.3` | `1.09` (HaCon, "NeTEx Export, Revision: 2.24") |
| Layout | the stops, the calendar, the network and one document per line, each a composite frame `NETEX_FRANCE` of general frames | the stops in a general frame `NETEX_ARRET`; a common and a calendar document; one composite frame per line of general frames `NETEX_LIGNE` and `NETEX_HORAIRE` | one document per line of specific frames — resource, site, service, calendar, timetable, vehicle schedule —, each typed by the profile, and general frames of link projections and of fares | as Fluo, per network | one composite frame of the specific frames and a general frame of journey parts; no `TypeOfFrameRef` |
| Days of the journeys | 2026-10-30 to 2027-08-29 | 2026-10-30 to 2027-01-30 | 2026-10-31 to 2027-07-02 | 2026-03-26 to 2028-10-16 | 2026-10-30 to 2027-03-29 |

The figures of the Licence Ouverte deliveries are counted on the publications of
Région Grand Est (Fluo), Nantes Métropole (Naolib), Bordeaux Métropole (TBM) and
Région Normandie (the aggregate) at transport.data.gouv.fr. SNCF's delivery is
under the ODbL; it was read to count, and only counts of it are given here.

## What they state, and how the crate reads it

**Frames.** Fluo, Naolib and Normandy write everything in general frames: 103 in
Fluo's 53 documents, 198 in Naolib's, 1 276 in Normandy's. The crate types a
general frame since this change (`model::GeneralFrame`): its `members` hold every
kind of entity the specific frames hold, read in document order, and the lean
stream yields them as it yields the same entities of a specific frame
(`stream::FrameKind::General`). TBM writes the specific frames and types each by
the profile (`FR:TypeOfFrame:NETEX_CALENDRIER:`, …); SNCF writes the specific
frames and puts its journey parts (`JourneyPart`, the parts a train is coupled
from) in a general frame (`docs/INTERPRETATION.md` § 23).

**Identifiers.** Stops in the profile's form: `FR:67067:LMO:C193109:CG67`, a stop
place (*lieu d'arrêt monomodal*) of Fluo's in the commune 67067, and
`FR:67067:ZE:5674A:CG67`, one of its quays (*zone d'embarquement*); the other
objects in the producer's codespace (`CG67:Line:257:LOC`,
`FR_NAOLIB:ServiceJourney:…:LOC`, `TBM:ServiceJourney:…`). A reference into another
document names the version by `versionRef`, as the profile asks ("Éléments
communs", 7.5); the index resolves it as it resolves any reference. SNCF's stop
places carry a UUID (`FR::LMO:<UUID>:`), its stop points the eight digits of the
UIC station code.

**Stops.** Fluo, Naolib and Normandy define each quay on its own, beside its stop
place, which names it among its quays by a `QuayRef`; the quay names the stop
place back by its `SiteRef`. 1 035 quays in Fluo, 2 939 in Naolib, 12 604 in
Normandy, each named by exactly one stop place, its `SiteRef` naming the same.
Since this change a quay so defined is a quay of the stop place naming it, else
of the one its `SiteRef` names (§ 24); TBM nests its quays in their stop places;
SNCF states stop places without quays and assigns each of its 3 425 stop points
to one.

**Lines, routes and patterns.** Each journey pattern names its route, the route
its line (§ 22 reads the line through it); a line names its operator. Lines
carry a `Presentation` and, in Fluo, Naolib and Normandy, an
`AccessibilityAssessment`, both carried through untyped.

**Journeys and passing times.** Fluo, Naolib and Normandy write the passing
times of a journey without the point of the pattern each is at, in the order of
the points: 1 771, 124 908 and 85 356 journeys, each with as many passing times
as its pattern has points. NeTEx 1.x allows it, 2.0 does not. Since this change
a passing time naming no point is at the point of its position (§ 25). TBM and
SNCF name the point.

**Calendar.** Three forms (§ 26):

- Fluo and Normandy: a day type with a validity and days of the week, and an
  assignment of each date it runs on — the dates decide, the properties
  describe them.
- TBM: a day type with a validity and days of the week, its assignments the
  exceptions; 147 of its 283 day types have none, 72 only take days away, 62 add
  a day on another day of the week (a holiday run on a Sunday's timetable).
- Naolib: day types of days of the week, assigned plain operating periods —
  from a first to a last day, without day bits — and single dates.

SNCF gives each of its 8 449 day types one UIC operating period with a bit per
day, named by an `OperatingPeriodRef` (`Concern::ResolvedAsOtherClass`, 8 449
times). Since this change the index reads days of the week where they decide
the days: a plain period gives the days of the week its day type names, and a
day type with a validity none of whose assignments gives it a day of its own
runs on its validity's days of the week, the assignments its exceptions. A day
type of the first form gives a day of its own; one of the second does not.

**Walks.** Fluo states 826 walks between quays, Normandy 16 600 — all of Fluo's
and 16 565 of Normandy's with a distance and no duration. The index holds a walk
by its duration and chooses no walking speed: each is a finding
(`Concern::WalkWithoutDuration`, § 27), and Normandy's 35 walks with a duration
are its 70 transfers, both ways.

**Accessibility.** Quays, stop places and journeys state an
`AccessibilityAssessment`; the model reads its `MobilityImpairedAccess` and
`limitations` and carries its `suitabilities` (a need, such as
`wheelchair`, and whether it is met) through untyped, as it does a line's
assessment. The index does not hold accessibility; netex2gtfs reads it from the
model where it converts.

**What stays untyped**, carried through as written: postal addresses of stops,
the kind of place of a stop place (`placeTypes`, `monomodalStopPlace`), the
Lambert-93 position written beside latitude and longitude (`gml:pos`),
suitabilities, line presentations, networks and groups of lines, route links,
TBM's link projections and vehicle schedules, fares, SNCF's journey parts,
coupled journeys and its service facilities. None of it is a stop, a line, a
journey, a passing time or a day.

## Read before and after

The index of each delivery read with netex-rs 0.7.2 and with this change
(`netex validate`, `netex stats`); untyped, the share of the delivery's elements
inside fragments the typed model carries through.

| | Fluo | Naolib | TBM | Normandy | SNCF |
|---|---|---|---|---|---|
| Untyped, before → after | 99.66 % → 7.23 % | 99.99 % → 0.76 % | 8.48 % → 8.47 % | 99.91 % → 3.51 % | 10.61 % → 10.61 % |
| Stops (stop places and quays) | 0 → 1 555 | 0 → 4 100 | 6 118 | 0 → 25 095 | 3 425 |
| Stop points | 0 → 1 035 | 0 → 4 417 | 3 939 | 0 → 12 570 | 3 425 |
| Lines | 0 → 50 | 0 → 98 | 202 | 0 → 590 | 740 |
| Journeys | 0 → 1 771 | 0 → 124 908 | 39 749 | 0 → 85 356 | 60 142 |
| … of them without a day | – | – | 32 331 → 0 | – | 0 |
| Passing times | 0 → 27 800 | 0 → 3 031 065 | 1 579 489 | 0 → 1 890 835 | 523 621 |
| Journey days (`netex stats`) | 0 → 209 362 | 0 → 682 559 | 11 846 → 555 768 | 0 → 2 971 836 | 1 276 176 |
| Transfers | 0 | 0 | 0 | 0 → 70 | 0 |
| Findings after | 826 walks without a duration | 469 definitions nothing refers to | 221 definitions nothing refers to | 16 565 walks without a duration, 67 definitions nothing refers to | 8 449 references resolved as another class, 28 371 definitions nothing refers to |

References that do not resolve: none in any of the five, before or after.

## Checked against the producers' GTFS

Fluo, Naolib and TBM publish GTFS feeds beside their NeTEx, made from the same
data. `netex convert --to gtfs` of each NeTEx delivery against the producer's
own feed of the same day:

| | Trips | Trip days | Each trip's days and stop times |
|---|---|---|---|
| TBM | 39 749 = 39 749 | 555 768 = 555 768 | the same, trip for trip by identifier |
| Naolib | 124 908 = 124 908 | 682 559 = 682 559 | the same, trip for trip by identifier |
| Fluo | 1 771 = 1 771 | 209 362 = 209 362 within 2026-10-30 to 2027-08-29 | the same, as sets of trips (the feeds number trips otherwise) |

Fluo's GTFS also holds the days before 2026-10-30 its NeTEx no longer states.

## Converting to GTFS

`netex convert --to gtfs` converts each delivery: Fluo and Normandy name their
time zone (`Europe/Paris`) in their frame defaults; TBM and Naolib name none,
SNCF only its offsets from UTC, and France has more than one time zone, so the
converter takes none from the profile: `--time-zone Europe/Paris` gives it.
Fluo gives 1 771 trips with 27 800 stop times, Normandy 85 356 trips with
1 890 835 stop times, its trains named by the train numbers its general frames
hold, SNCF 60 142 trips with 523 621 stop times. Not carried into the feed, and said
so in the report: networks and groups of lines, route links, TBM's link
projections and fares, SNCF's journey parts.

## One document of 647 MB

SNCF's delivery is one document of 647 MB. The crate reads a document from its
text in memory — inflated from the archive into one buffer — and walks it once;
the lean stream keeps nothing of it but what the caller takes, and the index
what it holds.

| Reading | Time | Peak memory |
|---|---|---|
| lean, into the index (`netex stats`) | 6.6 to 9.0 s | 0.69 GiB |
| lean, into the index with the checking report (`netex validate`) | 10 to 14 s | 0.68 GiB |
| into the full typed model (`examples/inventory`) | 12 s | 2.47 GiB |

The document's text is 0.60 GiB of the 0.69: what the lean read keeps beside it
is the index. Measured on a shared host under a load of 73 to 87 on 64 cores,
netex-rs 0.7.2 and this change alternately, with the same times and the same
memory — SNCF writes the specific frames, which 0.7.2 read already. A document
must fit into memory once, as text; a delivery of many documents is read a few
at a time, largest first. At 647 MB that is no reason to read a document
without holding its text.

## The inventory of Fluo Grand Est in the Bas-Rhin

53 files, 16.5 MB inflated, inventoried with `examples/inventory`.

108 distinct elements, 232048 occurrences; 156 distinct (parent, child) pairs; 91 distinct (element, attribute) pairs.

The inventory counts a fragment once, whatever it holds; counted by the
elements inside the fragments, 7.23 % of the delivery's elements are carried
through untyped (see "Read before and after").

### Carried through untyped after the typed read

7055 fragments in all, 3.040 % of the 232048 element occurrences (a fragment counts once, whatever it holds).

| Parent | Untyped child | Occurrences |
|---|---|---|
| `Location` | `pos` | 3087 |
| `AccessibilityAssessment` | `suitabilities` | 1742 |
| `Quay` | `PostalAddress` | 1035 |
| `StopPlace` | `PostalAddress` | 520 |
| `StopPlace` | `placeTypes` | 520 |
| `ResourceFrame` | `typesOfValue` | 53 |
| `Line` | `Presentation` | 49 |
| `Line` | `AccessibilityAssessment` | 44 |
| `members` | `GroupOfLines` | 4 |
| `members` | `Network` | 1 |

### Elements

| Element | Occurrences | Files |
|---|---|---|
| `TimetabledPassingTime` | 27800 | 50 |
| `ArrivalDayOffset` | 26029 | 50 |
| `ArrivalTime` | 26029 | 50 |
| `DepartureDayOffset` | 26029 | 50 |
| `DepartureTime` | 26029 | 50 |
| `ScheduledStopPointRef` | 6001 | 50 |
| `Name` | 4748 | 53 |
| `ForAlighting` | 4469 | 50 |
| `ForBoarding` | 4469 | 50 |
| `StopPointInJourneyPattern` | 4469 | 50 |
| `QuayRef` | 4219 | 52 |
| `DayTypeRef` | 4036 | 51 |
| `PrivateCode` | 3715 | 52 |
| `TransportMode` | 3376 | 52 |
| `StopPlaceRef` | 3184 | 51 |
| `Latitude` | 3087 | 51 |
| `Location` | 3087 | 51 |
| `Longitude` | 3087 | 51 |
| `pos` | 3087 | 51 |
| `AccessibilityAssessment` | 1786 | 46 |
| `MobilityImpairedAccess` | 1786 | 46 |
| `MobilityNeed` | 1786 | 46 |
| `Suitability` | 1786 | 46 |
| `Suitable` | 1786 | 46 |
| `suitabilities` | 1786 | 46 |
| `JourneyPatternRef` | 1771 | 50 |
| `ServiceJourney` | 1771 | 50 |
| `dayTypes` | 1771 | 50 |
| `passingTimes` | 1771 | 50 |
| `Centroid` | 1555 | 1 |
| `PostalAddress` | 1555 | 1 |
| `PostalRegion` | 1555 | 1 |
| `Town` | 1555 | 1 |
| `PassengerStopAssignment` | 1532 | 50 |
| `ScheduledStopPoint` | 1532 | 50 |
| `Date` | 1394 | 1 |
| `DayTypeAssignment` | 1394 | 1 |
| `isAvailable` | 1394 | 1 |
| `Quay` | 1035 | 1 |
| `SiteRef` | 1035 | 1 |
| `BothWays` | 826 | 1 |
| `Distance` | 826 | 1 |
| `From` | 826 | 1 |
| `SiteConnection` | 826 | 1 |
| `To` | 826 | 1 |
| `StopPlace` | 520 | 1 |
| `StopPlaceType` | 520 | 1 |
| `TypeOfPlaceRef` | 520 | 1 |
| `placeTypes` | 520 | 1 |
| `quays` | 520 | 1 |
| `DestinationDisplay` | 327 | 50 |
| `DestinationDisplayRef` | 327 | 50 |
| `FrontText` | 327 | 50 |
| `RouteRef` | 327 | 50 |
| `ServiceJourneyPattern` | 327 | 50 |
| `pointsInSequence` | 327 | 50 |
| `DirectionType` | 200 | 50 |
| `LineRef` | 200 | 51 |
| `TypeOfFrame` | 156 | 53 |
| `TypeOfFrameRef` | 156 | 53 |
| `DaysOfWeek` | 125 | 1 |
| `PropertyOfDay` | 125 | 1 |
| `BusSubmode` | 116 | 8 |
| `members` | 108 | 53 |
| `GeneralFrame` | 103 | 53 |
| `Direction` | 100 | 50 |
| `DirectionRef` | 100 | 50 |
| `InverseRouteRef` | 100 | 50 |
| `Route` | 100 | 50 |
| `Description` | 59 | 53 |
| `ValueSet` | 54 | 53 |
| `values` | 54 | 53 |
| `CompositeFrame` | 53 | 53 |
| `DefaultLanguage` | 53 | 53 |
| `DefaultLocale` | 53 | 53 |
| `DefaultLocationSystem` | 53 | 53 |
| `DefaultSystemOfUnits` | 53 | 53 |
| `FrameDefaults` | 53 | 53 |
| `ParticipantRef` | 53 | 53 |
| `PublicationDelivery` | 53 | 53 |
| `PublicationTimestamp` | 53 | 53 |
| `ResourceFrame` | 53 | 53 |
| `TimeZone` | 53 | 53 |
| `dataObjects` | 53 | 53 |
| `frames` | 53 | 53 |
| `typesOfValue` | 53 | 53 |
| `Line` | 50 | 1 |
| `OperatorRef` | 50 | 1 |
| `PublicCode` | 50 | 1 |
| `Colour` | 49 | 1 |
| `Presentation` | 49 | 1 |
| `TransportSubmode` | 37 | 7 |
| `FromDate` | 31 | 1 |
| `ToDate` | 31 | 1 |
| `ValidBetween` | 31 | 1 |
| `DayType` | 30 | 1 |
| `properties` | 30 | 1 |
| `OrganisationType` | 11 | 1 |
| `Operator` | 10 | 1 |
| `GroupOfLines` | 4 | 1 |
| `TypeOfPlace` | 2 | 1 |
| `Authority` | 1 | 1 |
| `AuthorityRef` | 1 | 1 |
| `ContactDetails` | 1 | 1 |
| `Email` | 1 | 1 |
| `Network` | 1 | 1 |
| `Phone` | 1 | 1 |
| `Url` | 1 | 1 |

### Attributes

| Element | Attribute | Occurrences |
|---|---|---|
| `TimetabledPassingTime` | `version` | 27800 |
| `ScheduledStopPointRef` | `ref` | 6001 |
| `ScheduledStopPointRef` | `versionRef` | 6001 |
| `StopPointInJourneyPattern` | `id` | 4469 |
| `StopPointInJourneyPattern` | `order` | 4469 |
| `StopPointInJourneyPattern` | `version` | 4469 |
| `QuayRef` | `ref` | 4219 |
| `QuayRef` | `versionRef` | 4219 |
| `DayTypeRef` | `ref` | 4036 |
| `DayTypeRef` | `versionRef` | 4036 |
| `StopPlaceRef` | `ref` | 3184 |
| `StopPlaceRef` | `versionRef` | 3184 |
| `pos` | `srsName` | 3087 |
| `AccessibilityAssessment` | `id` | 1786 |
| `AccessibilityAssessment` | `version` | 1786 |
| `Suitability` | `id` | 1786 |
| `Suitability` | `version` | 1786 |
| `JourneyPatternRef` | `ref` | 1771 |
| `JourneyPatternRef` | `versionRef` | 1771 |
| `ServiceJourney` | `id` | 1771 |
| `ServiceJourney` | `version` | 1771 |
| `PostalAddress` | `id` | 1555 |
| `PostalAddress` | `version` | 1555 |
| `PassengerStopAssignment` | `id` | 1532 |
| `PassengerStopAssignment` | `order` | 1532 |
| `PassengerStopAssignment` | `version` | 1532 |
| `ScheduledStopPoint` | `id` | 1532 |
| `ScheduledStopPoint` | `version` | 1532 |
| `DayTypeAssignment` | `id` | 1394 |
| `DayTypeAssignment` | `order` | 1394 |
| `DayTypeAssignment` | `version` | 1394 |
| `Quay` | `id` | 1035 |
| `Quay` | `version` | 1035 |
| `SiteRef` | `ref` | 1035 |
| `SiteRef` | `versionRef` | 1035 |
| `SiteConnection` | `id` | 826 |
| `SiteConnection` | `version` | 826 |
| `StopPlace` | `id` | 520 |
| `StopPlace` | `version` | 520 |
| `TypeOfPlaceRef` | `ref` | 520 |
| `DestinationDisplay` | `id` | 327 |
| `DestinationDisplay` | `version` | 327 |
| `DestinationDisplayRef` | `ref` | 327 |
| `DestinationDisplayRef` | `versionRef` | 327 |
| `RouteRef` | `ref` | 327 |
| `RouteRef` | `versionRef` | 327 |
| `ServiceJourneyPattern` | `id` | 327 |
| `ServiceJourneyPattern` | `version` | 327 |
| `LineRef` | `ref` | 200 |
| `LineRef` | `versionRef` | 200 |
| `TypeOfFrame` | `id` | 156 |
| `TypeOfFrame` | `version` | 156 |
| `TypeOfFrameRef` | `ref` | 156 |
| `GeneralFrame` | `id` | 103 |
| `GeneralFrame` | `version` | 103 |
| `Direction` | `id` | 100 |
| `Direction` | `version` | 100 |
| `DirectionRef` | `ref` | 100 |
| `DirectionRef` | `versionRef` | 100 |
| `InverseRouteRef` | `ref` | 100 |
| `InverseRouteRef` | `versionRef` | 100 |
| `Route` | `id` | 100 |
| `Route` | `version` | 100 |
| `ValueSet` | `id` | 54 |
| `ValueSet` | `nameOfClass` | 54 |
| `ValueSet` | `version` | 54 |
| `CompositeFrame` | `id` | 53 |
| `CompositeFrame` | `version` | 53 |
| `PublicationDelivery` | `version` | 53 |
| `PublicationDelivery` | `xmlns` | 53 |
| `PublicationDelivery` | `xmlns:gml` | 53 |
| `ResourceFrame` | `id` | 53 |
| `ResourceFrame` | `version` | 53 |
| `Line` | `id` | 50 |
| `Line` | `version` | 50 |
| `OperatorRef` | `ref` | 50 |
| `OperatorRef` | `versionRef` | 50 |
| `DayType` | `id` | 30 |
| `DayType` | `version` | 30 |
| `Operator` | `id` | 10 |
| `Operator` | `version` | 10 |
| `GroupOfLines` | `id` | 4 |
| `GroupOfLines` | `version` | 4 |
| `TypeOfPlace` | `id` | 2 |
| `TypeOfPlace` | `version` | 2 |
| `Authority` | `id` | 1 |
| `Authority` | `version` | 1 |
| `AuthorityRef` | `ref` | 1 |
| `AuthorityRef` | `versionRef` | 1 |
| `Network` | `id` | 1 |
| `Network` | `version` | 1 |

### Parents and children

| Parent | Child | Occurrences |
|---|---|---|
| `passingTimes` | `TimetabledPassingTime` | 27800 |
| `TimetabledPassingTime` | `ArrivalDayOffset` | 26029 |
| `TimetabledPassingTime` | `ArrivalTime` | 26029 |
| `TimetabledPassingTime` | `DepartureDayOffset` | 26029 |
| `TimetabledPassingTime` | `DepartureTime` | 26029 |
| `StopPointInJourneyPattern` | `ForAlighting` | 4469 |
| `StopPointInJourneyPattern` | `ForBoarding` | 4469 |
| `StopPointInJourneyPattern` | `ScheduledStopPointRef` | 4469 |
| `pointsInSequence` | `StopPointInJourneyPattern` | 4469 |
| `Location` | `Latitude` | 3087 |
| `Location` | `Longitude` | 3087 |
| `Location` | `pos` | 3087 |
| `dayTypes` | `DayTypeRef` | 2642 |
| `AccessibilityAssessment` | `MobilityImpairedAccess` | 1786 |
| `AccessibilityAssessment` | `suitabilities` | 1786 |
| `Suitability` | `MobilityNeed` | 1786 |
| `Suitability` | `Suitable` | 1786 |
| `suitabilities` | `Suitability` | 1786 |
| `ServiceJourney` | `JourneyPatternRef` | 1771 |
| `ServiceJourney` | `PrivateCode` | 1771 |
| `ServiceJourney` | `TransportMode` | 1771 |
| `ServiceJourney` | `dayTypes` | 1771 |
| `ServiceJourney` | `passingTimes` | 1771 |
| `members` | `ServiceJourney` | 1771 |
| `ServiceJourney` | `AccessibilityAssessment` | 1740 |
| `Centroid` | `Location` | 1555 |
| `PostalAddress` | `PostalRegion` | 1555 |
| `PostalAddress` | `Town` | 1555 |
| `PassengerStopAssignment` | `QuayRef` | 1532 |
| `PassengerStopAssignment` | `ScheduledStopPointRef` | 1532 |
| `PassengerStopAssignment` | `StopPlaceRef` | 1532 |
| `ScheduledStopPoint` | `Location` | 1532 |
| `ScheduledStopPoint` | `Name` | 1532 |
| `members` | `PassengerStopAssignment` | 1532 |
| `members` | `ScheduledStopPoint` | 1532 |
| `DayTypeAssignment` | `Date` | 1394 |
| `DayTypeAssignment` | `DayTypeRef` | 1394 |
| `DayTypeAssignment` | `isAvailable` | 1394 |
| `members` | `DayTypeAssignment` | 1394 |
| `Quay` | `Centroid` | 1035 |
| `Quay` | `Name` | 1035 |
| `Quay` | `PostalAddress` | 1035 |
| `Quay` | `PrivateCode` | 1035 |
| `Quay` | `SiteRef` | 1035 |
| `Quay` | `TransportMode` | 1035 |
| `members` | `Quay` | 1035 |
| `quays` | `QuayRef` | 1035 |
| `From` | `QuayRef` | 826 |
| `From` | `StopPlaceRef` | 826 |
| `SiteConnection` | `BothWays` | 826 |
| `SiteConnection` | `Distance` | 826 |
| `SiteConnection` | `From` | 826 |
| `SiteConnection` | `Name` | 826 |
| `SiteConnection` | `To` | 826 |
| `To` | `QuayRef` | 826 |
| `To` | `StopPlaceRef` | 826 |
| `members` | `SiteConnection` | 826 |
| `StopPlace` | `Centroid` | 520 |
| `StopPlace` | `Name` | 520 |
| `StopPlace` | `PostalAddress` | 520 |
| `StopPlace` | `PrivateCode` | 520 |
| `StopPlace` | `StopPlaceType` | 520 |
| `StopPlace` | `TransportMode` | 520 |
| `StopPlace` | `placeTypes` | 520 |
| `StopPlace` | `quays` | 520 |
| `members` | `StopPlace` | 520 |
| `placeTypes` | `TypeOfPlaceRef` | 520 |
| `DestinationDisplay` | `FrontText` | 327 |
| `ServiceJourneyPattern` | `DestinationDisplayRef` | 327 |
| `ServiceJourneyPattern` | `Name` | 327 |
| `ServiceJourneyPattern` | `PrivateCode` | 327 |
| `ServiceJourneyPattern` | `RouteRef` | 327 |
| `ServiceJourneyPattern` | `pointsInSequence` | 327 |
| `members` | `DestinationDisplay` | 327 |
| `members` | `ServiceJourneyPattern` | 327 |
| `TypeOfFrame` | `Name` | 156 |
| `values` | `TypeOfFrame` | 156 |
| `PropertyOfDay` | `DaysOfWeek` | 125 |
| `properties` | `PropertyOfDay` | 125 |
| `GeneralFrame` | `TypeOfFrameRef` | 103 |
| `GeneralFrame` | `members` | 103 |
| `frames` | `GeneralFrame` | 103 |
| `Direction` | `DirectionType` | 100 |
| `Direction` | `Name` | 100 |
| `Route` | `DirectionRef` | 100 |
| `Route` | `DirectionType` | 100 |
| `Route` | `InverseRouteRef` | 100 |
| `Route` | `LineRef` | 100 |
| `Route` | `Name` | 100 |
| `members` | `Direction` | 100 |
| `members` | `LineRef` | 100 |
| `members` | `Route` | 100 |
| `ValueSet` | `Name` | 54 |
| `ValueSet` | `values` | 54 |
| `typesOfValue` | `ValueSet` | 54 |
| `` | `PublicationDelivery` | 53 |
| `CompositeFrame` | `FrameDefaults` | 53 |
| `CompositeFrame` | `TypeOfFrameRef` | 53 |
| `CompositeFrame` | `frames` | 53 |
| `DefaultLocale` | `DefaultLanguage` | 53 |
| `DefaultLocale` | `TimeZone` | 53 |
| `FrameDefaults` | `DefaultLocale` | 53 |
| `FrameDefaults` | `DefaultLocationSystem` | 53 |
| `FrameDefaults` | `DefaultSystemOfUnits` | 53 |
| `PublicationDelivery` | `Description` | 53 |
| `PublicationDelivery` | `ParticipantRef` | 53 |
| `PublicationDelivery` | `PublicationTimestamp` | 53 |
| `PublicationDelivery` | `dataObjects` | 53 |
| `ResourceFrame` | `typesOfValue` | 53 |
| `dataObjects` | `CompositeFrame` | 53 |
| `frames` | `ResourceFrame` | 53 |
| `Quay` | `BusSubmode` | 51 |
| `Line` | `Name` | 50 |
| `Line` | `OperatorRef` | 50 |
| `Line` | `PrivateCode` | 50 |
| `Line` | `PublicCode` | 50 |
| `Line` | `TransportMode` | 50 |
| `members` | `Line` | 50 |
| `Line` | `Presentation` | 49 |
| `Presentation` | `Colour` | 49 |
| `Line` | `AccessibilityAssessment` | 44 |
| `TransportSubmode` | `BusSubmode` | 37 |
| `ServiceJourney` | `TransportSubmode` | 31 |
| `ValidBetween` | `FromDate` | 31 |
| `ValidBetween` | `ToDate` | 31 |
| `DayType` | `Name` | 30 |
| `DayType` | `ValidBetween` | 30 |
| `DayType` | `properties` | 30 |
| `members` | `DayType` | 30 |
| `StopPlace` | `BusSubmode` | 28 |
| `Operator` | `Name` | 10 |
| `Operator` | `OrganisationType` | 10 |
| `Operator` | `PrivateCode` | 10 |
| `members` | `Operator` | 10 |
| `Line` | `Description` | 6 |
| `Line` | `TransportSubmode` | 6 |
| `GroupOfLines` | `Name` | 4 |
| `GroupOfLines` | `members` | 4 |
| `members` | `GroupOfLines` | 4 |
| `Quay` | `AccessibilityAssessment` | 2 |
| `TypeOfPlace` | `Name` | 2 |
| `values` | `TypeOfPlace` | 2 |
| `Authority` | `ContactDetails` | 1 |
| `Authority` | `Name` | 1 |
| `Authority` | `OrganisationType` | 1 |
| `Authority` | `PrivateCode` | 1 |
| `ContactDetails` | `Email` | 1 |
| `ContactDetails` | `Phone` | 1 |
| `ContactDetails` | `Url` | 1 |
| `GeneralFrame` | `ValidBetween` | 1 |
| `Network` | `AuthorityRef` | 1 |
| `Network` | `Name` | 1 |
| `Network` | `PrivateCode` | 1 |
| `Network` | `members` | 1 |
| `members` | `Authority` | 1 |
| `members` | `Network` | 1 |
