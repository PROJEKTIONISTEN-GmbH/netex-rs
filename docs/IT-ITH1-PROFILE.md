# The Italian profile of South Tyrol (IT-ITH1)

South Tyrol's public transport (STA) publishes its timetable at the Italian
national access point (NAP) in the Italian NeTEx profile, "IT-ITH1 – Profilo
NeTEx (Bolzano)", in four levels: level 1 the timetable, the network, the
calendar, the lines, the operators and the stops; level 2 adds contracts and
vehicle equipment, level 3 fares, zones and points of sale, level 4 sharing
fleets. Each level is one document, gzip-compressed XML. This page describes the
form of levels 1 and 3 as fetched on 2026-10-20 (`IT-ITH1_L1.xml.gz`,
`IT-ITH1_L3.xml.gz`; the validated download of level 1 is the same file): what
they state, how `netex-rs` reads it, and the inventory of level 1 with
`examples/inventory`. Nothing of the deliveries is in this repository: the
crate's own example of the profile is `tests/fixtures/own/it/`.

## The delivery

| | Level 1 | Level 3 |
|---|---|---|
| Size, compressed / inflated | 23.7 MB / 828.1 MB | 23.7 MB / 828.7 MB |
| Release | NeTEx 1.10 (`version="1.10"`), written by MENTZ (`NeTEx Export, Version: 19.67.0.6309`) | the same |
| Frames | one composite frame (`epip:EU_PI_LINE_OFFER`) of a resource, site, service, calendar and timetable frame, each typed `epip:EU_PI_*` | the same, and a `GeneralFrame` of contracts and a `FareFrame` |
| Published / valid | 2025-08-14 / 2024-12-15 to 2025-12-13 | the same |

The document names itself `EU_Minimal` in a comment; its frames are those of the
European Passenger Information Profile (EPIP). Level 3 states the timetable of
level 1 as it is: read into the index, both give the same figures.

## What it states, and how the crate reads it

**Identifiers** are of the form `IT:ITH1:<class>:<code>:`; stop places and
quays carry their national stop identifier as a `GlobalID` key (`it:22021:…`
for South Tyrol, `at:47:…` for the Tyrolean stops served across the border).

**Names.** Every name but the authority's and the calendar's states its language
— and every one of them, of stop places, quays, stop points, lines, patterns,
day types, destination displays, operators and notices, states `de`, whatever
the language of the text: South Tyrol's stops are named in Italian under
`lang="de"`, the Austrian stops in German. A stop place, quay or stop point has
one name (two quays none); 153 stop places have two alternative names, the name
with and without its town, also under `de`, with no `Lang` and no `NameType`.
There is no German or Ladin name of a South Tyrolean stop. Only the 280 tariff
zones state `it`, 265 of them named in both languages in one text. The crate
gives every name with the language it states — `MultilingualString::lang`, and
for an alternative name its `Lang`, `NameType` and the `lang` of its text — and
takes none for another.

**Lines and routes.** 254 lines in 797 versions, each valid from the whole
timetable year down to a single day. Of the 12 256 journey patterns, 11 625 name
their line in a route view; 631 name only a route, which names the line: the 929
journeys following them name no line themselves. Since 0.6.0 the index finds
their line through the route (`docs/INTERPRETATION.md` § 22). The days of every
journey are those of its day types its line version governs (§ 4): 50 414
journeys lose days so, 1 924 are left out with none — among them 871 of the 929.

**Calendar.** One service calendar in the calendar frame: 209 day types, each
assigned one UIC operating period through an `OperatingPeriodRef`
(`Concern::ResolvedAsOtherClass`), each period of 365 day bits over the 364 days
from 2024-12-15 to 2025-12-13 (`Concern::DayBitsLengthDiffers`; the bits are
read from the first day).

**Journeys.** 54 939 journeys, each with its passing times (1 166 644, 13 980 of
them past midnight, on a following day); no time demand type, no calls in place
of passing times, no template journey, no flexible line: STA's on-demand
services are not in this delivery. 6 967 journeys name a train number; the 558
`TrainNumber`s stand in the timetable frame, read since 0.6.0. 4 300 points of
the patterns are passed through (`StopUse` `passthrough`).

**Stops.** 2 962 stop places with 3 170 quays, 5 425 stop points each assigned
to a quay of its stop place, 2 266 walks between stops (`SiteConnection`), each
end a quay or a stop place. 2 959 stop places state a submode beside their mode
(`BusSubmode`, `TramSubmode`, `RailSubmode`), every one `unknown`, read since
0.6.0 (`StopPlace::submode`).

**Geometry.** 8 485 service links with a `gml:LineString` in longitude–latitude
order (`Concern::AxisOrderSwapped`), carried through untyped as every GML
geometry is and read where the index checks axis order or a converter draws
shapes.

**What stays untyped**, carried through as written: the service facility of
every journey (`facilities`, 54 939, all `freeSeating`), the GML geometries, an
authority's empty address and the frame's tariff zones; in level 3 besides, the
fare frame, the contracts' general frame, a list of vehicles (54, and 683
commented out) and a travel agent. None of it is timetable or a name of a stop
or line.

## Read before and after 0.6.0

| | netex-rs 0.5.1 | netex-rs 0.6.0 |
|---|---|---|
| Fragments carried through untyped, level 1 | 66 386 (0.866 % of 7 668 733 elements) | 63 426 (0.827 %) |
| of them stop-place submodes / the frame's train numbers | 2 959 / 1 | 0 / 0 |
| Index: stops, stop points, lines | 6 132, 5 425, 797 | the same |
| Index: journeys, of them without a line | 53 886, 929 | 53 015, 0 |
| Index: journeys left out by their line version | 1 053 | 1 924 |
| Index: passing times | 1 135 300 | 1 119 332 |
| References that do not resolve | 0 | 0 |
| `netex convert --to gtfs`: trips (journeys left out as naming no line) | 52 957 (929) | 53 015 (0) |
| `netex validate`: definitions nothing refers to | 482 | 10 984 — the 10 503 routes nothing names join them, one line named only through routes leaves |

## The inventory of level 1

1 files, 0.8 GB inflated, inventoried in 42 s on 8 threads.

168 distinct elements, 7668733 occurrences; 228 distinct (parent, child) pairs; 145 distinct (element, attribute) pairs.

### Carried through untyped after the typed read

63426 fragments in all, 0.827 % of the 7668733 element occurrences (a fragment counts once, whatever it holds).

| Parent | Untyped child | Occurrences |
|---|---|---|
| `ServiceJourney` | `facilities` | 54939 |
| `ServiceLink` | `LineString` | 8485 |
| `Authority` | `Address` | 1 |
| `SiteFrame` | `tariffZones` | 1 |

### Elements

| Element | Occurrences | Files |
|---|---|---|
| `DepartureTime` | 1166644 | 1 |
| `StopPointInJourneyPatternRef` | 1166644 | 1 |
| `TimetabledPassingTime` | 1166644 | 1 |
| `ArrivalTime` | 1111705 | 1 |
| `ScheduledStopPointRef` | 242836 | 1 |
| `ForAlighting` | 237411 | 1 |
| `ForBoarding` | 237411 | 1 |
| `RequestStop` | 237411 | 1 |
| `StopPointInJourneyPattern` | 237411 | 1 |
| `StopUse` | 237411 | 1 |
| `OnwardServiceLinkRef` | 212943 | 1 |
| `PointOnRoute` | 210703 | 1 |
| `RoutePointRef` | 210703 | 1 |
| `TransportMode` | 58695 | 1 |
| `DayTypeRef` | 55148 | 1 |
| `dayTypes` | 54940 | 1 |
| `AccommodationAccessList` | 54939 | 1 |
| `ServiceFacilitySet` | 54939 | 1 |
| `ServiceJourney` | 54939 | 1 |
| `ServiceJourneyPatternRef` | 54939 | 1 |
| `facilities` | 54939 | 1 |
| `passingTimes` | 54939 | 1 |
| `VehicleTypeRef` | 54861 | 1 |
| `Name` | 25933 | 1 |
| `pointsInSequence` | 23359 | 1 |
| `LineRef` | 22728 | 1 |
| `DestinationDisplayRef` | 15603 | 1 |
| `DepartureDayOffset` | 13845 | 1 |
| `ArrivalDayOffset` | 13592 | 1 |
| `ShortName` | 12653 | 1 |
| `ServiceJourneyPattern` | 12256 | 1 |
| `RouteView` | 11625 | 1 |
| `Latitude` | 11557 | 1 |
| `Location` | 11557 | 1 |
| `Longitude` | 11557 | 1 |
| `DirectionRef` | 11103 | 1 |
| `Route` | 11103 | 1 |
| `StopPlaceRef` | 9957 | 1 |
| `Key` | 9757 | 1 |
| `KeyValue` | 9757 | 1 |
| `Value` | 9757 | 1 |
| `keyList` | 9249 | 1 |
| `FromPointRef` | 8495 | 1 |
| `ServiceLink` | 8495 | 1 |
| `ToPointRef` | 8495 | 1 |
| `Distance` | 8489 | 1 |
| `LineString` | 8485 | 1 |
| `posList` | 8485 | 1 |
| `QuayRef` | 8235 | 1 |
| `NoticeAssignment` | 7892 | 1 |
| `NoticeRef` | 7892 | 1 |
| `noticeAssignments` | 7720 | 1 |
| `PrivateCode` | 7375 | 1 |
| `trainNumbers` | 6968 | 1 |
| `TrainNumberRef` | 6967 | 1 |
| `Centroid` | 6132 | 1 |
| `PassengerStopAssignment` | 5425 | 1 |
| `ScheduledStopPoint` | 5425 | 1 |
| `FromDate` | 3970 | 1 |
| `PublicCode` | 3968 | 1 |
| `ValidBetween` | 3760 | 1 |
| `BusSubmode` | 3527 | 1 |
| `Quay` | 3170 | 1 |
| `StopPlace` | 2962 | 1 |
| `StopPlaceEntrance` | 2962 | 1 |
| `StopPlaceType` | 2962 | 1 |
| `TopographicPlaceRef` | 2962 | 1 |
| `entrances` | 2962 | 1 |
| `quays` | 2962 | 1 |
| `BothWays` | 2266 | 1 |
| `DefaultDuration` | 2266 | 1 |
| `From` | 2266 | 1 |
| `SiteConnection` | 2266 | 1 |
| `To` | 2266 | 1 |
| `WalkTransferDuration` | 2266 | 1 |
| `DestinationDisplay` | 867 | 1 |
| `FrontText` | 867 | 1 |
| `SideText` | 867 | 1 |
| `Line` | 797 | 1 |
| `OperatorRef` | 797 | 1 |
| `TransportSubmode` | 797 | 1 |
| `ToDate` | 647 | 1 |
| `RouteRef` | 631 | 1 |
| `ForAdvertisement` | 558 | 1 |
| `TrainNumber` | 558 | 1 |
| `Descriptor` | 508 | 1 |
| `TopographicPlace` | 508 | 1 |
| `TopographicPlaceType` | 508 | 1 |
| `QuayType` | 354 | 1 |
| `AlternativeName` | 306 | 1 |
| `TariffZone` | 280 | 1 |
| `DayType` | 209 | 1 |
| `DayTypeAssignment` | 209 | 1 |
| `OperatingPeriodRef` | 209 | 1 |
| `UicOperatingPeriod` | 209 | 1 |
| `ValidDayBits` | 209 | 1 |
| `alternativeNames` | 153 | 1 |
| `TramSubmode` | 120 | 1 |
| `Notice` | 92 | 1 |
| `RailSubmode` | 87 | 1 |
| `Text` | 86 | 1 |
| `ResponsibilityRoleAssignment` | 41 | 1 |
| `ResponsibleOrganisationRef` | 41 | 1 |
| `StakeholderRoleType` | 41 | 1 |
| `HasHoist` | 24 | 1 |
| `HasLiftOrRamp` | 24 | 1 |
| `LowFloor` | 24 | 1 |
| `VehicleType` | 24 | 1 |
| `ContactDetails` | 21 | 1 |
| `Email` | 21 | 1 |
| `OrganisationType` | 21 | 1 |
| `ResponsibilitySet` | 21 | 1 |
| `roles` | 21 | 1 |
| `LegalName` | 20 | 1 |
| `Operator` | 20 | 1 |
| `TelecabinSubmode` | 20 | 1 |
| `Length` | 15 | 1 |
| `TypeOfFrameRef` | 6 | 1 |
| `Description` | 2 | 1 |
| `Direction` | 2 | 1 |
| `DirectionType` | 2 | 1 |
| `FunicularSubmode` | 2 | 1 |
| `Address` | 1 | 1 |
| `Authority` | 1 | 1 |
| `Codespace` | 1 | 1 |
| `CompositeFrame` | 1 | 1 |
| `DataRoleType` | 1 | 1 |
| `DefaultCodespaceRef` | 1 | 1 |
| `DefaultLocale` | 1 | 1 |
| `DefaultResponsibilitySetRef` | 1 | 1 |
| `FrameDefaults` | 1 | 1 |
| `ParentSiteRef` | 1 | 1 |
| `ParticipantRef` | 1 | 1 |
| `PublicationDelivery` | 1 | 1 |
| `PublicationTimestamp` | 1 | 1 |
| `ResourceFrame` | 1 | 1 |
| `ServiceCalendar` | 1 | 1 |
| `ServiceCalendarFrame` | 1 | 1 |
| `ServiceFrame` | 1 | 1 |
| `SiteFrame` | 1 | 1 |
| `TimeZone` | 1 | 1 |
| `TimetableFrame` | 1 | 1 |
| `Url` | 1 | 1 |
| `Width` | 1 | 1 |
| `Xmlns` | 1 | 1 |
| `XmlnsUrl` | 1 | 1 |
| `codespaces` | 1 | 1 |
| `connections` | 1 | 1 |
| `dataObjects` | 1 | 1 |
| `dayTypeAssignments` | 1 | 1 |
| `destinationDisplays` | 1 | 1 |
| `directions` | 1 | 1 |
| `frames` | 1 | 1 |
| `journeyPatterns` | 1 | 1 |
| `lines` | 1 | 1 |
| `notices` | 1 | 1 |
| `operatingPeriods` | 1 | 1 |
| `organisations` | 1 | 1 |
| `responsibilitySets` | 1 | 1 |
| `routes` | 1 | 1 |
| `scheduledStopPoints` | 1 | 1 |
| `serviceLinks` | 1 | 1 |
| `stopAssignments` | 1 | 1 |
| `stopPlaces` | 1 | 1 |
| `tariffZones` | 1 | 1 |
| `topographicPlaces` | 1 | 1 |
| `vehicleJourneys` | 1 | 1 |
| `vehicleTypes` | 1 | 1 |

### Attributes

| Element | Attribute | Occurrences |
|---|---|---|
| `StopPointInJourneyPatternRef` | `order` | 1166644 |
| `StopPointInJourneyPatternRef` | `ref` | 1166644 |
| `StopPointInJourneyPatternRef` | `version` | 1166644 |
| `TimetabledPassingTime` | `id` | 1166644 |
| `TimetabledPassingTime` | `version` | 1166644 |
| `ScheduledStopPointRef` | `ref` | 242836 |
| `ScheduledStopPointRef` | `version` | 242836 |
| `StopPointInJourneyPattern` | `id` | 237411 |
| `StopPointInJourneyPattern` | `order` | 237411 |
| `StopPointInJourneyPattern` | `version` | 237411 |
| `OnwardServiceLinkRef` | `ref` | 212943 |
| `OnwardServiceLinkRef` | `version` | 212943 |
| `PointOnRoute` | `id` | 210703 |
| `PointOnRoute` | `order` | 210703 |
| `PointOnRoute` | `version` | 210703 |
| `RoutePointRef` | `ref` | 210703 |
| `RoutePointRef` | `version` | 210703 |
| `DayTypeRef` | `ref` | 55148 |
| `DayTypeRef` | `version` | 55148 |
| `ServiceFacilitySet` | `id` | 54939 |
| `ServiceFacilitySet` | `version` | 54939 |
| `ServiceJourney` | `id` | 54939 |
| `ServiceJourney` | `version` | 54939 |
| `ServiceJourneyPatternRef` | `ref` | 54939 |
| `ServiceJourneyPatternRef` | `version` | 54939 |
| `VehicleTypeRef` | `ref` | 54861 |
| `VehicleTypeRef` | `version` | 54861 |
| `Name` | `lang` | 25931 |
| `LineRef` | `ref` | 22728 |
| `LineRef` | `version` | 22728 |
| `DestinationDisplayRef` | `ref` | 15603 |
| `DestinationDisplayRef` | `version` | 15603 |
| `ShortName` | `lang` | 12652 |
| `ServiceJourneyPattern` | `id` | 12256 |
| `ServiceJourneyPattern` | `version` | 12256 |
| `DirectionRef` | `ref` | 11103 |
| `DirectionRef` | `version` | 11103 |
| `Route` | `id` | 11103 |
| `Route` | `version` | 11103 |
| `StopPlaceRef` | `ref` | 9957 |
| `StopPlaceRef` | `version` | 9957 |
| `FromPointRef` | `ref` | 8495 |
| `FromPointRef` | `version` | 8495 |
| `ServiceLink` | `id` | 8495 |
| `ServiceLink` | `version` | 8495 |
| `ToPointRef` | `ref` | 8495 |
| `ToPointRef` | `version` | 8495 |
| `LineString` | `gml:id` | 8485 |
| `QuayRef` | `ref` | 8235 |
| `QuayRef` | `version` | 8235 |
| `ServiceJourney` | `responsibilitySetRef` | 8114 |
| `NoticeAssignment` | `id` | 7892 |
| `NoticeAssignment` | `order` | 7892 |
| `NoticeAssignment` | `version` | 7892 |
| `NoticeRef` | `ref` | 7892 |
| `NoticeRef` | `version` | 7892 |
| `TrainNumberRef` | `ref` | 6967 |
| `TrainNumberRef` | `version` | 6967 |
| `PassengerStopAssignment` | `id` | 5425 |
| `PassengerStopAssignment` | `order` | 5425 |
| `PassengerStopAssignment` | `version` | 5425 |
| `ScheduledStopPoint` | `id` | 5425 |
| `ScheduledStopPoint` | `version` | 5425 |
| `Quay` | `id` | 3170 |
| `Quay` | `version` | 3170 |
| `StopPlace` | `id` | 2962 |
| `StopPlace` | `version` | 2962 |
| `StopPlaceEntrance` | `id` | 2962 |
| `StopPlaceEntrance` | `version` | 2962 |
| `TopographicPlaceRef` | `ref` | 2962 |
| `TopographicPlaceRef` | `version` | 2962 |
| `SiteConnection` | `id` | 2266 |
| `SiteConnection` | `version` | 2266 |
| `DestinationDisplay` | `id` | 867 |
| `DestinationDisplay` | `version` | 867 |
| `FrontText` | `lang` | 867 |
| `SideText` | `lang` | 867 |
| `Line` | `id` | 797 |
| `Line` | `responsibilitySetRef` | 797 |
| `Line` | `version` | 797 |
| `OperatorRef` | `ref` | 797 |
| `OperatorRef` | `version` | 797 |
| `RouteRef` | `ref` | 631 |
| `RouteRef` | `version` | 631 |
| `TrainNumber` | `id` | 558 |
| `TrainNumber` | `version` | 558 |
| `TopographicPlace` | `id` | 508 |
| `TopographicPlace` | `version` | 508 |
| `TariffZone` | `id` | 280 |
| `TariffZone` | `version` | 280 |
| `DayType` | `id` | 209 |
| `DayType` | `version` | 209 |
| `DayTypeAssignment` | `id` | 209 |
| `DayTypeAssignment` | `order` | 209 |
| `DayTypeAssignment` | `version` | 209 |
| `OperatingPeriodRef` | `ref` | 209 |
| `OperatingPeriodRef` | `version` | 209 |
| `UicOperatingPeriod` | `id` | 209 |
| `UicOperatingPeriod` | `version` | 209 |
| `Notice` | `id` | 92 |
| `Notice` | `version` | 92 |
| `Text` | `lang` | 86 |
| `ResponsibilityRoleAssignment` | `id` | 41 |
| `ResponsibilityRoleAssignment` | `version` | 41 |
| `ResponsibleOrganisationRef` | `ref` | 41 |
| `ResponsibleOrganisationRef` | `version` | 41 |
| `VehicleType` | `id` | 24 |
| `VehicleType` | `version` | 24 |
| `ResponsibilitySet` | `id` | 21 |
| `ResponsibilitySet` | `version` | 21 |
| `Operator` | `id` | 20 |
| `Operator` | `version` | 20 |
| `TypeOfFrameRef` | `ref` | 6 |
| `Direction` | `id` | 2 |
| `Direction` | `version` | 2 |
| `Address` | `id` | 1 |
| `Address` | `version` | 1 |
| `Authority` | `id` | 1 |
| `Authority` | `version` | 1 |
| `Codespace` | `id` | 1 |
| `CompositeFrame` | `id` | 1 |
| `CompositeFrame` | `version` | 1 |
| `DefaultCodespaceRef` | `ref` | 1 |
| `DefaultResponsibilitySetRef` | `ref` | 1 |
| `DefaultResponsibilitySetRef` | `version` | 1 |
| `ParentSiteRef` | `ref` | 1 |
| `ParentSiteRef` | `version` | 1 |
| `PublicationDelivery` | `version` | 1 |
| `PublicationDelivery` | `xmlns` | 1 |
| `PublicationDelivery` | `xmlns:gml` | 1 |
| `PublicationDelivery` | `xmlns:siri` | 1 |
| `PublicationDelivery` | `xmlns:xsi` | 1 |
| `PublicationDelivery` | `xsi:schemaLocation` | 1 |
| `ResourceFrame` | `id` | 1 |
| `ResourceFrame` | `version` | 1 |
| `ServiceCalendar` | `id` | 1 |
| `ServiceCalendar` | `version` | 1 |
| `ServiceCalendarFrame` | `id` | 1 |
| `ServiceCalendarFrame` | `version` | 1 |
| `ServiceFrame` | `id` | 1 |
| `ServiceFrame` | `version` | 1 |
| `SiteFrame` | `id` | 1 |
| `SiteFrame` | `version` | 1 |
| `TimetableFrame` | `id` | 1 |
| `TimetableFrame` | `version` | 1 |

### Parents and children

| Parent | Child | Occurrences |
|---|---|---|
| `TimetabledPassingTime` | `StopPointInJourneyPatternRef` | 1166644 |
| `passingTimes` | `TimetabledPassingTime` | 1166644 |
| `TimetabledPassingTime` | `ArrivalTime` | 1111705 |
| `TimetabledPassingTime` | `DepartureTime` | 1111705 |
| `StopPointInJourneyPattern` | `ForAlighting` | 237411 |
| `StopPointInJourneyPattern` | `ForBoarding` | 237411 |
| `StopPointInJourneyPattern` | `RequestStop` | 237411 |
| `StopPointInJourneyPattern` | `ScheduledStopPointRef` | 237411 |
| `StopPointInJourneyPattern` | `StopUse` | 237411 |
| `pointsInSequence` | `StopPointInJourneyPattern` | 237411 |
| `StopPointInJourneyPattern` | `OnwardServiceLinkRef` | 212943 |
| `PointOnRoute` | `RoutePointRef` | 210703 |
| `pointsInSequence` | `PointOnRoute` | 210703 |
| `ServiceFacilitySet` | `AccommodationAccessList` | 54939 |
| `ServiceJourney` | `DepartureTime` | 54939 |
| `ServiceJourney` | `ServiceJourneyPatternRef` | 54939 |
| `ServiceJourney` | `TransportMode` | 54939 |
| `ServiceJourney` | `dayTypes` | 54939 |
| `ServiceJourney` | `facilities` | 54939 |
| `ServiceJourney` | `passingTimes` | 54939 |
| `dayTypes` | `DayTypeRef` | 54939 |
| `facilities` | `ServiceFacilitySet` | 54939 |
| `vehicleJourneys` | `ServiceJourney` | 54939 |
| `ServiceJourney` | `VehicleTypeRef` | 54861 |
| `StopPointInJourneyPattern` | `DestinationDisplayRef` | 15603 |
| `TimetabledPassingTime` | `ArrivalDayOffset` | 13592 |
| `TimetabledPassingTime` | `DepartureDayOffset` | 13459 |
| `ServiceJourneyPattern` | `Name` | 12256 |
| `ServiceJourneyPattern` | `pointsInSequence` | 12256 |
| `journeyPatterns` | `ServiceJourneyPattern` | 12256 |
| `RouteView` | `LineRef` | 11625 |
| `ServiceJourneyPattern` | `RouteView` | 11625 |
| `Location` | `Latitude` | 11557 |
| `Location` | `Longitude` | 11557 |
| `Route` | `DirectionRef` | 11103 |
| `Route` | `LineRef` | 11103 |
| `Route` | `pointsInSequence` | 11103 |
| `routes` | `Route` | 11103 |
| `KeyValue` | `Key` | 9757 |
| `KeyValue` | `Value` | 9757 |
| `keyList` | `KeyValue` | 9757 |
| `ServiceLink` | `FromPointRef` | 8495 |
| `ServiceLink` | `ToPointRef` | 8495 |
| `serviceLinks` | `ServiceLink` | 8495 |
| `ServiceLink` | `Distance` | 8489 |
| `LineString` | `posList` | 8485 |
| `ServiceLink` | `LineString` | 8485 |
| `NoticeAssignment` | `NoticeRef` | 7892 |
| `noticeAssignments` | `NoticeAssignment` | 7892 |
| `ServiceJourney` | `trainNumbers` | 6967 |
| `trainNumbers` | `TrainNumberRef` | 6967 |
| `StopPointInJourneyPattern` | `noticeAssignments` | 6632 |
| `Centroid` | `Location` | 6132 |
| `PassengerStopAssignment` | `QuayRef` | 5425 |
| `PassengerStopAssignment` | `ScheduledStopPointRef` | 5425 |
| `PassengerStopAssignment` | `StopPlaceRef` | 5425 |
| `ScheduledStopPoint` | `Location` | 5425 |
| `ScheduledStopPoint` | `Name` | 5425 |
| `ScheduledStopPoint` | `ShortName` | 5425 |
| `ScheduledStopPoint` | `keyList` | 5425 |
| `scheduledStopPoints` | `ScheduledStopPoint` | 5425 |
| `stopAssignments` | `PassengerStopAssignment` | 5425 |
| `ValidBetween` | `FromDate` | 3760 |
| `Quay` | `Centroid` | 3170 |
| `Quay` | `PublicCode` | 3170 |
| `quays` | `Quay` | 3170 |
| `Quay` | `Name` | 3168 |
| `Quay` | `ShortName` | 3168 |
| `StopPlace` | `Centroid` | 2962 |
| `StopPlace` | `Name` | 2962 |
| `StopPlace` | `PrivateCode` | 2962 |
| `StopPlace` | `ShortName` | 2962 |
| `StopPlace` | `StopPlaceType` | 2962 |
| `StopPlace` | `TopographicPlaceRef` | 2962 |
| `StopPlace` | `ValidBetween` | 2962 |
| `StopPlace` | `entrances` | 2962 |
| `StopPlace` | `keyList` | 2962 |
| `StopPlace` | `quays` | 2962 |
| `StopPlaceEntrance` | `PrivateCode` | 2962 |
| `entrances` | `StopPlaceEntrance` | 2962 |
| `stopPlaces` | `StopPlace` | 2962 |
| `StopPlace` | `TransportMode` | 2959 |
| `StopPlace` | `BusSubmode` | 2827 |
| `From` | `StopPlaceRef` | 2266 |
| `SiteConnection` | `BothWays` | 2266 |
| `SiteConnection` | `From` | 2266 |
| `SiteConnection` | `To` | 2266 |
| `SiteConnection` | `WalkTransferDuration` | 2266 |
| `To` | `StopPlaceRef` | 2266 |
| `WalkTransferDuration` | `DefaultDuration` | 2266 |
| `connections` | `SiteConnection` | 2266 |
| `From` | `QuayRef` | 1405 |
| `To` | `QuayRef` | 1405 |
| `ServiceJourney` | `noticeAssignments` | 1088 |
| `DestinationDisplay` | `FrontText` | 867 |
| `DestinationDisplay` | `SideText` | 867 |
| `destinationDisplays` | `DestinationDisplay` | 867 |
| `Line` | `Name` | 797 |
| `Line` | `OperatorRef` | 797 |
| `Line` | `PrivateCode` | 797 |
| `Line` | `PublicCode` | 797 |
| `Line` | `ShortName` | 797 |
| `Line` | `TransportMode` | 797 |
| `Line` | `TransportSubmode` | 797 |
| `Line` | `ValidBetween` | 797 |
| `lines` | `Line` | 797 |
| `TransportSubmode` | `BusSubmode` | 700 |
| `ServiceJourneyPattern` | `RouteRef` | 631 |
| `TrainNumber` | `ForAdvertisement` | 558 |
| `trainNumbers` | `TrainNumber` | 558 |
| `Descriptor` | `Name` | 508 |
| `TopographicPlace` | `Descriptor` | 508 |
| `TopographicPlace` | `TopographicPlaceType` | 508 |
| `TopographicPlace` | `keyList` | 508 |
| `topographicPlaces` | `TopographicPlace` | 508 |
| `ValidBetween` | `ToDate` | 437 |
| `ServiceJourney` | `DepartureDayOffset` | 386 |
| `Quay` | `PrivateCode` | 354 |
| `Quay` | `QuayType` | 354 |
| `Quay` | `keyList` | 354 |
| `AlternativeName` | `Name` | 306 |
| `alternativeNames` | `AlternativeName` | 306 |
| `TariffZone` | `Name` | 280 |
| `TariffZone` | `PrivateCode` | 280 |
| `TariffZone` | `ShortName` | 280 |
| `tariffZones` | `TariffZone` | 280 |
| `DayType` | `Name` | 209 |
| `DayTypeAssignment` | `DayTypeRef` | 209 |
| `DayTypeAssignment` | `OperatingPeriodRef` | 209 |
| `UicOperatingPeriod` | `FromDate` | 209 |
| `UicOperatingPeriod` | `ToDate` | 209 |
| `UicOperatingPeriod` | `ValidDayBits` | 209 |
| `dayTypeAssignments` | `DayTypeAssignment` | 209 |
| `dayTypes` | `DayType` | 209 |
| `operatingPeriods` | `UicOperatingPeriod` | 209 |
| `StopPlace` | `alternativeNames` | 153 |
| `StopPlace` | `TramSubmode` | 120 |
| `notices` | `Notice` | 92 |
| `Notice` | `Text` | 86 |
| `TransportSubmode` | `RailSubmode` | 75 |
| `ResponsibilityRoleAssignment` | `ResponsibleOrganisationRef` | 41 |
| `ResponsibilityRoleAssignment` | `StakeholderRoleType` | 41 |
| `roles` | `ResponsibilityRoleAssignment` | 41 |
| `VehicleType` | `HasHoist` | 24 |
| `VehicleType` | `HasLiftOrRamp` | 24 |
| `VehicleType` | `LowFloor` | 24 |
| `vehicleTypes` | `VehicleType` | 24 |
| `ContactDetails` | `Email` | 21 |
| `ResponsibilitySet` | `roles` | 21 |
| `responsibilitySets` | `ResponsibilitySet` | 21 |
| `Operator` | `ContactDetails` | 20 |
| `Operator` | `LegalName` | 20 |
| `Operator` | `Name` | 20 |
| `Operator` | `OrganisationType` | 20 |
| `Operator` | `ShortName` | 20 |
| `ResponsibilitySet` | `PrivateCode` | 20 |
| `TransportSubmode` | `TelecabinSubmode` | 20 |
| `organisations` | `Operator` | 20 |
| `VehicleType` | `Length` | 15 |
| `StopPlace` | `RailSubmode` | 12 |
| `Direction` | `DirectionType` | 2 |
| `TransportSubmode` | `FunicularSubmode` | 2 |
| `directions` | `Direction` | 2 |
| `` | `PublicationDelivery` | 1 |
| `Authority` | `Address` | 1 |
| `Authority` | `ContactDetails` | 1 |
| `Authority` | `Name` | 1 |
| `Authority` | `OrganisationType` | 1 |
| `Authority` | `PublicCode` | 1 |
| `Authority` | `ShortName` | 1 |
| `Codespace` | `Description` | 1 |
| `Codespace` | `Xmlns` | 1 |
| `Codespace` | `XmlnsUrl` | 1 |
| `CompositeFrame` | `FrameDefaults` | 1 |
| `CompositeFrame` | `TypeOfFrameRef` | 1 |
| `CompositeFrame` | `ValidBetween` | 1 |
| `CompositeFrame` | `codespaces` | 1 |
| `CompositeFrame` | `frames` | 1 |
| `ContactDetails` | `Url` | 1 |
| `DefaultLocale` | `TimeZone` | 1 |
| `FrameDefaults` | `DefaultCodespaceRef` | 1 |
| `FrameDefaults` | `DefaultLocale` | 1 |
| `FrameDefaults` | `DefaultResponsibilitySetRef` | 1 |
| `PublicationDelivery` | `Description` | 1 |
| `PublicationDelivery` | `ParticipantRef` | 1 |
| `PublicationDelivery` | `PublicationTimestamp` | 1 |
| `PublicationDelivery` | `dataObjects` | 1 |
| `ResourceFrame` | `TypeOfFrameRef` | 1 |
| `ResourceFrame` | `organisations` | 1 |
| `ResourceFrame` | `responsibilitySets` | 1 |
| `ResourceFrame` | `vehicleTypes` | 1 |
| `ResponsibilityRoleAssignment` | `DataRoleType` | 1 |
| `ServiceCalendar` | `FromDate` | 1 |
| `ServiceCalendar` | `Name` | 1 |
| `ServiceCalendar` | `ToDate` | 1 |
| `ServiceCalendar` | `dayTypeAssignments` | 1 |
| `ServiceCalendar` | `dayTypes` | 1 |
| `ServiceCalendar` | `operatingPeriods` | 1 |
| `ServiceCalendarFrame` | `ServiceCalendar` | 1 |
| `ServiceCalendarFrame` | `TypeOfFrameRef` | 1 |
| `ServiceFrame` | `TypeOfFrameRef` | 1 |
| `ServiceFrame` | `connections` | 1 |
| `ServiceFrame` | `destinationDisplays` | 1 |
| `ServiceFrame` | `directions` | 1 |
| `ServiceFrame` | `journeyPatterns` | 1 |
| `ServiceFrame` | `lines` | 1 |
| `ServiceFrame` | `notices` | 1 |
| `ServiceFrame` | `routes` | 1 |
| `ServiceFrame` | `scheduledStopPoints` | 1 |
| `ServiceFrame` | `serviceLinks` | 1 |
| `ServiceFrame` | `stopAssignments` | 1 |
| `SiteFrame` | `TypeOfFrameRef` | 1 |
| `SiteFrame` | `stopPlaces` | 1 |
| `SiteFrame` | `tariffZones` | 1 |
| `SiteFrame` | `topographicPlaces` | 1 |
| `StopPlace` | `ParentSiteRef` | 1 |
| `TimetableFrame` | `TypeOfFrameRef` | 1 |
| `TimetableFrame` | `trainNumbers` | 1 |
| `TimetableFrame` | `vehicleJourneys` | 1 |
| `VehicleType` | `Width` | 1 |
| `codespaces` | `Codespace` | 1 |
| `dataObjects` | `CompositeFrame` | 1 |
| `frames` | `ResourceFrame` | 1 |
| `frames` | `ServiceCalendarFrame` | 1 |
| `frames` | `ServiceFrame` | 1 |
| `frames` | `SiteFrame` | 1 |
| `frames` | `TimetableFrame` | 1 |
| `organisations` | `Authority` | 1 |
