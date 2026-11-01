//! The timetable frame: journeys, their passing times, and the interchanges
//! between them.

use super::{
    AccessibilityAssessment, Codespaces, DirectionType, Duration, FrameDefaults, Id, KeyList,
    LineRef, MultilingualString, NoticeAssignments, Notices, Order, PrivateCode, Ref, StopPointRef,
    StopUse, Time, TimingPointStatus, TransportSubmode, ValidBetween, ValidityConditions,
    VehicleMode, VehicleTypes, Version,
};

netex_element! {
    /// The frame holding the timetable.
    pub struct TimetableFrame {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
        }
        children {
            /// The frame's validity conditions, or the conditions it defines.
            pub validity_conditions: Option<ValidityConditions> = "validityConditions",
            /// When the frame is valid.
            pub valid_between: Vec<ValidBetween> = "ValidBetween",
            /// Key-value pairs.
            pub key_list: Option<KeyList> = "keyList",
            /// The frame's branding.
            pub branding_ref: Option<Ref> = "BrandingRef",
            /// Name of the frame.
            pub name: Option<MultilingualString> = "Name",
            /// What the frame holds.
            pub description: Option<MultilingualString> = "Description",
            /// The profile the frame follows.
            pub type_of_frame_ref: Option<Ref> = "TypeOfFrameRef",
            /// The version frame this one derives from.
            pub baseline_version_frame_ref: Option<Ref> = "BaselineVersionFrameRef",
            /// Codespaces of the identifiers.
            pub codespaces: Option<Codespaces> = "codespaces",
            /// What the entities assume unless they say otherwise.
            pub frame_defaults: Option<FrameDefaults> = "FrameDefaults",
            /// The modes, space-separated.
            pub vehicle_modes: Option<Box<str>> = "VehicleModes",
            /// Whether the journeys are monitored in real time.
            pub monitored: Option<bool> = "Monitored",
            /// The calendar frame the day types are in.
            pub service_calendar_frame_ref: Option<Ref> = "ServiceCalendarFrameRef",
            /// The journeys.
            pub vehicle_journeys: Option<VehicleJourneys> = "vehicleJourneys",
            /// The numbers trains are advertised and run under, named by
            /// their journeys.
            pub train_numbers: Option<TrainNumbers> = "trainNumbers",
            /// Notices.
            pub notices: Option<Notices> = "notices",
            /// Where notices apply.
            pub notice_assignments: Option<NoticeAssignments> = "noticeAssignments",
            /// Planned interchanges between journeys.
            pub journey_interchanges: Option<JourneyInterchanges> = "journeyInterchanges",
            /// Rules for interchanges.
            pub interchange_rules: Option<InterchangeRules> = "interchangeRules",
            /// Vehicle types.
            pub vehicle_types: Option<VehicleTypes> = "vehicleTypes",
        }
    }
}

netex_container! {
    /// The day types a journey runs on.
    pub struct DayTypeRefs of DayTypeRefItem {
        /// A day type.
        DayTypeRef(Ref) = "DayTypeRef",
    }
}

netex_container! {
    /// The train numbers of a journey.
    pub struct TrainNumberRefs of TrainNumberRefItem {
        /// A train number.
        TrainNumberRef(Ref) = "TrainNumberRef",
    }
}

netex_container! {
    /// The train numbers of a timetable frame.
    pub struct TrainNumbers of TrainNumberItem {
        /// A train number.
        TrainNumber(TrainNumber) = "TrainNumber",
        /// A train number another frame defines.
        TrainNumberRef(Ref) = "TrainNumberRef",
    }
}

netex_element! {
    /// A train number: the number a train is advertised under, and the one
    /// it is run under where they differ.
    pub struct TrainNumber {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
        }
        children {
            /// When the entity is valid.
            pub valid_between: Vec<ValidBetween> = "ValidBetween",
            /// Key-value pairs.
            pub key_list: Option<KeyList> = "keyList",
            /// The entity's branding.
            pub branding_ref: Option<Ref> = "BrandingRef",
            /// What it is.
            pub description: Option<MultilingualString> = "Description",
            /// The number passengers are shown.
            pub for_advertisement: Option<Box<str>> = "ForAdvertisement",
            /// The number the train is run under.
            pub for_production: Option<Box<str>> = "ForProduction",
        }
    }
}

netex_choice! {
    /// A reference to a journey of some kind.
    pub enum JourneyRef {
        /// A service journey.
        ServiceJourney(Ref) = "ServiceJourneyRef",
        /// A vehicle journey.
        VehicleJourney(Ref) = "VehicleJourneyRef",
        /// A dated vehicle journey.
        DatedVehicleJourney(Ref) = "DatedVehicleJourneyRef",
        /// A normal dated vehicle journey.
        NormalDatedVehicleJourney(Ref) = "NormalDatedVehicleJourneyRef",
        /// A special service.
        SpecialService(Ref) = "SpecialServiceRef",
        /// A dated special service.
        DatedSpecialService(Ref) = "DatedSpecialServiceRef",
        /// A template service journey.
        TemplateServiceJourney(Ref) = "TemplateServiceJourneyRef",
        /// A single journey.
        SingleJourney(Ref) = "SingleJourneyRef",
        /// A dead run.
        DeadRun(Ref) = "DeadRunRef",
    }
}

netex_choice! {
    /// The journey pattern a journey follows.
    pub enum JourneyPatternRef {
        /// A journey pattern.
        JourneyPattern(Ref) = "JourneyPatternRef",
        /// A service journey pattern.
        ServiceJourneyPattern(Ref) = "ServiceJourneyPatternRef",
        /// A dead run journey pattern.
        DeadRunJourneyPattern(Ref) = "DeadRunJourneyPatternRef",
        /// A service pattern.
        ServicePattern(Ref) = "ServicePatternRef",
    }
}

netex_choice! {
    /// The vehicle type, or the train, a journey is run with.
    pub enum VehicleTypeRef {
        /// A vehicle type.
        VehicleType(Ref) = "VehicleTypeRef",
        /// A compound train.
        CompoundTrain(Ref) = "CompoundTrainRef",
        /// A train.
        Train(Ref) = "TrainRef",
        /// An unpowered train.
        UnpoweredTrain(Ref) = "UnpoweredTrainRef",
        /// A powered train.
        PoweredTrain(Ref) = "PoweredTrainRef",
    }
}

netex_choice! {
    /// The block a journey is part of.
    pub enum BlockRef {
        /// A block.
        Block(Ref) = "BlockRef",
        /// A train block.
        TrainBlock(Ref) = "TrainBlockRef",
    }
}

netex_choice! {
    /// The point in the journey pattern a passing time is at.
    pub enum PointInPatternRef {
        /// A stop point in the journey pattern.
        StopPointInJourneyPattern(Ref) = "StopPointInJourneyPatternRef",
        /// A timing point in the journey pattern.
        TimingPointInJourneyPattern(Ref) = "TimingPointInJourneyPatternRef",
        /// Any point in the journey pattern.
        PointInJourneyPattern(Ref) = "PointInJourneyPatternRef",
        /// A fare point in the pattern.
        FarePointInPattern(Ref) = "FarePointInPatternRef",
        /// A point in a single journey path.
        PointInSingleJourneyPath(Ref) = "PointInSingleJourneyPathRef",
    }
}

netex_element! {
    /// When a journey passes one point of its pattern.
    pub struct TimetabledPassingTime {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
        }
        children {
            /// When the entity is valid.
            pub valid_between: Vec<ValidBetween> = "ValidBetween",
            /// Key-value pairs.
            pub key_list: Option<KeyList> = "keyList",
            /// The journey, when stated on the passing time.
            pub journey_ref: Option<JourneyRef> = choice,
            /// Whether passengers may alight and reboard.
            pub alight_and_reboard: Option<bool> = "AlightAndReboard",
            /// The point.
            pub point_ref: Option<PointInPatternRef> = choice,
            /// Arrival time.
            pub arrival_time: Option<Time> = "ArrivalTime",
            /// Days after the operating day the arrival is on.
            pub arrival_day_offset: Option<i64> = "ArrivalDayOffset",
            /// Departure time.
            pub departure_time: Option<Time> = "DepartureTime",
            /// Days after the operating day the departure is on.
            pub departure_day_offset: Option<i64> = "DepartureDayOffset",
            /// How long the vehicle waits.
            pub waiting_time: Option<Duration> = "WaitingTime",
            /// Latest arrival time, for flexible services.
            pub latest_arrival_time: Option<Time> = "LatestArrivalTime",
            /// Days after the operating day the latest arrival is on.
            pub latest_arrival_day_offset: Option<i64> = "LatestArrivalDayOffset",
            /// Earliest departure time, for flexible services.
            pub earliest_departure_time: Option<Time> = "EarliestDepartureTime",
            /// Days after the operating day the earliest departure is on.
            pub earliest_departure_day_offset: Option<i64> = "EarliestDepartureDayOffset",
        }
    }
}

netex_element! {
    /// The passing times of a journey, in order.
    pub struct PassingTimes {
        attributes {}
        children {
            /// The passing times.
            pub passing_times: Vec<TimetabledPassingTime> = "TimetabledPassingTime",
        }
    }
}

netex_element! {
    /// The arrival or the departure of a call: the time, and whether one may
    /// alight (an arrival) or board (a departure).
    pub struct CallTime {
        attributes {}
        children {
            /// The time.
            pub time: Option<Time> = "Time",
            /// Days after the operating day the time is on.
            pub day_offset: Option<i64> = "DayOffset",
            /// Of an arrival: whether one may alight.
            pub for_alighting: Option<bool> = "ForAlighting",
            /// Of a departure: whether one may board.
            pub for_boarding: Option<bool> = "ForBoarding",
            /// Whether the time is flexible.
            pub is_flexible: Option<bool> = "IsFlexible",
            /// Of a departure: how long the vehicle waits.
            pub wait_time: Option<Duration> = "WaitTime",
        }
    }
}

netex_element! {
    /// One stop of a journey stated in calling order, with its times.
    pub struct Call {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
            /// The `order` attribute.
            pub order: Option<Order> = "order",
        }
        children {
            /// Which visit of the stop point, when the journey calls there more than once.
            pub visit_number: Option<i64> = "VisitNumber",
            /// The stop point.
            pub scheduled_stop_point_ref: Option<StopPointRef> = choice,
            /// Whether it is a timing point.
            pub timing_point_status: Option<TimingPointStatus> = "TimingPointStatus",
            /// The arrival.
            pub arrival: Option<CallTime> = "Arrival",
            /// The departure.
            pub departure: Option<CallTime> = "Departure",
            /// The destination display from here on.
            pub destination_display_ref: Option<Ref> = "DestinationDisplayRef",
            /// Whether the destination display changes here.
            pub change_of_destination_display: Option<bool> = "ChangeOfDestinationDisplay",
            /// Whether the service requirements change here.
            pub change_of_service_requirements: Option<bool> = "ChangeOfServiceRequirements",
            /// Notices about this stop.
            pub notice_assignments: Option<NoticeAssignments> = "noticeAssignments",
            /// Whether the vehicle stops on request only.
            pub request_stop: Option<bool> = "RequestStop",
            /// What the stop is used for.
            pub stop_use: Option<StopUse> = "StopUse",
        }
    }
}

netex_container! {
    /// The calls of a journey, in calling order.
    pub struct Calls of CallItem {
        /// A call.
        Call(Call) = "Call",
    }
}

netex_element! {
    /// A group of runs of a template journey at a headway: from the first
    /// departure to the last, one every interval.
    pub struct HeadwayJourneyGroup {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
        }
        children {
            /// Name.
            pub name: Option<MultilingualString> = "Name",
            /// When the first run leaves.
            pub first_departure_time: Option<Time> = "FirstDepartureTime",
            /// Days after the operating day the first run leaves.
            pub first_day_offset: Option<i64> = "FirstDayOffset",
            /// When the last run leaves.
            pub last_departure_time: Option<Time> = "LastDepartureTime",
            /// Days after the operating day the last run leaves.
            pub last_day_offset: Option<i64> = "LastDayOffset",
            /// How long from one run to the next, as planned.
            pub scheduled_headway_interval: Option<Duration> = "ScheduledHeadwayInterval",
            /// How long at least.
            pub minimum_headway_interval: Option<Duration> = "MinimumHeadwayInterval",
            /// How long at most.
            pub maximum_headway_interval: Option<Duration> = "MaximumHeadwayInterval",
            /// How the headway is shown: `displayAsHeadway`, `displayAsTimes`, …
            pub headway_display: Option<Box<str>> = "HeadwayDisplay",
        }
    }
}

netex_container! {
    /// The groups of runs of a template journey.
    pub struct FrequencyGroups of FrequencyGroupItem {
        /// A group of runs at a headway.
        HeadwayJourneyGroup(HeadwayJourneyGroup) = "HeadwayJourneyGroup",
    }
}

netex_choice! {
    /// The operating day of a dated journey.
    pub enum OperatingDayRef {
        /// By reference.
        OperatingDayRef(Ref) = "OperatingDayRef",
    }
}

netex_element! {
    /// A journey: one run of a vehicle along a journey pattern.
    ///
    /// Read for `<ServiceJourney>`, for `<DatedServiceJourney>`, which adds
    /// the children after `calls` up to `DriverRef`, and for
    /// `<TemplateServiceJourney>`, which adds `frequencyGroups`: the runs of a
    /// template are those its headway groups make, at its calls' times shifted.
    pub struct ServiceJourney {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
        }
        children {
            /// The availability conditions it runs under, named or stated in place.
            pub validity_conditions: Option<ValidityConditions> = "validityConditions",
            /// When the entity is valid.
            pub valid_between: Vec<ValidBetween> = "ValidBetween",
            /// Key-value pairs.
            pub key_list: Option<KeyList> = "keyList",
            /// The entity's branding.
            pub branding_ref: Option<Ref> = "BrandingRef",
            /// Name.
            pub name: Option<MultilingualString> = "Name",
            /// Short name.
            pub short_name: Option<MultilingualString> = "ShortName",
            /// What it is.
            pub description: Option<MultilingualString> = "Description",
            /// Length in metres.
            pub distance: Option<f64> = "Distance",
            /// The operator's own code.
            pub private_code: Option<PrivateCode> = "PrivateCode",
            /// The mode.
            pub transport_mode: Option<VehicleMode> = "TransportMode",
            /// The submode.
            pub transport_submode: Option<TransportSubmode> = "TransportSubmode",
            /// The product category.
            pub type_of_product_category_ref: Option<Ref> = "TypeOfProductCategoryRef",
            /// The kind of service.
            pub type_of_service_ref: Option<Ref> = "TypeOfServiceRef",
            /// Whether it is monitored in real time.
            pub monitored: Option<bool> = "Monitored",
            /// How accessible it is.
            pub accessibility_assessment: Option<AccessibilityAssessment> = "AccessibilityAssessment",
            /// Notices about it.
            pub notice_assignments: Option<NoticeAssignments> = "noticeAssignments",
            /// How it differs from the plan: `cancellation`, `extraJourney`, …
            pub service_alteration: Option<Box<str>> = "ServiceAlteration",
            /// When it leaves its first stop.
            pub departure_time: Option<Time> = "DepartureTime",
            /// Days after the operating day it leaves.
            pub departure_day_offset: Option<i64> = "DepartureDayOffset",
            /// How long it takes.
            pub journey_duration: Option<Duration> = "JourneyDuration",
            /// The day types it runs on.
            pub day_types: Option<DayTypeRefs> = "dayTypes",
            /// The route.
            pub route_ref: Option<Ref> = "RouteRef",
            /// The journey pattern.
            pub journey_pattern_ref: Option<JourneyPatternRef> = choice,
            /// The time demand type.
            pub time_demand_type_ref: Option<Ref> = "TimeDemandTypeRef",
            /// The timing algorithm type.
            pub timing_algorithm_type_ref: Option<Ref> = "TimingAlgorithmTypeRef",
            /// The vehicle type.
            pub vehicle_type_ref: Option<VehicleTypeRef> = choice,
            /// The vehicle.
            pub vehicle_ref: Option<Ref> = "VehicleRef",
            /// The operational context.
            pub operational_context_ref: Option<Ref> = "OperationalContextRef",
            /// The block.
            pub block_ref: Option<BlockRef> = choice,
            /// The course of journeys.
            pub course_of_journeys_ref: Option<Ref> = "CourseOfJourneysRef",
            /// Public code.
            pub public_code: Option<Box<str>> = "PublicCode",
            /// The operator.
            pub operator_ref: Option<Ref> = "OperatorRef",
            /// The line.
            pub line_ref: Option<LineRef> = choice,
            /// Which way.
            pub direction_type: Option<DirectionType> = "DirectionType",
            /// The direction.
            pub direction_ref: Option<Ref> = "DirectionRef",
            /// Train numbers.
            pub train_numbers: Option<TrainNumberRefs> = "trainNumbers",
            /// Whether it is printed in the timetable.
            pub print: Option<bool> = "Print",
            /// Whether it is shown on displays.
            pub dynamic: Option<Box<str>> = "Dynamic",
            /// When it passes each point.
            pub passing_times: Option<PassingTimes> = "passingTimes",
            /// Its stops in calling order, each with its times — in place of a
            /// pattern and passing times, as the Swiss delivery states every
            /// journey.
            pub calls: Option<Calls> = "calls",
            /// For a dated journey: the journey it is a run of.
            pub journey_ref: Option<JourneyRef> = choice,
            /// For a dated journey, as the Nordic profile writes it: the
            /// dated journeys it replaces.
            pub dated_service_journey_refs: Vec<Ref> = "DatedServiceJourneyRef",
            /// For a dated journey: the day.
            pub operating_day_ref: Option<OperatingDayRef> = choice,
            /// For a dated journey: the dated journey pattern.
            pub dated_journey_pattern_ref: Option<Ref> = "DatedJourneyPatternRef",
            /// For a dated journey: the driver.
            pub driver_ref: Option<Ref> = "DriverRef",
            /// For a template journey: the groups of runs it stands for.
            pub frequency_groups: Option<FrequencyGroups> = "frequencyGroups",
        }
    }
}

netex_container! {
    /// The journeys of a timetable frame.
    pub struct VehicleJourneys of VehicleJourney {
        /// A service journey.
        ServiceJourney(ServiceJourney) = "ServiceJourney",
        /// A service journey on one day.
        DatedServiceJourney(ServiceJourney) = "DatedServiceJourney",
        /// A journey standing for runs at a headway.
        TemplateServiceJourney(ServiceJourney) = "TemplateServiceJourney",
    }
}

netex_element! {
    /// A planned interchange from one journey to another at a stop.
    pub struct ServiceJourneyInterchange {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
        }
        children {
            /// When the entity is valid.
            pub valid_between: Vec<ValidBetween> = "ValidBetween",
            /// Key-value pairs.
            pub key_list: Option<KeyList> = "keyList",
            /// The entity's branding.
            pub branding_ref: Option<Ref> = "BrandingRef",
            /// Name.
            pub name: Option<MultilingualString> = "Name",
            /// What it is.
            pub description: Option<MultilingualString> = "Description",
            /// The publisher's own code.
            pub private_code: Option<PrivateCode> = "PrivateCode",
            /// The connection it uses.
            pub connection_ref: Option<Ref> = "ConnectionRef",
            /// Its priority among interchanges.
            pub priority: Option<i64> = "Priority",
            /// Whether passengers stay on the vehicle.
            pub stay_seated: Option<bool> = "StaySeated",
            /// Whether it crosses a border.
            pub cross_border: Option<bool> = "CrossBorder",
            /// Whether it is planned.
            pub planned: Option<bool> = "Planned",
            /// Whether the distributor waits.
            pub guaranteed: Option<bool> = "Guaranteed",
            /// Whether it is shown to passengers.
            pub advertised: Option<bool> = "Advertised",
            /// Whether it is controlled in operation.
            pub controlled: Option<bool> = "Controlled",
            /// How certain it is: `certain`, `probable`, `possible`, …
            pub connection_certainty: Option<Box<str>> = "ConnectionCertainty",
            /// How long the distributor normally waits.
            pub standard_wait_time: Option<Duration> = "StandardWaitTime",
            /// How long it waits at most.
            pub maximum_wait_time: Option<Duration> = "MaximumWaitTime",
            /// How long it waits at most without a decision.
            pub maximum_automatic_wait_time: Option<Duration> = "MaximumAutomaticWaitTime",
            /// How long the transfer normally takes.
            pub standard_transfer_time: Option<Duration> = "StandardTransferTime",
            /// How long it takes at least.
            pub minimum_transfer_time: Option<Duration> = "MinimumTransferTime",
            /// How long it takes at most.
            pub maximum_transfer_time: Option<Duration> = "MaximumTransferTime",
            /// Notices about it.
            pub notice_assignments: Option<NoticeAssignments> = "noticeAssignments",
            /// The stop point the feeder arrives at.
            pub from_point_ref: Option<Ref> = "FromPointRef",
            /// Which visit of the feeder, when it calls there more than once.
            pub from_visit_number: Option<i64> = "FromVisitNumber",
            /// The stop point the distributor leaves from.
            pub to_point_ref: Option<Ref> = "ToPointRef",
            /// Which visit of the distributor.
            pub to_visit_number: Option<i64> = "ToVisitNumber",
            /// The feeder journey.
            pub from_journey_ref: Option<Ref> = "FromJourneyRef",
            /// The distributor journey.
            pub to_journey_ref: Option<Ref> = "ToJourneyRef",
            /// The pattern interchange this is an instance of.
            pub service_journey_pattern_interchange_ref: Option<Ref> = "ServiceJourneyPatternInterchangeRef",
        }
    }
}

netex_container! {
    /// The journey interchanges of a timetable frame.
    pub struct JourneyInterchanges of JourneyInterchangeItem {
        /// A service journey interchange.
        ServiceJourneyInterchange(ServiceJourneyInterchange) = "ServiceJourneyInterchange",
    }
}

netex_element! {
    /// A rule for interchanges: what applies where no interchange is planned.
    pub struct InterchangeRule {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
        }
        children {
            /// When the entity is valid.
            pub valid_between: Vec<ValidBetween> = "ValidBetween",
            /// Key-value pairs.
            pub key_list: Option<KeyList> = "keyList",
            /// The entity's branding.
            pub branding_ref: Option<Ref> = "BrandingRef",
            /// Name.
            pub name: Option<MultilingualString> = "Name",
            /// What it is.
            pub description: Option<MultilingualString> = "Description",
            /// The publisher's own code.
            pub private_code: Option<PrivateCode> = "PrivateCode",
            /// The connection it applies to.
            pub connection_ref: Option<Ref> = "ConnectionRef",
            /// Its priority among rules.
            pub priority: Option<i64> = "Priority",
            /// Whether passengers stay on the vehicle.
            pub stay_seated: Option<bool> = "StaySeated",
            /// Whether it crosses a border.
            pub cross_border: Option<bool> = "CrossBorder",
            /// Whether it is planned.
            pub planned: Option<bool> = "Planned",
            /// Whether the distributor waits.
            pub guaranteed: Option<bool> = "Guaranteed",
            /// Whether it is shown to passengers.
            pub advertised: Option<bool> = "Advertised",
            /// Whether it is controlled in operation.
            pub controlled: Option<bool> = "Controlled",
            /// How certain it is.
            pub connection_certainty: Option<Box<str>> = "ConnectionCertainty",
            /// How long the distributor normally waits.
            pub standard_wait_time: Option<Duration> = "StandardWaitTime",
            /// How long it waits at most.
            pub maximum_wait_time: Option<Duration> = "MaximumWaitTime",
            /// How long it waits at most without a decision.
            pub maximum_automatic_wait_time: Option<Duration> = "MaximumAutomaticWaitTime",
            /// How long the transfer normally takes.
            pub standard_transfer_time: Option<Duration> = "StandardTransferTime",
            /// How long it takes at least.
            pub minimum_transfer_time: Option<Duration> = "MinimumTransferTime",
            /// How long it takes at most.
            pub maximum_transfer_time: Option<Duration> = "MaximumTransferTime",
            /// Notices about it.
            pub notice_assignments: Option<NoticeAssignments> = "noticeAssignments",
            /// The zone the rule applies in.
            pub connection_zone_ref: Option<Ref> = "ConnectionZoneRef",
            /// The control centre in charge.
            pub control_centre_ref: Option<Ref> = "ControlCentreRef",
            /// Whether the rule excludes interchanges rather than allowing them.
            pub exclude: Option<bool> = "Exclude",
        }
    }
}

netex_container! {
    /// The interchange rules of a timetable frame.
    pub struct InterchangeRules of InterchangeRuleItem {
        /// An interchange rule.
        InterchangeRule(InterchangeRule) = "InterchangeRule",
    }
}
