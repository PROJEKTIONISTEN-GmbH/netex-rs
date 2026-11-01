//! The service calendar frame: day types, operating periods and days, and
//! which day type each day is.

use super::{
    Codespaces, Date, DateTime, Duration, FrameDefaults, Id, KeyList, MultilingualString, Order,
    PrivateCode, PrivateCodes, Ref, Time, ValidBetween, ValidityConditions, Version,
};

netex_element! {
    /// The frame holding the calendar.
    pub struct ServiceCalendarFrame {
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
            /// The calendar, when the frame wraps one.
            pub service_calendar: Option<ServiceCalendar> = "ServiceCalendar",
            /// Day types.
            pub day_types: Option<DayTypes> = "dayTypes",
            /// Operating days.
            pub operating_days: Option<OperatingDays> = "operatingDays",
            /// Operating periods.
            pub operating_periods: Option<OperatingPeriods> = "operatingPeriods",
            /// Which day type each day is.
            pub day_type_assignments: Option<DayTypeAssignments> = "dayTypeAssignments",
        }
    }
}

netex_element! {
    /// A calendar: the days a timetable covers, and their types.
    pub struct ServiceCalendar {
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
            /// First day.
            pub from_date: Option<Date> = "FromDate",
            /// Last day, inclusive.
            pub to_date: Option<Date> = "ToDate",
            /// When an operating day starts.
            pub earliest_time: Option<Time> = "EarliestTime",
            /// How long an operating day is.
            pub day_length: Option<Duration> = "DayLength",
            /// Day types.
            pub day_types: Option<DayTypes> = "dayTypes",
            /// Operating days.
            pub operating_days: Option<OperatingDays> = "operatingDays",
            /// Operating periods.
            pub operating_periods: Option<OperatingPeriods> = "operatingPeriods",
            /// Which day type each day is.
            pub day_type_assignments: Option<DayTypeAssignments> = "dayTypeAssignments",
        }
    }
}

netex_element! {
    /// What makes a day a day of this type.
    pub struct PropertyOfDay {
        attributes {}
        children {
            /// Name.
            pub name: Option<MultilingualString> = "Name",
            /// What it is.
            pub description: Option<MultilingualString> = "Description",
            /// The days of the week, space-separated: `Monday Tuesday`, `Weekdays`, `Everyday`.
            pub days_of_week: Option<Box<str>> = "DaysOfWeek",
            /// The weeks of the month, space-separated.
            pub weeks_of_month: Option<Box<str>> = "WeeksOfMonth",
            /// The month.
            pub month_of_year: Option<Box<str>> = "MonthOfYear",
            /// The day of the month.
            pub day_of_month: Option<Box<str>> = "DayOfMonth",
            /// The day of the year.
            pub day_of_year: Option<Box<str>> = "DayOfYear",
            /// The country whose holidays count.
            pub country_ref: Option<Ref> = "CountryRef",
            /// The kinds of holiday, space-separated.
            pub holiday_types: Option<Box<str>> = "HolidayTypes",
            /// The seasons, space-separated.
            pub seasons: Option<Box<str>> = "Seasons",
        }
    }
}

netex_element! {
    /// The properties of a day type.
    pub struct DayProperties {
        attributes {}
        children {
            /// The properties.
            pub properties: Vec<PropertyOfDay> = "PropertyOfDay",
        }
    }
}

netex_element! {
    /// A type of day: weekdays, Saturdays, school holidays.
    pub struct DayType {
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
            /// The publisher's own code.
            pub private_code: Option<PrivateCode> = "PrivateCode",
            /// When such a day starts.
            pub earliest_time: Option<Time> = "EarliestTime",
            /// How long such a day is.
            pub day_length: Option<Duration> = "DayLength",
            /// What makes a day one of these.
            pub properties: Option<DayProperties> = "properties",
        }
    }
}

netex_container! {
    /// The day types of a calendar.
    pub struct DayTypes of DayTypeItem {
        /// A day type.
        DayType(DayType) = "DayType",
    }
}

netex_choice! {
    /// The first day of an operating period.
    pub enum PeriodStart {
        /// An operating day.
        FromOperatingDayRef(Ref) = "FromOperatingDayRef",
        /// A date.
        FromDate(DateTime) = "FromDate",
    }
}

netex_choice! {
    /// The last day of an operating period.
    pub enum PeriodEnd {
        /// An operating day.
        ToOperatingDayRef(Ref) = "ToOperatingDayRef",
        /// A date.
        ToDate(DateTime) = "ToDate",
    }
}

netex_element! {
    /// A run of days, and — for `<UicOperatingPeriod>` — which of them are
    /// operated, as a bit per day.
    ///
    /// Read for `<OperatingPeriod>` and for `<UicOperatingPeriod>`, which adds
    /// the last two children.
    pub struct OperatingPeriod {
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
            /// The calendar.
            pub service_calendar_ref: Option<Ref> = "ServiceCalendarRef",
            /// Name.
            pub name: Option<MultilingualString> = "Name",
            /// Short name.
            pub short_name: Option<MultilingualString> = "ShortName",
            /// First day.
            pub start: Option<PeriodStart> = choice,
            /// Last day, inclusive.
            pub end: Option<PeriodEnd> = choice,
            /// The kinds of holiday in the period.
            pub holiday_type: Vec<Box<str>> = "HolidayType",
            /// The seasons in the period.
            pub season: Vec<Box<str>> = "Season",
            /// One bit per day from the first day: `1` operated, `0` not.
            pub valid_day_bits: Option<Box<str>> = "ValidDayBits",
            /// The days of the week the bits cover, space-separated.
            pub days_of_week: Option<Box<str>> = "DaysOfWeek",
        }
    }
}

netex_container! {
    /// The operating periods of a calendar.
    pub struct OperatingPeriods of OperatingPeriodItem {
        /// An operating period.
        OperatingPeriod(OperatingPeriod) = "OperatingPeriod",
        /// An operating period with a bit per day.
        UicOperatingPeriod(OperatingPeriod) = "UicOperatingPeriod",
    }
}

netex_element! {
    /// One day of operation.
    pub struct OperatingDay {
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
            /// The date.
            pub calendar_date: Option<Date> = "CalendarDate",
            /// The calendar.
            pub service_calendar_ref: Option<Ref> = "ServiceCalendarRef",
            /// Name.
            pub name: Option<MultilingualString> = "Name",
            /// Short name.
            pub short_name: Option<MultilingualString> = "ShortName",
            /// The day's number.
            pub day_number: Option<Box<str>> = "DayNumber",
            /// The publisher's own code.
            pub private_code: Option<PrivateCode> = "PrivateCode",
            /// When the day starts.
            pub earliest_time: Option<Time> = "EarliestTime",
            /// How long it is.
            pub day_length: Option<Duration> = "DayLength",
        }
    }
}

netex_container! {
    /// The operating days of a calendar.
    pub struct OperatingDays of OperatingDayItem {
        /// An operating day.
        OperatingDay(OperatingDay) = "OperatingDay",
    }
}

netex_choice! {
    /// The days a day type assignment covers.
    pub enum AssignedDays {
        /// An operating period.
        OperatingPeriodRef(Ref) = "OperatingPeriodRef",
        /// An operating period with a bit per day.
        UicOperatingPeriodRef(Ref) = "UicOperatingPeriodRef",
        /// One operating day.
        OperatingDayRef(Ref) = "OperatingDayRef",
        /// One date.
        Date(Date) = "Date",
    }
}

netex_choice! {
    /// The day type a day type assignment assigns.
    pub enum AssignedDayType {
        /// A day type.
        DayTypeRef(Ref) = "DayTypeRef",
        /// A fare day type.
        FareDayTypeRef(Ref) = "FareDayTypeRef",
    }
}

netex_element! {
    /// Which day type some days are, or are not.
    pub struct DayTypeAssignment {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
            /// The `order` attribute: later assignments override earlier ones.
            pub order: Option<Order> = "order",
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
            /// What it is.
            pub description: Option<MultilingualString> = "Description",
            /// The calendar.
            pub service_calendar_ref: Option<Ref> = "ServiceCalendarRef",
            /// The days.
            pub days: Option<AssignedDays> = choice,
            /// The day type.
            pub day_type: Option<AssignedDayType> = choice,
            /// The timebands.
            pub timeband_ref: Vec<Ref> = "TimebandRef",
            /// Whether the days are of the type (`true`, the default) or not.
            pub is_available: Option<bool> = "isAvailable",
        }
    }
}

netex_container! {
    /// The day type assignments of a calendar.
    pub struct DayTypeAssignments of DayTypeAssignmentItem {
        /// A day type assignment.
        DayTypeAssignment(DayTypeAssignment) = "DayTypeAssignment",
    }
}
