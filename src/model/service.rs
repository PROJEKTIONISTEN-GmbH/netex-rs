//! The service frame: lines, routes, stop points, journey patterns, notices.

use super::{
    Codespaces, Connections, DirectionType, Duration, FrameDefaults, Id, KeyList, Location,
    MultilingualString, Order, PrivateCode, Ref, StopType, StopUse, TimingPointStatus,
    ValidBetween, ValidityConditions, VehicleMode, Version,
};

netex_element! {
    /// The frame holding the network as passengers see it.
    pub struct ServiceFrame {
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
            /// Directions.
            pub directions: Option<Directions> = "directions",
            /// Points routes go through.
            pub route_points: Option<RoutePoints> = "routePoints",
            /// Routes.
            pub routes: Option<Routes> = "routes",
            /// Lines.
            pub lines: Option<Lines> = "lines",
            /// Destination displays.
            pub destination_displays: Option<DestinationDisplays> = "destinationDisplays",
            /// Scheduled stop points.
            pub scheduled_stop_points: Option<ScheduledStopPoints> = "scheduledStopPoints",
            /// Links between stop points.
            pub service_links: Option<ServiceLinks> = "serviceLinks",
            /// Walks between sites.
            pub connections: Option<Connections> = "connections",
            /// Which stop place or quay a stop point is.
            pub stop_assignments: Option<StopAssignments> = "stopAssignments",
            /// Journey patterns.
            pub journey_patterns: Option<JourneyPatterns> = "journeyPatterns",
            /// Notices.
            pub notices: Option<Notices> = "notices",
            /// Where notices apply.
            pub notice_assignments: Option<NoticeAssignments> = "noticeAssignments",
        }
    }
}

netex_element! {
    /// A direction of travel along a line.
    pub struct Direction {
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
            /// Short name.
            pub short_name: Option<MultilingualString> = "ShortName",
            /// What it is.
            pub description: Option<MultilingualString> = "Description",
            /// The publisher's own code.
            pub private_code: Option<PrivateCode> = "PrivateCode",
            /// Which way: `inbound`, `outbound`, …
            pub direction_type: Option<DirectionType> = "DirectionType",
            /// The direction the other way.
            pub opposite_direction_ref: Option<Ref> = "OppositeDirectionRef",
        }
    }
}

netex_container! {
    /// The directions of a service frame.
    pub struct Directions of DirectionItem {
        /// A direction.
        Direction(Direction) = "Direction",
    }
}

netex_element! {
    /// A direction a line may be run in.
    pub struct AllowedLineDirection {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
        }
        children {
            /// The line.
            pub line_ref: Option<Ref> = "LineRef",
            /// The direction.
            pub direction_ref: Option<Ref> = "DirectionRef",
        }
    }
}

netex_element! {
    /// The directions a line may be run in.
    pub struct AllowedDirections {
        attributes {}
        children {
            /// The directions.
            pub allowed_line_directions: Vec<AllowedLineDirection> = "AllowedLineDirection",
        }
    }
}

netex_container! {
    /// The operators of a line besides its main one.
    pub struct AdditionalOperators of AdditionalOperator {
        /// An operator.
        OperatorRef(Ref) = "OperatorRef",
    }
}

netex_choice! {
    /// The submode of a mode: one element per mode.
    pub enum Submode {
        /// An air submode.
        Air(Box<str>) = "AirSubmode",
        /// A bus submode: `regionalBus`, `expressBus`, …
        Bus(Box<str>) = "BusSubmode",
        /// A coach submode.
        Coach(Box<str>) = "CoachSubmode",
        /// A funicular submode.
        Funicular(Box<str>) = "FunicularSubmode",
        /// A metro submode.
        Metro(Box<str>) = "MetroSubmode",
        /// A tram submode.
        Tram(Box<str>) = "TramSubmode",
        /// A telecabin submode.
        Telecabin(Box<str>) = "TelecabinSubmode",
        /// A rail submode: `regionalRail`, `interregionalRail`, …
        Rail(Box<str>) = "RailSubmode",
        /// A water submode.
        Water(Box<str>) = "WaterSubmode",
        /// A snow-and-ice submode.
        SnowAndIce(Box<str>) = "SnowAndIceSubmode",
        /// A taxi submode.
        Taxi(Box<str>) = "TaxiSubmode",
        /// A self-drive submode.
        SelfDrive(Box<str>) = "SelfDriveSubmode",
    }
}

netex_element! {
    /// The submode of a line or journey.
    pub struct TransportSubmode {
        attributes {}
        children {
            /// The submode.
            pub submode: Option<Submode> = choice,
        }
    }
}

netex_element! {
    /// A line: a named service passengers know.
    pub struct Line {
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
            /// Short name.
            pub short_name: Option<MultilingualString> = "ShortName",
            /// What it is.
            pub description: Option<MultilingualString> = "Description",
            /// The mode.
            pub transport_mode: Option<VehicleMode> = "TransportMode",
            /// The submode.
            pub transport_submode: Option<TransportSubmode> = "TransportSubmode",
            /// The number passengers see.
            pub public_code: Option<Box<str>> = "PublicCode",
            /// The operator's own code.
            pub private_code: Option<PrivateCode> = "PrivateCode",
            /// The authority ordering it.
            pub authority_ref: Option<Ref> = "AuthorityRef",
            /// The operator running it.
            pub operator_ref: Option<Ref> = "OperatorRef",
            /// Other operators running it.
            pub additional_operators: Option<AdditionalOperators> = "additionalOperators",
            /// The operational context.
            pub operational_context_ref: Option<Ref> = "OperationalContextRef",
            /// What kind of line: `local`, `express`, …
            pub line_type: Option<Box<str>> = "LineType",
            /// What kind of line, by reference.
            pub type_of_line_ref: Option<Ref> = "TypeOfLineRef",
            /// The product category.
            pub type_of_product_category_ref: Option<Ref> = "TypeOfProductCategoryRef",
            /// The kind of service.
            pub type_of_service_ref: Option<Ref> = "TypeOfServiceRef",
            /// Whether it is monitored in real time.
            pub monitored: Option<bool> = "Monitored",
            /// The group of lines it is shown under.
            pub represented_by_group_ref: Option<Ref> = "RepresentedByGroupRef",
            /// The directions it runs in.
            pub allowed_directions: Option<AllowedDirections> = "allowedDirections",
            /// Notices about it.
            pub notice_assignments: Option<NoticeAssignments> = "noticeAssignments",
        }
    }
}

netex_container! {
    /// The lines of a service frame.
    pub struct Lines of LineItem {
        /// A line.
        Line(Line) = "Line",
    }
}

netex_element! {
    /// A point a route goes through.
    pub struct RoutePoint {
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
            /// Where it is.
            pub location: Option<Location> = "Location",
            /// The point's number.
            pub point_number: Option<Box<str>> = "PointNumber",
            /// Whether it is a via point.
            pub via_flag: Option<bool> = "ViaFlag",
            /// Whether it is at a border.
            pub border_crossing: Option<bool> = "BorderCrossing",
        }
    }
}

netex_container! {
    /// The route points of a service frame.
    pub struct RoutePoints of RoutePointItem {
        /// A route point.
        RoutePoint(RoutePoint) = "RoutePoint",
    }
}

netex_choice! {
    /// The point a point on a route stands at.
    pub enum PointOnRouteRef {
        /// A route point.
        RoutePoint(Ref) = "RoutePointRef",
        /// A scheduled stop point.
        ScheduledStopPoint(Ref) = "ScheduledStopPointRef",
        /// Any point.
        Point(Ref) = "PointRef",
    }
}

netex_element! {
    /// One point of a route, in order.
    pub struct PointOnRoute {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
            /// The `order` attribute.
            pub order: Option<Order> = "order",
        }
        children {
            /// What it is.
            pub description: Option<MultilingualString> = "Description",
            /// The point.
            pub point_ref: Option<PointOnRouteRef> = choice,
            /// The link onward from it.
            pub onward_route_link_ref: Option<Ref> = "OnwardRouteLinkRef",
        }
    }
}

netex_container! {
    /// The points of a route, in order.
    pub struct PointsOnRoute of PointOnRouteItem {
        /// A point on the route.
        PointOnRoute(PointOnRoute) = "PointOnRoute",
    }
}

netex_choice! {
    /// The line a route or journey belongs to.
    pub enum LineRef {
        /// A line.
        Line(Ref) = "LineRef",
        /// A flexible line.
        FlexibleLine(Ref) = "FlexibleLineRef",
    }
}

netex_element! {
    /// A route: the path through the infrastructure a line's journeys follow.
    pub struct Route {
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
            /// Short name.
            pub short_name: Option<MultilingualString> = "ShortName",
            /// What it is.
            pub description: Option<MultilingualString> = "Description",
            /// Length in metres.
            pub distance: Option<f64> = "Distance",
            /// The operator's own code.
            pub private_code: Option<PrivateCode> = "PrivateCode",
            /// The line.
            pub line_ref: Option<LineRef> = choice,
            /// Which way.
            pub direction_type: Option<DirectionType> = "DirectionType",
            /// The direction.
            pub direction_ref: Option<Ref> = "DirectionRef",
            /// The points, in order.
            pub points_in_sequence: Option<PointsOnRoute> = "pointsInSequence",
            /// The route the other way.
            pub inverse_route_ref: Option<Ref> = "InverseRouteRef",
        }
    }
}

netex_container! {
    /// The routes of a service frame.
    pub struct Routes of RouteItem {
        /// A route.
        Route(Route) = "Route",
    }
}

netex_element! {
    /// What the vehicle shows as its destination.
    pub struct DestinationDisplay {
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
            /// Short name.
            pub short_name: Option<MultilingualString> = "ShortName",
            /// The text on the side of the vehicle.
            pub side_text: Option<MultilingualString> = "SideText",
            /// The text on the front of the vehicle.
            pub front_text: Option<MultilingualString> = "FrontText",
            /// The text the driver sees.
            pub driver_display_text: Option<MultilingualString> = "DriverDisplayText",
            /// Short code.
            pub short_code: Option<Box<str>> = "ShortCode",
            /// Public code.
            pub public_code: Option<Box<str>> = "PublicCode",
            /// The operator's own code.
            pub private_code: Option<PrivateCode> = "PrivateCode",
        }
    }
}

netex_container! {
    /// The destination displays of a service frame.
    pub struct DestinationDisplays of DestinationDisplayItem {
        /// A destination display.
        DestinationDisplay(DestinationDisplay) = "DestinationDisplay",
    }
}

netex_element! {
    /// A scheduled stop point: a stop as the timetable sees it.
    pub struct ScheduledStopPoint {
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
            /// Where it is.
            pub location: Option<Location> = "Location",
            /// The point's number.
            pub point_number: Option<Box<str>> = "PointNumber",
            /// Whether it is a timing point.
            pub timing_point_status: Option<TimingPointStatus> = "TimingPointStatus",
            /// How long a vehicle may wait here.
            pub allowed_for_wait_time: Option<Duration> = "AllowedForWaitTime",
            /// Short name.
            pub short_name: Option<MultilingualString> = "ShortName",
            /// What it is.
            pub description: Option<MultilingualString> = "Description",
            /// The label on it.
            pub label: Option<MultilingualString> = "Label",
            /// Short stop code.
            pub short_stop_code: Option<Box<str>> = "ShortStopCode",
            /// Public code.
            pub public_code: Option<Box<str>> = "PublicCode",
            /// The publisher's own code.
            pub private_code: Option<PrivateCode> = "PrivateCode",
            /// What kind of stop.
            pub stop_type: Option<StopType> = "StopType",
            /// Bearing of vehicles leaving, in degrees.
            pub compass_bearing: Option<f64> = "CompassBearing",
            /// The modes served, space-separated.
            pub vehicle_modes: Option<Box<str>> = "VehicleModes",
            /// Whether one may alight here.
            pub for_alighting: Option<bool> = "ForAlighting",
            /// Whether one may board here.
            pub for_boarding: Option<bool> = "ForBoarding",
            /// Whether the vehicle stops on request only.
            pub request_stop: Option<bool> = "RequestStop",
            /// Country.
            pub country_ref: Option<Ref> = "CountryRef",
            /// The town or district it is in.
            pub topographic_place_ref: Option<Ref> = "TopographicPlaceRef",
            /// Whether it is at the centre of that place.
            pub at_centre: Option<bool> = "AtCentre",
        }
    }
}

netex_container! {
    /// The scheduled stop points of a service frame.
    pub struct ScheduledStopPoints of ScheduledStopPointItem {
        /// A scheduled stop point.
        ScheduledStopPoint(ScheduledStopPoint) = "ScheduledStopPoint",
    }
}

netex_element! {
    /// A link between two scheduled stop points.
    pub struct ServiceLink {
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
            /// Length in metres.
            pub distance: Option<f64> = "Distance",
            /// Where it starts.
            pub from_point_ref: Option<Ref> = "FromPointRef",
            /// Where it ends.
            pub to_point_ref: Option<Ref> = "ToPointRef",
            /// The mode.
            pub vehicle_mode: Option<VehicleMode> = "VehicleMode",
            /// The operational context.
            pub operational_context_ref: Option<Ref> = "OperationalContextRef",
        }
    }
}

netex_container! {
    /// The service links of a service frame.
    pub struct ServiceLinks of ServiceLinkItem {
        /// A service link.
        ServiceLink(ServiceLink) = "ServiceLink",
    }
}

netex_element! {
    /// Which stop place, and which quay of it, a scheduled stop point is.
    pub struct PassengerStopAssignment {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
            /// The `order` attribute.
            pub order: Option<Order> = "order",
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
            /// Whether one may board here.
            pub boarding_use: Option<bool> = "BoardingUse",
            /// Whether one may alight here.
            pub alighting_use: Option<bool> = "AlightingUse",
            /// The publisher's own code.
            pub private_code: Option<PrivateCode> = "PrivateCode",
            /// The scheduled stop point.
            pub scheduled_stop_point_ref: Option<Ref> = "ScheduledStopPointRef",
            /// The stop place.
            pub stop_place_ref: Option<Ref> = "StopPlaceRef",
            /// The quay.
            pub quay_ref: Option<Ref> = "QuayRef",
            /// The boarding position.
            pub boarding_position_ref: Option<Ref> = "BoardingPositionRef",
        }
    }
}

netex_container! {
    /// The stop assignments of a service frame.
    pub struct StopAssignments of StopAssignmentItem {
        /// A passenger stop assignment.
        PassengerStopAssignment(PassengerStopAssignment) = "PassengerStopAssignment",
    }
}

netex_element! {
    /// A route seen from a journey pattern, by reference and by line.
    pub struct RouteView {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
        }
        children {
            /// The branding.
            pub branding_ref: Option<Ref> = "BrandingRef",
            /// The route.
            pub route_ref: Option<Ref> = "RouteRef",
            /// The mode.
            pub vehicle_mode: Option<VehicleMode> = "VehicleMode",
            /// Name.
            pub name: Option<MultilingualString> = "Name",
            /// The line.
            pub line_ref: Option<LineRef> = choice,
        }
    }
}

netex_choice! {
    /// The route of a journey pattern, by reference or as a view.
    pub enum RouteRefOrView {
        /// By reference.
        RouteRef(Ref) = "RouteRef",
        /// As a view.
        RouteView(Box<RouteView>) = "RouteView",
    }
}

netex_choice! {
    /// The notice a notice assignment assigns.
    pub enum NoticeOrRef {
        /// A notice defined here.
        Notice(Box<Notice>) = "Notice",
        /// A notice defined elsewhere.
        NoticeRef(Ref) = "NoticeRef",
        /// A group of notices.
        GroupOfNoticesRef(Ref) = "GroupOfNoticesRef",
    }
}

netex_element! {
    /// A notice: a text about a line, journey or stop.
    pub struct Notice {
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
            /// Short name.
            pub short_name: Option<MultilingualString> = "ShortName",
            /// The text.
            pub text: Option<MultilingualString> = "Text",
            /// Public code; the German delivery's notice catalogue key.
            pub public_code: Option<Box<str>> = "PublicCode",
            /// Short code.
            pub short_code: Option<Box<str>> = "ShortCode",
            /// The publisher's own code.
            pub private_code: Option<PrivateCode> = "PrivateCode",
            /// What kind of notice.
            pub type_of_notice_ref: Option<Ref> = "TypeOfNoticeRef",
            /// Whether it may be shown to passengers.
            pub can_be_advertised: Option<bool> = "CanBeAdvertised",
            /// The text the driver sees.
            pub driver_display_text: Option<MultilingualString> = "DriverDisplayText",
        }
    }
}

netex_container! {
    /// The notices of a frame.
    pub struct Notices of NoticeItem {
        /// A notice.
        Notice(Notice) = "Notice",
    }
}

netex_element! {
    /// Where a notice applies.
    pub struct NoticeAssignment {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
            /// The `order` attribute.
            pub order: Option<Order> = "order",
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
            /// The notice.
            pub notice: Option<NoticeOrRef> = choice,
            /// What the notice is about.
            pub noticed_object_ref: Option<Ref> = "NoticedObjectRef",
            /// Where in the pattern it starts to apply.
            pub start_point_in_pattern_ref: Option<Ref> = "StartPointInPatternRef",
            /// Where in the pattern it stops applying.
            pub end_point_in_pattern_ref: Option<Ref> = "EndPointInPatternRef",
            /// A mark to show.
            pub mark: Option<Box<str>> = "Mark",
            /// The mark's web address.
            pub mark_url: Option<Box<str>> = "MarkUrl",
            /// Where it is published.
            pub publicity_channel: Option<Box<str>> = "PublicityChannel",
            /// Whether it is shown to passengers.
            pub advertised: Option<bool> = "Advertised",
        }
    }
}

netex_element! {
    /// The notice assignments of an entity or frame.
    pub struct NoticeAssignments {
        attributes {}
        children {
            /// The assignments.
            pub notice_assignments: Vec<NoticeAssignment> = "NoticeAssignment",
        }
    }
}

netex_choice! {
    /// The scheduled stop point of a stop in a journey pattern.
    pub enum StopPointRef {
        /// A scheduled stop point.
        ScheduledStopPoint(Ref) = "ScheduledStopPointRef",
        /// A fare scheduled stop point.
        FareScheduledStopPoint(Ref) = "FareScheduledStopPointRef",
    }
}

netex_element! {
    /// One stop of a journey pattern, in order.
    pub struct StopPointInJourneyPattern {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
            /// The `order` attribute.
            pub order: Option<Order> = "order",
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
            /// The stop point.
            pub scheduled_stop_point_ref: Option<StopPointRef> = choice,
            /// The timing link onward.
            pub onward_timing_link_ref: Option<Ref> = "OnwardTimingLinkRef",
            /// Whether the vehicle waits here to keep time.
            pub is_wait_point: Option<bool> = "IsWaitPoint",
            /// How long it waits.
            pub wait_time: Option<Duration> = "WaitTime",
            /// The service link onward.
            pub onward_service_link_ref: Option<Ref> = "OnwardServiceLinkRef",
            /// Whether one may alight here.
            pub for_alighting: Option<bool> = "ForAlighting",
            /// Whether one may board here.
            pub for_boarding: Option<bool> = "ForBoarding",
            /// The destination display from here on.
            pub destination_display_ref: Option<Ref> = "DestinationDisplayRef",
            /// Whether the destination display changes here.
            pub change_of_destination_display: Option<bool> = "ChangeOfDestinationDisplay",
            /// Whether the service requirements change here.
            pub change_of_service_requirements: Option<bool> = "ChangeOfServiceRequirements",
            /// Notices about this stop; the German delivery's accessibility facts.
            pub notice_assignments: Option<NoticeAssignments> = "noticeAssignments",
            /// Whether the vehicle stops on request only.
            pub request_stop: Option<bool> = "RequestStop",
            /// How to request it.
            pub request_method: Option<Box<str>> = "RequestMethod",
            /// How to request boarding.
            pub boarding_request_method: Option<Box<str>> = "BoardingRequestMethod",
            /// How to request alighting.
            pub alighting_request_method: Option<Box<str>> = "AlightingRequestMethod",
            /// What the stop is used for.
            pub stop_use: Option<StopUse> = "StopUse",
            /// Whether it is printed in the timetable.
            pub print: Option<bool> = "Print",
            /// Whether it is shown on displays.
            pub dynamic: Option<Box<str>> = "Dynamic",
        }
    }
}

netex_container! {
    /// The stops of a journey pattern, in order.
    pub struct PointsInSequence of PointInSequenceItem {
        /// A stop.
        StopPointInJourneyPattern(StopPointInJourneyPattern) = "StopPointInJourneyPattern",
    }
}

netex_element! {
    /// A journey pattern: the stops a journey calls at, in order.
    ///
    /// Read for `<ServiceJourneyPattern>` and for `<JourneyPattern>`, which
    /// share every child but the last.
    pub struct ServiceJourneyPattern {
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
            /// Short name.
            pub short_name: Option<MultilingualString> = "ShortName",
            /// What it is.
            pub description: Option<MultilingualString> = "Description",
            /// Length in metres.
            pub distance: Option<f64> = "Distance",
            /// The operator's own code.
            pub private_code: Option<PrivateCode> = "PrivateCode",
            /// The route.
            pub route: Option<RouteRefOrView> = choice,
            /// Which way.
            pub direction_type: Option<DirectionType> = "DirectionType",
            /// The direction.
            pub direction_ref: Option<Ref> = "DirectionRef",
            /// The destination display.
            pub destination_display_ref: Option<Ref> = "DestinationDisplayRef",
            /// What kind of pattern.
            pub type_of_journey_pattern_ref: Option<Ref> = "TypeOfJourneyPatternRef",
            /// The operational context.
            pub operational_context_ref: Option<Ref> = "OperationalContextRef",
            /// The timing pattern.
            pub timing_pattern_ref: Option<Ref> = "TimingPatternRef",
            /// Notices about the pattern.
            pub notice_assignments: Option<NoticeAssignments> = "noticeAssignments",
            /// The stops, in order.
            pub points_in_sequence: Option<PointsInSequence> = "pointsInSequence",
            /// What kind of service pattern: `passenger`, `garageRunOut`, …
            pub service_journey_pattern_type: Option<Box<str>> = "ServiceJourneyPatternType",
        }
    }
}

netex_container! {
    /// The journey patterns of a service frame.
    pub struct JourneyPatterns of JourneyPatternItem {
        /// A service journey pattern.
        ServiceJourneyPattern(ServiceJourneyPattern) = "ServiceJourneyPattern",
        /// A plain journey pattern.
        JourneyPattern(ServiceJourneyPattern) = "JourneyPattern",
    }
}
