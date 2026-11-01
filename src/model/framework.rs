//! The framework: validity, codespaces, frame defaults.

use super::{
    DateTime, DayTypeRefs, Id, KeyList, Locale, MultilingualString, PrivateCodes, Ref, Version,
};

netex_element! {
    /// The period an entity or frame is valid for.
    ///
    /// The schema: "OPTIMISATION. Simple version of a VALIDITY CONDITION.
    /// Comprises a simple period." `ToDate` is inclusive.
    pub struct ValidBetween {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
        }
        children {
            /// Start of the period.
            pub from_date: Option<DateTime> = "FromDate",
            /// End of the period, inclusive.
            pub to_date: Option<DateTime> = "ToDate",
        }
    }
}

netex_element! {
    /// The days an entity is available on, or not: a period, its days as a
    /// bit per day where it states them.
    ///
    /// The Swiss delivery dates every journey so, the Austrian ones the
    /// journeys of their flexible lines.
    pub struct AvailabilityCondition {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
        }
        children {
            /// Key-value pairs.
            pub key_list: Option<KeyList> = "keyList",
            /// Codes it is known by in other systems, each with its type.
            pub private_codes: Option<PrivateCodes> = "privateCodes",
            /// The entity's branding.
            pub branding_ref: Option<Ref> = "BrandingRef",
            /// Name.
            pub name: Option<MultilingualString> = "Name",
            /// What it is.
            pub description: Option<MultilingualString> = "Description",
            /// Start of the period.
            pub from_date: Option<DateTime> = "FromDate",
            /// End of the period, inclusive.
            pub to_date: Option<DateTime> = "ToDate",
            /// Whether the days are available (`true`, the default) or not.
            pub is_available: Option<bool> = "IsAvailable",
            /// The day types the condition holds on.
            pub day_types: Option<DayTypeRefs> = "dayTypes",
            /// One bit per day of the period, `1` where the condition holds.
            pub valid_day_bits: Option<Box<str>> = "ValidDayBits",
        }
    }
}

netex_container! {
    /// The validity conditions of an entity or frame: those it names, those
    /// it states in place, and simple periods.
    pub struct ValidityConditions of ValidityConditionItem {
        /// An availability condition stated in place.
        AvailabilityCondition(AvailabilityCondition) = "AvailabilityCondition",
        /// An availability condition named.
        AvailabilityConditionRef(Ref) = "AvailabilityConditionRef",
        /// A simple period.
        ValidBetween(ValidBetween) = "ValidBetween",
    }
}

netex_element! {
    /// A namespace for identifiers.
    pub struct Codespace {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
        }
        children {
            /// The prefix.
            pub xmlns: Option<Box<str>> = "Xmlns",
            /// The URL the prefix stands for.
            pub xmlns_url: Option<Box<str>> = "XmlnsUrl",
            /// What the codespace is for.
            pub description: Option<MultilingualString> = "Description",
        }
    }
}

netex_element! {
    /// The codespaces a frame's identifiers use.
    pub struct Codespaces {
        attributes {}
        children {
            /// The codespaces.
            pub codespaces: Vec<Codespace> = "Codespace",
        }
    }
}

netex_element! {
    /// What a frame's entities assume unless they say otherwise.
    pub struct FrameDefaults {
        attributes {}
        children {
            /// The codespace of identifiers without a prefix.
            pub default_codespace_ref: Option<Ref> = "DefaultCodespaceRef",
            /// The data source of entities that name none.
            pub default_data_source_ref: Option<Ref> = "DefaultDataSourceRef",
            /// The responsibility set of entities that name none.
            pub default_responsibility_set_ref: Option<Ref> = "DefaultResponsibilitySetRef",
            /// Time zone and language of the frame.
            pub default_locale: Option<Locale> = "DefaultLocale",
            /// The coordinate reference system of locations that name none: `urn:ogc:def:crs:EPSG::4326`.
            pub default_location_system: Option<Box<str>> = "DefaultLocationSystem",
            /// The system of units.
            pub default_system_of_units: Option<Box<str>> = "DefaultSystemOfUnits",
            /// The currency.
            pub default_currency: Option<Box<str>> = "DefaultCurrency",
        }
    }
}
