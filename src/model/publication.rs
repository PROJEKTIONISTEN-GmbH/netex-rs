//! The publication: the root element and the frames it delivers.

use super::{
    AvailabilityCondition, Codespaces, DataSource, DateTime, DayType, DayTypeAssignment,
    DestinationDisplay, Direction, Duration, FrameDefaults, GroupOfStopPlaces, Id, InterchangeRule,
    KeyList, Line, MultilingualString, Notice, NoticeAssignment, OperatingDay, OperatingPeriod,
    Organisation, PassengerStopAssignment, PrivateCodes, Quay, Ref, ResourceFrame,
    ResponsibilitySet, Route, RoutePoint, ScheduledStopPoint, ServiceCalendarFrame, ServiceFrame,
    ServiceJourney, ServiceJourneyInterchange, ServiceJourneyPattern, ServiceLink, SiteConnection,
    SiteFrame, StopPlace, TimeDemandType, TimetableFrame, TopographicPlace, TrainNumber,
    ValidBetween, ValidityConditions, VehicleType, Version,
};

netex_element! {
    /// The root of a NeTEx document.
    ///
    /// The `version` attribute is the schema version the document declares:
    /// `1.0` (the default), `1.1`, `ntx:1.1` (the German delivery), `2.0`.
    /// The 2.0 examples mostly still declare `1.0`, so the crate does not take
    /// the attribute's word for it; see [`crate::version::detect`].
    pub struct PublicationDelivery {
        attributes {
            /// The `version` attribute.
            pub version: Option<Box<str>> = "version",
        }
        children {
            /// When the document was produced.
            pub publication_timestamp: Option<DateTime> = "PublicationTimestamp",
            /// Who produced it.
            pub participant_ref: Option<Box<str>> = "ParticipantRef",
            /// How often it is refreshed.
            pub publication_refresh_interval: Option<Duration> = "PublicationRefreshInterval",
            /// What it holds.
            pub description: Option<MultilingualString> = "Description",
            /// The frames delivered.
            pub data_objects: Option<Frames> = "dataObjects",
        }
    }
}

netex_container! {
    /// A list of frames: the root's `dataObjects`, or a composite frame's `frames`.
    pub struct Frames of Frame {
        /// A frame holding other frames.
        Composite(CompositeFrame) = "CompositeFrame",
        /// Organisations, data sources, vehicle types.
        Resource(ResourceFrame) = "ResourceFrame",
        /// Stop places and other sites.
        Site(SiteFrame) = "SiteFrame",
        /// Lines, routes, stop points, journey patterns.
        Service(ServiceFrame) = "ServiceFrame",
        /// Day types and operating periods.
        ServiceCalendar(ServiceCalendarFrame) = "ServiceCalendarFrame",
        /// Journeys and passing times.
        Timetable(TimetableFrame) = "TimetableFrame",
        /// Entities of any kind, side by side: the frame the French profile
        /// writes everything in.
        General(GeneralFrame) = "GeneralFrame",
    }
}

netex_element! {
    /// A frame holding other frames, with defaults they share.
    pub struct CompositeFrame {
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
            /// Codes it is known by in other systems, each with its type.
            pub private_codes: Option<PrivateCodes> = "privateCodes",
            /// The frame's branding.
            pub branding_ref: Option<Ref> = "BrandingRef",
            /// Name of the frame.
            pub name: Option<MultilingualString> = "Name",
            /// What the frame holds.
            pub description: Option<MultilingualString> = "Description",
            /// The profile the frame follows: `epip:EU_PI_LINE_OFFER`.
            pub type_of_frame_ref: Option<Ref> = "TypeOfFrameRef",
            /// The version frame this one derives from.
            pub baseline_version_frame_ref: Option<Ref> = "BaselineVersionFrameRef",
            /// Codespaces of the identifiers.
            pub codespaces: Option<Codespaces> = "codespaces",
            /// What the entities assume unless they say otherwise.
            pub frame_defaults: Option<FrameDefaults> = "FrameDefaults",
            /// The availability conditions the frame's content shares, the
            /// Dutch deliveries' calendar.
            pub content_validity_conditions: Option<ValidityConditions> = "contentValidityConditions",
            /// The frames held.
            pub frames: Option<Frames> = "frames",
        }
    }
}

netex_element! {
    /// A frame holding entities of any kind in one list, its `members`. The
    /// French profile (NeTEx France) writes every part of a delivery so — its
    /// stops, its network, its calendar, its timetable —, naming the part by
    /// the frame's `TypeOfFrameRef`: `FR:TypeOfFrame:NETEX_ARRET`, …
    pub struct GeneralFrame {
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
            /// Codes it is known by in other systems, each with its type.
            pub private_codes: Option<PrivateCodes> = "privateCodes",
            /// The frame's branding.
            pub branding_ref: Option<Ref> = "BrandingRef",
            /// Name of the frame.
            pub name: Option<MultilingualString> = "Name",
            /// What the frame holds.
            pub description: Option<MultilingualString> = "Description",
            /// The part of the profile the frame holds: `FR:TypeOfFrame:NETEX_HORAIRE`, …
            pub type_of_frame_ref: Option<Ref> = "TypeOfFrameRef",
            /// The version frame this one derives from.
            pub baseline_version_frame_ref: Option<Ref> = "BaselineVersionFrameRef",
            /// Codespaces of the identifiers.
            pub codespaces: Option<Codespaces> = "codespaces",
            /// What the entities assume unless they say otherwise.
            pub frame_defaults: Option<FrameDefaults> = "FrameDefaults",
            /// The availability conditions the frame's content shares, the
            /// Dutch deliveries' calendar.
            pub content_validity_conditions: Option<ValidityConditions> = "contentValidityConditions",
            /// The entities, in document order.
            pub members: Option<GeneralFrameMembers> = "members",
        }
    }
}

netex_container! {
    /// The entities of a general frame: every kind the specific frames hold
    /// that the model types, and a quay standing on its own.
    pub struct GeneralFrameMembers of GeneralFrameMember {
        /// A town, district or region.
        TopographicPlace(TopographicPlace) = "TopographicPlace",
        /// A group of stop places.
        GroupOfStopPlaces(GroupOfStopPlaces) = "GroupOfStopPlaces",
        /// A stop place.
        StopPlace(StopPlace) = "StopPlace",
        /// A quay defined beside its stop place, which names it by a
        /// `QuayRef` among its quays.
        Quay(Quay) = "Quay",
        /// A data source.
        DataSource(DataSource) = "DataSource",
        /// A responsibility set.
        ResponsibilitySet(ResponsibilitySet) = "ResponsibilitySet",
        /// An operator.
        Operator(Organisation) = "Operator",
        /// An authority.
        Authority(Organisation) = "Authority",
        /// A vehicle type.
        VehicleType(VehicleType) = "VehicleType",
        /// A direction of travel.
        Direction(Direction) = "Direction",
        /// A point a route goes through.
        RoutePoint(RoutePoint) = "RoutePoint",
        /// A route.
        Route(Route) = "Route",
        /// A line.
        Line(Line) = "Line",
        /// A line served on demand.
        FlexibleLine(Line) = "FlexibleLine",
        /// A destination display.
        DestinationDisplay(DestinationDisplay) = "DestinationDisplay",
        /// A scheduled stop point.
        ScheduledStopPoint(ScheduledStopPoint) = "ScheduledStopPoint",
        /// A link between stop points.
        ServiceLink(ServiceLink) = "ServiceLink",
        /// A walk between sites.
        SiteConnection(SiteConnection) = "SiteConnection",
        /// Which stop place or quay a stop point is.
        PassengerStopAssignment(PassengerStopAssignment) = "PassengerStopAssignment",
        /// A service journey pattern.
        ServiceJourneyPattern(ServiceJourneyPattern) = "ServiceJourneyPattern",
        /// A journey pattern.
        JourneyPattern(ServiceJourneyPattern) = "JourneyPattern",
        /// A notice.
        Notice(Notice) = "Notice",
        /// Where a notice applies.
        NoticeAssignment(NoticeAssignment) = "NoticeAssignment",
        /// A time demand type.
        TimeDemandType(TimeDemandType) = "TimeDemandType",
        /// A day type.
        DayType(DayType) = "DayType",
        /// An operating day.
        OperatingDay(OperatingDay) = "OperatingDay",
        /// An operating period.
        OperatingPeriod(OperatingPeriod) = "OperatingPeriod",
        /// An operating period with a bit per day.
        UicOperatingPeriod(OperatingPeriod) = "UicOperatingPeriod",
        /// Which day type a day is.
        DayTypeAssignment(DayTypeAssignment) = "DayTypeAssignment",
        /// The days an entity is available on.
        AvailabilityCondition(AvailabilityCondition) = "AvailabilityCondition",
        /// A journey.
        ServiceJourney(ServiceJourney) = "ServiceJourney",
        /// A journey on one day.
        DatedServiceJourney(ServiceJourney) = "DatedServiceJourney",
        /// A journey standing for runs at a headway.
        TemplateServiceJourney(ServiceJourney) = "TemplateServiceJourney",
        /// A train number.
        TrainNumber(TrainNumber) = "TrainNumber",
        /// A planned interchange between journeys.
        ServiceJourneyInterchange(ServiceJourneyInterchange) = "ServiceJourneyInterchange",
        /// A rule for interchanges.
        InterchangeRule(InterchangeRule) = "InterchangeRule",
    }
}

impl PublicationDelivery {
    /// The frames delivered, composite frames opened: every frame at any depth,
    /// in document order, the composite frames themselves included.
    pub fn frames(&self) -> impl Iterator<Item = &Frame> {
        let mut pending: Vec<&Frame> = self
            .data_objects
            .iter()
            .flat_map(|frames| frames.items.iter().rev())
            .collect();
        std::iter::from_fn(move || {
            let frame = pending.pop()?;
            if let Frame::Composite(composite) = frame {
                if let Some(frames) = &composite.frames {
                    pending.extend(frames.items.iter().rev());
                }
            }
            Some(frame)
        })
    }
}
