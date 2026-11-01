//! The resource frame: organisations, data sources, vehicle types.

use super::{
    AlternativeNames, Codespaces, ContactDetails, FareClass, FrameDefaults, Id, KeyList, Locale,
    MultilingualString, PrivateCode, PrivateCodes, Ref, ValidBetween, ValidityConditions,
    VehicleMode, Version,
};

netex_element! {
    /// The frame holding what the other frames share: who publishes and
    /// operates, and what the vehicles are.
    pub struct ResourceFrame {
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
            /// The profile the frame follows.
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
            /// Where the data comes from.
            pub data_sources: Option<DataSources> = "dataSources",
            /// Who is responsible for what.
            pub responsibility_sets: Option<ResponsibilitySets> = "responsibilitySets",
            /// Operators and authorities.
            pub organisations: Option<Organisations> = "organisations",
            /// Kinds of vehicle.
            pub vehicle_types: Option<VehicleTypes> = "vehicleTypes",
        }
    }
}

netex_element! {
    /// A source of data.
    pub struct DataSource {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
        }
        children {
            /// Name of the source.
            pub name: Option<MultilingualString> = "Name",
            /// Its short name.
            pub short_name: Option<MultilingualString> = "ShortName",
            /// What it is.
            pub description: Option<MultilingualString> = "Description",
            /// Its web address.
            pub url: Option<Box<str>> = "Url",
            /// Its own code for itself.
            pub private_code: Option<PrivateCode> = "PrivateCode",
            /// Its email address.
            pub email: Option<Box<str>> = "Email",
        }
    }
}

netex_container! {
    /// The data sources of a resource frame.
    pub struct DataSources of DataSourceItem {
        /// A data source.
        DataSource(DataSource) = "DataSource",
    }
}

netex_element! {
    /// One organisation's role over some data.
    pub struct ResponsibilityRoleAssignment {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
        }
        children {
            /// The set the assignment belongs to.
            pub responsibility_set_ref: Option<Ref> = "ResponsibilitySetRef",
            /// What the assignment is.
            pub description: Option<MultilingualString> = "Description",
            /// The roles over the data, space-separated: `creates`, `distributes`, …
            pub data_role_type: Option<Box<str>> = "DataRoleType",
            /// The roles as a stakeholder, space-separated: `operation`, `planning`, …
            pub stakeholder_role_type: Option<Box<str>> = "StakeholderRoleType",
            /// The organisation responsible.
            pub responsible_organisation_ref: Option<Ref> = "ResponsibleOrganisationRef",
            /// The part of the organisation responsible.
            pub responsible_part_ref: Option<Ref> = "ResponsiblePartRef",
            /// The area the responsibility covers.
            pub responsible_area_ref: Option<Ref> = "ResponsibleAreaRef",
        }
    }
}

netex_element! {
    /// The role assignments of a responsibility set.
    pub struct Roles {
        attributes {}
        children {
            /// The assignments.
            pub roles: Vec<ResponsibilityRoleAssignment> = "ResponsibilityRoleAssignment",
        }
    }
}

netex_element! {
    /// A set of responsibilities: who does what with the data.
    pub struct ResponsibilitySet {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
        }
        children {
            /// Name of the set.
            pub name: Option<MultilingualString> = "Name",
            /// Its own code.
            pub private_code: Option<PrivateCode> = "PrivateCode",
            /// The roles.
            pub roles: Option<Roles> = "roles",
        }
    }
}

netex_container! {
    /// The responsibility sets of a resource frame.
    pub struct ResponsibilitySets of ResponsibilitySetItem {
        /// A responsibility set.
        ResponsibilitySet(ResponsibilitySet) = "ResponsibilitySet",
    }
}

netex_element! {
    /// An operator or authority: the schema's `Organisation_VersionStructure`,
    /// which the two share.
    pub struct Organisation {
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
            /// Codes it is known by in other systems, each with its type.
            pub private_codes: Option<PrivateCodes> = "privateCodes",
            /// The entity's branding.
            pub branding_ref: Option<Ref> = "BrandingRef",
            /// Public code.
            pub public_code: Option<Box<str>> = "PublicCode",
            /// The organisation's own code.
            pub private_code: Option<PrivateCode> = "PrivateCode",
            /// Registered company number.
            pub company_number: Option<Box<str>> = "CompanyNumber",
            /// Name.
            pub name: Option<MultilingualString> = "Name",
            /// Short name.
            pub short_name: Option<MultilingualString> = "ShortName",
            /// Legal name.
            pub legal_name: Option<MultilingualString> = "LegalName",
            /// Trading name.
            pub trading_name: Option<MultilingualString> = "TradingName",
            /// Other names.
            pub alternative_names: Option<AlternativeNames> = "alternativeNames",
            /// What the organisation is.
            pub description: Option<MultilingualString> = "Description",
            /// Remarks.
            pub remarks: Option<MultilingualString> = "Remarks",
            /// Time zone and language.
            pub locale: Option<Locale> = "Locale",
            /// How to reach it.
            pub contact_details: Option<ContactDetails> = "ContactDetails",
            /// How to reach it privately.
            pub private_contact_details: Option<ContactDetails> = "PrivateContactDetails",
            /// The kinds of organisation, space-separated: `authority`, `operator`, …
            pub organisation_type: Option<Box<str>> = "OrganisationType",
            /// The country.
            pub country_ref: Option<Ref> = "CountryRef",
            /// The main mode operated.
            pub primary_mode: Option<VehicleMode> = "PrimaryMode",
        }
    }
}

netex_container! {
    /// The organisations of a resource frame.
    pub struct Organisations of OrganisationItem {
        /// An operator of services.
        Operator(Organisation) = "Operator",
        /// An authority ordering services.
        Authority(Organisation) = "Authority",
    }
}

netex_element! {
    /// How many passengers of one class a vehicle carries.
    pub struct PassengerCapacity {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
        }
        children {
            /// The class.
            pub fare_class: Option<FareClass> = "FareClass",
            /// All places.
            pub total_capacity: Option<u32> = "TotalCapacity",
            /// Seats.
            pub seating_capacity: Option<u32> = "SeatingCapacity",
            /// Standing places.
            pub standing_capacity: Option<u32> = "StandingCapacity",
            /// Special places.
            pub special_place_capacity: Option<u32> = "SpecialPlaceCapacity",
            /// Pushchair places.
            pub pushchair_capacity: Option<u32> = "PushchairCapacity",
            /// Wheelchair places.
            pub wheelchair_place_capacity: Option<u32> = "WheelchairPlaceCapacity",
            /// Pram places.
            pub pram_place_capacity: Option<u32> = "PramPlaceCapacity",
            /// Bicycle rack places.
            pub bicycle_rack_capacity: Option<u32> = "BicycleRackCapacity",
        }
    }
}

netex_element! {
    /// The capacities of a vehicle type, one per class.
    pub struct Capacities {
        attributes {}
        children {
            /// The capacities.
            pub capacities: Vec<PassengerCapacity> = "PassengerCapacity",
        }
    }
}

netex_element! {
    /// A kind of vehicle.
    pub struct VehicleType {
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
            /// Codes it is known by in other systems, each with its type.
            pub private_codes: Option<PrivateCodes> = "privateCodes",
            /// The entity's branding.
            pub branding_ref: Option<Ref> = "BrandingRef",
            /// Name.
            pub name: Option<MultilingualString> = "Name",
            /// Short name.
            pub short_name: Option<MultilingualString> = "ShortName",
            /// What it is.
            pub description: Option<MultilingualString> = "Description",
            /// The operator's own code.
            pub private_code: Option<PrivateCode> = "PrivateCode",
            /// The mode.
            pub transport_mode: Option<VehicleMode> = "TransportMode",
            /// How many it carries.
            pub capacities: Option<Capacities> = "capacities",
            /// Whether the floor is low.
            pub low_floor: Option<bool> = "LowFloor",
            /// Whether it has a lift or ramp.
            pub has_lift_or_ramp: Option<bool> = "HasLiftOrRamp",
            /// Whether it has a hoist.
            pub has_hoist: Option<bool> = "HasHoist",
            /// Length in metres.
            pub length: Option<f64> = "Length",
            /// Width in metres.
            pub width: Option<f64> = "Width",
            /// Height in metres.
            pub height: Option<f64> = "Height",
            /// Weight in kilograms.
            pub weight: Option<f64> = "Weight",
        }
    }
}

netex_container! {
    /// The vehicle types of a frame.
    pub struct VehicleTypes of VehicleTypeItem {
        /// A vehicle type.
        VehicleType(VehicleType) = "VehicleType",
    }
}
