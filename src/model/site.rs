//! The site frame: stop places, their quays, levels and entrances, and the
//! walks between them.

use super::{
    AccessMode, AccessSpaceType, AlternativeNames, Centroid, Codespaces, Duration, EntranceType,
    FrameDefaults, Id, KeyList, LimitationStatus, Locale, MultilingualString, PrivateCode,
    QuayType, Ref, StopType, Submode, ValidBetween, ValidityConditions, VehicleMode, Version,
};

netex_element! {
    /// The frame holding sites: stop places, and the places they are in.
    pub struct SiteFrame {
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
            /// Towns, districts, regions.
            pub topographic_places: Option<TopographicPlaces> = "topographicPlaces",
            /// Groups of stop places.
            pub groups_of_stop_places: Option<GroupsOfStopPlaces> = "groupsOfStopPlaces",
            /// Stop places.
            pub stop_places: Option<StopPlaces> = "stopPlaces",
        }
    }
}

netex_element! {
    /// The name of a topographic place, qualified.
    pub struct TopographicPlaceDescriptor {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
        }
        children {
            /// The name.
            pub name: Option<MultilingualString> = "Name",
            /// Its short form.
            pub short_name: Option<MultilingualString> = "ShortName",
        }
    }
}

netex_element! {
    /// A town, district or region.
    pub struct TopographicPlace {
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
            /// Its centre.
            pub centroid: Option<Centroid> = "Centroid",
            /// The zone it is part of.
            pub parent_zone_ref: Option<Ref> = "ParentZoneRef",
            /// Its ISO 3166-2 code; the German delivery puts the municipality key here.
            pub iso_code: Option<Box<str>> = "IsoCode",
            /// Its qualified name.
            pub descriptor: Option<TopographicPlaceDescriptor> = "Descriptor",
            /// What kind of place: `town`, `city`, `region`, …
            pub topographic_place_type: Option<Box<str>> = "TopographicPlaceType",
            /// Postcode.
            pub post_code: Option<Box<str>> = "PostCode",
            /// Country.
            pub country_ref: Option<Ref> = "CountryRef",
            /// The place it is part of.
            pub parent_topographic_place_ref: Option<Ref> = "ParentTopographicPlaceRef",
        }
    }
}

netex_container! {
    /// The topographic places of a site frame.
    pub struct TopographicPlaces of TopographicPlaceItem {
        /// A topographic place.
        TopographicPlace(TopographicPlace) = "TopographicPlace",
    }
}

netex_element! {
    /// One limitation of accessibility, feature by feature.
    pub struct AccessibilityLimitation {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
        }
        children {
            /// When the entity is valid.
            pub valid_between: Vec<ValidBetween> = "ValidBetween",
            /// Whether a wheelchair can pass.
            pub wheelchair_access: Option<LimitationStatus> = "WheelchairAccess",
            /// Whether there are no steps.
            pub step_free_access: Option<LimitationStatus> = "StepFreeAccess",
            /// Whether there are no stairs.
            pub stair_free_access: Option<LimitationStatus> = "StairFreeAccess",
            /// Whether no escalator has to be used.
            pub escalator_free_access: Option<LimitationStatus> = "EscalatorFreeAccess",
            /// Whether no lift has to be used.
            pub lift_free_access: Option<LimitationStatus> = "LiftFreeAccess",
            /// Whether no ramp has to be used.
            pub ramp_free_access: Option<LimitationStatus> = "RampFreeAccess",
            /// Whether the vehicle is boarded at level.
            pub level_access_into_vehicle: Option<LimitationStatus> = "LevelAccessIntoVehicle",
            /// Whether there are audible signals.
            pub audible_signals_available: Option<LimitationStatus> = "AudibleSignalsAvailable",
            /// Whether there are visual signs.
            pub visual_signs_available: Option<LimitationStatus> = "VisualSignsAvailable",
            /// Whether there is tactile guidance.
            pub tactile_guidance_available: Option<LimitationStatus> = "TactileGuidanceAvailable",
            /// Whether a guide dog may come.
            pub guide_dog_access: Option<LimitationStatus> = "GuideDogAccess",
        }
    }
}

netex_element! {
    /// The limitations of an accessibility assessment.
    pub struct Limitations {
        attributes {}
        children {
            /// The limitations.
            pub limitations: Vec<AccessibilityLimitation> = "AccessibilityLimitation",
        }
    }
}

netex_element! {
    /// How accessible a site or vehicle is.
    pub struct AccessibilityAssessment {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
        }
        children {
            /// When the entity is valid.
            pub valid_between: Vec<ValidBetween> = "ValidBetween",
            /// Whether mobility-impaired passengers can use it.
            pub mobility_impaired_access: Option<LimitationStatus> = "MobilityImpairedAccess",
            /// The limitations, feature by feature.
            pub limitations: Option<Limitations> = "limitations",
            /// A remark.
            pub comment: Option<MultilingualString> = "Comment",
        }
    }
}

netex_element! {
    /// A floor of a site.
    pub struct Level {
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
            /// Name.
            pub name: Option<MultilingualString> = "Name",
            /// Short name: `UG`, `EG`, `1`.
            pub short_name: Option<MultilingualString> = "ShortName",
            /// What it is.
            pub description: Option<MultilingualString> = "Description",
            /// Public code.
            pub public_code: Option<Box<str>> = "PublicCode",
            /// The level's order among the site's levels, ground being 0.
            pub relative_level_order: Option<i64> = "RelativeLevelOrder",
            /// How accessible it is.
            pub accessibility_assessment: Option<AccessibilityAssessment> = "AccessibilityAssessment",
        }
    }
}

netex_element! {
    /// The levels of a site.
    pub struct Levels {
        attributes {}
        children {
            /// The levels.
            pub levels: Vec<Level> = "Level",
        }
    }
}

netex_element! {
    /// An entrance to a stop place.
    pub struct StopPlaceEntrance {
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
            /// Where it is.
            pub centroid: Option<Centroid> = "Centroid",
            /// How accessible it is.
            pub accessibility_assessment: Option<AccessibilityAssessment> = "AccessibilityAssessment",
            /// Other names.
            pub alternative_names: Option<AlternativeNames> = "alternativeNames",
            /// The site it belongs to.
            pub site_ref: Option<Ref> = "SiteRef",
            /// The level it is on.
            pub level_ref: Option<Ref> = "LevelRef",
            /// Public code.
            pub public_code: Option<Box<str>> = "PublicCode",
            /// The label on it.
            pub label: Option<MultilingualString> = "Label",
            /// What kind of entrance.
            pub entrance_type: Option<EntranceType> = "EntranceType",
            /// Whether it leads outside.
            pub is_external: Option<bool> = "IsExternal",
            /// Whether one may enter.
            pub is_entry: Option<bool> = "IsEntry",
            /// Whether one may exit.
            pub is_exit: Option<bool> = "IsExit",
            /// Width in metres.
            pub width: Option<f64> = "Width",
            /// Height in metres.
            pub height: Option<f64> = "Height",
            /// Whether the kerb outside is dropped.
            pub dropped_kerb_outside: Option<bool> = "DroppedKerbOutside",
            /// The mode it serves.
            pub transport_mode: Option<VehicleMode> = "TransportMode",
            /// The submode it serves, stated beside the mode.
            pub submode: Option<Submode> = choice,
        }
    }
}

netex_container! {
    /// The entrances of a stop place.
    pub struct Entrances of EntranceItem {
        /// A stop place entrance.
        StopPlaceEntrance(StopPlaceEntrance) = "StopPlaceEntrance",
    }
}

netex_element! {
    /// A quay: where a vehicle stops and passengers board.
    pub struct Quay {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
        }
        children {
            /// When the entity is valid.
            pub valid_between: Vec<ValidBetween> = "ValidBetween",
            /// Key-value pairs; the German delivery's DHID is under `GlobalID`.
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
            /// Where it is.
            pub centroid: Option<Centroid> = "Centroid",
            /// The zone it is part of.
            pub parent_zone_ref: Option<Ref> = "ParentZoneRef",
            /// How accessible it is.
            pub accessibility_assessment: Option<AccessibilityAssessment> = "AccessibilityAssessment",
            /// Other names.
            pub alternative_names: Option<AlternativeNames> = "alternativeNames",
            /// Whether it is covered.
            pub covered: Option<Box<str>> = "Covered",
            /// The site it belongs to.
            pub site_ref: Option<Ref> = "SiteRef",
            /// The level it is on.
            pub level_ref: Option<Ref> = "LevelRef",
            /// The mode it serves.
            pub transport_mode: Option<VehicleMode> = "TransportMode",
            /// The submode it serves, stated beside the mode.
            pub submode: Option<Submode> = choice,
            /// Other modes it serves, space-separated.
            pub other_transport_modes: Option<Box<str>> = "OtherTransportModes",
            /// Whether one may board here.
            pub boarding_use: Option<bool> = "BoardingUse",
            /// Whether one may alight here.
            pub alighting_use: Option<bool> = "AlightingUse",
            /// The label on it.
            pub label: Option<MultilingualString> = "Label",
            /// Its entrances.
            pub entrances: Option<Entrances> = "entrances",
            /// Length in metres.
            pub length: Option<f64> = "Length",
            /// Width in metres.
            pub width: Option<f64> = "Width",
            /// Public code: the platform number.
            pub public_code: Option<Box<str>> = "PublicCode",
            /// Code on the plate.
            pub plate_code: Option<Box<str>> = "PlateCode",
            /// Short code.
            pub short_code: Option<Box<str>> = "ShortCode",
            /// Bearing of vehicles leaving, in degrees.
            pub compass_bearing: Option<f64> = "CompassBearing",
            /// Bearing of vehicles leaving, as an octant: `N`, `SW`, …
            pub compass_octant: Option<Box<str>> = "CompassOctant",
            /// What kind of quay.
            pub quay_type: Option<QuayType> = "QuayType",
            /// Height of the platform above the rail, in metres.
            pub platform_height: Option<f64> = "PlatformHeight",
            /// The quay this one is a sector of.
            pub parent_quay_ref: Option<Ref> = "ParentQuayRef",
        }
    }
}

netex_container! {
    /// The quays of a stop place: defined in place, or — as the French
    /// profile writes them — defined beside it and named here.
    pub struct Quays of QuayItem {
        /// A quay.
        Quay(Quay) = "Quay",
        /// A quay defined elsewhere.
        QuayRef(Ref) = "QuayRef",
    }
}

netex_element! {
    /// A space passengers move through: a concourse, a passage, a hall.
    pub struct AccessSpace {
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
            /// Where it is.
            pub centroid: Option<Centroid> = "Centroid",
            /// How accessible it is.
            pub accessibility_assessment: Option<AccessibilityAssessment> = "AccessibilityAssessment",
            /// Other names.
            pub alternative_names: Option<AlternativeNames> = "alternativeNames",
            /// The site it belongs to.
            pub site_ref: Option<Ref> = "SiteRef",
            /// The level it is on.
            pub level_ref: Option<Ref> = "LevelRef",
            /// Whether one may board here.
            pub boarding_use: Option<bool> = "BoardingUse",
            /// Whether one may alight here.
            pub alighting_use: Option<bool> = "AlightingUse",
            /// The label on it.
            pub label: Option<MultilingualString> = "Label",
            /// Its entrances.
            pub entrances: Option<Entrances> = "entrances",
            /// Length in metres.
            pub length: Option<f64> = "Length",
            /// Width in metres.
            pub width: Option<f64> = "Width",
            /// What kind of space.
            pub access_space_type: Option<AccessSpaceType> = "AccessSpaceType",
            /// What kind of passage: `pathway`, `corridor`, `overpass`, …
            pub passage_type: Option<Box<str>> = "PassageType",
            /// The space this one is part of.
            pub parent_access_space_ref: Option<Ref> = "ParentAccessSpaceRef",
        }
    }
}

netex_container! {
    /// The access spaces of a stop place.
    pub struct AccessSpaces of AccessSpaceItem {
        /// An access space.
        AccessSpace(AccessSpace) = "AccessSpace",
    }
}

netex_choice! {
    /// The organisation in charge of a site.
    pub enum OperatingOrganisationRef {
        /// An authority.
        Authority(Ref) = "AuthorityRef",
        /// An operator.
        Operator(Ref) = "OperatorRef",
        /// Any organisation.
        Organisation(Ref) = "OrganisationRef",
    }
}

netex_element! {
    /// A stop place: a station, a bus stop with all its poles, a harbour.
    pub struct StopPlace {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
        }
        children {
            /// When the entity is valid.
            pub valid_between: Vec<ValidBetween> = "ValidBetween",
            /// Key-value pairs; the German delivery's DHID is under `GlobalID`.
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
            /// Where it is.
            pub centroid: Option<Centroid> = "Centroid",
            /// The zone it is part of.
            pub parent_zone_ref: Option<Ref> = "ParentZoneRef",
            /// How accessible it is.
            pub accessibility_assessment: Option<AccessibilityAssessment> = "AccessibilityAssessment",
            /// Other names.
            pub alternative_names: Option<AlternativeNames> = "alternativeNames",
            /// Whether it is covered.
            pub covered: Option<Box<str>> = "Covered",
            /// Whether every area can be reached by wheelchair.
            pub all_areas_wheelchair_accessible: Option<bool> = "AllAreasWheelchairAccessible",
            /// The town or district it is in.
            pub topographic_place_ref: Option<Ref> = "TopographicPlaceRef",
            /// Time zone and language.
            pub locale: Option<Locale> = "Locale",
            /// Who is in charge of it.
            pub operating_organisation_ref: Option<OperatingOrganisationRef> = choice,
            /// The site it is part of.
            pub parent_site_ref: Option<Ref> = "ParentSiteRef",
            /// Its levels.
            pub levels: Option<Levels> = "levels",
            /// Its entrances.
            pub entrances: Option<Entrances> = "entrances",
            /// Public code.
            pub public_code: Option<Box<str>> = "PublicCode",
            /// The main mode.
            pub transport_mode: Option<VehicleMode> = "TransportMode",
            /// The submode of the main mode, stated beside it: `regionalRail`,
            /// `localBus`, …
            pub submode: Option<Submode> = choice,
            /// Other modes, space-separated.
            pub other_transport_modes: Option<Box<str>> = "OtherTransportModes",
            /// What kind of stop place.
            pub stop_place_type: Option<StopType> = "StopPlaceType",
            /// Whether it is at a border.
            pub border_crossing: Option<bool> = "BorderCrossing",
            /// Whether its use is limited: `interchangeOnly`, `noThroughAccess`, …
            pub limited_use: Option<Box<str>> = "LimitedUse",
            /// Its quays.
            pub quays: Option<Quays> = "quays",
            /// Its access spaces.
            pub access_spaces: Option<AccessSpaces> = "accessSpaces",
        }
    }
}

netex_container! {
    /// The stop places of a site frame.
    pub struct StopPlaces of StopPlaceItem {
        /// A stop place.
        StopPlace(StopPlace) = "StopPlace",
    }
}

netex_container! {
    /// The members of a group of stop places.
    pub struct StopPlaceMembers of StopPlaceMember {
        /// A member.
        StopPlaceRef(Ref) = "StopPlaceRef",
    }
}

netex_element! {
    /// Stop places grouped under one name: the stops of an interchange.
    pub struct GroupOfStopPlaces {
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
            /// The members.
            pub members: Option<StopPlaceMembers> = "members",
            /// Other names.
            pub alternative_names: Option<AlternativeNames> = "alternativeNames",
            /// The centre of the group.
            pub centroid: Option<Centroid> = "Centroid",
            /// The main mode.
            pub transport_mode: Option<VehicleMode> = "TransportMode",
        }
    }
}

netex_container! {
    /// The groups of stop places of a site frame.
    pub struct GroupsOfStopPlaces of GroupOfStopPlacesItem {
        /// A group.
        GroupOfStopPlaces(GroupOfStopPlaces) = "GroupOfStopPlaces",
    }
}

netex_element! {
    /// How long a transfer takes, for different passengers.
    pub struct TransferDuration {
        attributes {}
        children {
            /// For most passengers.
            pub default_duration: Option<Duration> = "DefaultDuration",
            /// For passengers who know the way.
            pub frequent_traveller_duration: Option<Duration> = "FrequentTravellerDuration",
            /// For passengers who do not.
            pub occasional_traveller_duration: Option<Duration> = "OccasionalTravellerDuration",
            /// For passengers with restricted mobility.
            pub mobility_restricted_traveller_duration: Option<Duration> = "MobilityRestrictedTravellerDuration",
        }
    }
}

netex_element! {
    /// One end of a site connection: the schema's `SiteConnectionEndStructure`.
    pub struct SiteConnectionEnd {
        attributes {}
        children {
            /// The mode at this end.
            pub transport_mode: Option<VehicleMode> = "TransportMode",
            /// The stop area at this end.
            pub stop_area_ref: Option<Ref> = "StopAreaRef",
            /// The scheduled stop point at this end.
            pub scheduled_stop_point_ref: Option<Ref> = "ScheduledStopPointRef",
            /// The stop place at this end.
            pub stop_place_ref: Option<Ref> = "StopPlaceRef",
            /// The quay at this end.
            pub quay_ref: Option<Ref> = "QuayRef",
            /// The access space at this end.
            pub access_space_ref: Option<Ref> = "AccessSpaceRef",
            /// The boarding position at this end.
            pub boarding_position_ref: Option<Ref> = "BoardingPositionRef",
            /// The entrance at this end.
            pub stop_place_entrance_ref: Option<Ref> = "StopPlaceEntranceRef",
            /// The label at this end.
            pub label: Option<MultilingualString> = "Label",
        }
    }
}

netex_element! {
    /// A measured walk between two sites or parts of sites.
    pub struct SiteConnection {
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
            /// What kind of transfer.
            pub type_of_transfer_ref: Option<Ref> = "TypeOfTransferRef",
            /// What it is.
            pub description: Option<MultilingualString> = "Description",
            /// Length in metres.
            pub distance: Option<f64> = "Distance",
            /// How long it takes.
            pub transfer_duration: Option<TransferDuration> = "TransferDuration",
            /// How long it takes on foot.
            pub walk_transfer_duration: Option<TransferDuration> = "WalkTransferDuration",
            /// Whether it can be walked in both directions.
            pub both_ways: Option<bool> = "BothWays",
            /// How it is made.
            pub transfer_mode: Option<AccessMode> = "TransferMode",
            /// Where it starts.
            pub from: Option<SiteConnectionEnd> = "From",
            /// Where it ends.
            pub to: Option<SiteConnectionEnd> = "To",
        }
    }
}

netex_container! {
    /// The connections of a service frame.
    pub struct Connections of ConnectionItem {
        /// A site connection.
        SiteConnection(SiteConnection) = "SiteConnection",
    }
}
