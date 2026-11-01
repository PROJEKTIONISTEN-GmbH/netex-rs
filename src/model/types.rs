//! The value types the model is built from: identifiers, references, texts,
//! and the lexical types of XML Schema.

use std::borrow::Cow;
use std::fmt;

use super::{AttributeValue, Element, Leaf, Merge, Walk};
use crate::error::Result;
use crate::xml::{Attribute, Child, Content, Cursor, Emitter, Unprocessed};

/// The identifier of an entity: the `id` attribute, or the `ref` of a reference.
///
/// NeTEx identifiers are structured (`DE::StopPlace:32901_vgm_d::`) but the
/// crate treats them as opaque; what structure a dataset gives them is the
/// dataset's business.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id(pub Box<str>);

impl Id {
    /// The identifier as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for Id {
    fn from(text: &str) -> Self {
        Id(text.into())
    }
}

impl AttributeValue for Id {
    const DATATYPE: &'static str = "ObjectIdType";

    fn from_text(text: &str) -> Option<Self> {
        Some(Id(text.into()))
    }

    fn to_text(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.0)
    }
}

/// The version of an entity: the `version` attribute.
///
/// The schema says of it: "Version number of entity. The value may be "any" or
/// a VERSION number adhering to agreed-upon formatting and interpretation rules
/// (representing, e.g., a number or a date). Consecutive VERSION numbers should
/// be strictly increasing." (`netex_version_support.xsd`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Version(pub Box<str>);

impl Version {
    /// The token that refers to whatever version is current.
    pub const ANY: &'static str = "any";

    /// The version as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Whether this is the `any` version.
    pub fn is_any(&self) -> bool {
        &*self.0 == Self::ANY
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for Version {
    fn from(text: &str) -> Self {
        Version(text.into())
    }
}

/// The `order` attribute: the position of a child among its siblings.
///
/// Kept as written (`0001` is written back as `0001`); [`Order::value`] reads
/// the number out.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Order(pub Box<str>);

impl Order {
    /// The position as a number, if the attribute is one.
    pub fn value(&self) -> Option<u32> {
        self.0.trim().parse().ok()
    }
}

impl From<u32> for Order {
    fn from(value: u32) -> Self {
        Order(value.to_string().into())
    }
}

impl fmt::Display for Order {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AttributeValue for Order {
    const DATATYPE: &'static str = "xsd:positiveInteger";

    fn from_text(text: &str) -> Option<Self> {
        Some(Order(text.into()))
    }

    fn to_text(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.0)
    }
}

impl AttributeValue for Version {
    const DATATYPE: &'static str = "VersionIdType";

    fn from_text(text: &str) -> Option<Self> {
        Some(Version(text.into()))
    }

    fn to_text(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.0)
    }
}

/// A reference to an entity: `<LineRef ref="…" version="…"/>`.
///
/// The schema's `VersionOfObjectRefStructure`: `ref` names the entity,
/// `version` "Version number of referenced entity", and `versionRef` the
/// version "for use when reference is External and a specific version is
/// required. Mutually exclusive with version." A reference without a version
/// is to the current version of the entity.
///
/// The schema requires `ref` on nearly every reference; a `<CountryRef>` may
/// instead name the country as its content, so `ref` is optional here and the
/// content is kept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ref {
    /// The referenced entity.
    pub r#ref: Option<Id>,
    /// The referenced version, if a specific one.
    pub version: Option<Version>,
    /// The referenced version of an external entity, if a specific one.
    pub version_ref: Option<Box<str>>,
    /// Attributes beyond those three, kept as written.
    pub extra_attributes: Vec<Attribute>,
    /// Character data inside the reference: the referenced entity's name,
    /// which some producers write there.
    pub text: Option<Box<str>>,
}

impl Ref {
    /// A reference to `id`, with no version.
    pub fn to(id: impl Into<Id>) -> Self {
        Ref {
            r#ref: Some(id.into()),
            version: None,
            version_ref: None,
            extra_attributes: Vec::new(),
            text: None,
        }
    }

    /// A reference to `id` at `version`.
    pub fn to_version(id: impl Into<Id>, version: impl Into<Version>) -> Self {
        Ref {
            version: Some(version.into()),
            ..Self::to(id)
        }
    }
}

impl Element for Ref {
    fn read<'i>(cursor: &mut Cursor<'i>, child: &Child<'i>) -> Result<Self> {
        let mut r#ref = None;
        let mut version = None;
        let mut version_ref = None;
        let mut extra_attributes = Vec::new();
        for attribute in child.attributes() {
            let (key, value) = attribute.map_err(|e| cursor.error_at(e.kind, child.offset))?;
            match key {
                "ref" => r#ref = Some(Id(value.into_owned().into())),
                "version" => version = Some(Version(value.into_owned().into())),
                "versionRef" => version_ref = Some(value.into_owned().into()),
                other => extra_attributes.push(Attribute {
                    name: other.into(),
                    value: value.into_owned().into(),
                }),
            }
        }
        cursor.enter(child)?;
        let text = cursor.text(child)?;
        cursor.leave();
        let text = (!text.is_empty()).then(|| text.into_owned().into());
        Ok(Ref {
            r#ref,
            version,
            version_ref,
            extra_attributes,
            text,
        })
    }

    fn write(&self, name: &str, out: &mut Emitter) {
        let mut attributes: Vec<(&str, &str)> = Vec::with_capacity(3);
        if let Some(r#ref) = &self.r#ref {
            attributes.push(("ref", r#ref.as_str()));
        }
        if let Some(version) = &self.version {
            attributes.push(("version", version.as_str()));
        }
        if let Some(version_ref) = &self.version_ref {
            attributes.push(("versionRef", version_ref));
        }
        out.leaf(
            name,
            &attributes,
            &self.extra_attributes,
            self.text.as_deref().unwrap_or(""),
        );
    }
}

/// A text in one language, or its translations: the schema's `MultilingualString`.
///
/// In 1.x the element holds the text and, optionally, its language. In 2.0 it
/// may instead hold a list of `<Text lang="…">` children, one per language;
/// those are [`translations`](Self::translations), and the element's own text
/// is then empty.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MultilingualString {
    /// The text.
    pub text: Box<str>,
    /// The language, as the `lang` attribute (an RFC 5646 tag: `de-DE`).
    pub lang: Option<Box<str>>,
    /// The `textIdType` attribute: an identifier for the text in a translation resource.
    pub text_id_type: Option<Box<str>>,
    /// Attributes beyond those two, kept as written.
    pub extra_attributes: Vec<Attribute>,
    /// The text in each language, when the element holds translations
    /// instead of a text.
    pub translations: Vec<MultilingualString>,
    /// Children other than translations, kept as written.
    pub unprocessed: Vec<Unprocessed>,
}

impl MultilingualString {
    /// A text with no language given.
    pub fn new(text: impl Into<Box<str>>) -> Self {
        MultilingualString {
            text: text.into(),
            ..Self::default()
        }
    }

    /// The text.
    pub fn as_str(&self) -> &str {
        &self.text
    }
}

impl From<&str> for MultilingualString {
    fn from(text: &str) -> Self {
        Self::new(text)
    }
}

impl Element for MultilingualString {
    fn read<'i>(cursor: &mut Cursor<'i>, child: &Child<'i>) -> Result<Self> {
        let mut this = Self::default();
        for attribute in child.attributes() {
            let (key, value) = attribute.map_err(|e| cursor.error_at(e.kind, child.offset))?;
            match key {
                "lang" => this.lang = Some(value.into_owned().into()),
                "textIdType" => this.text_id_type = Some(value.into_owned().into()),
                other => this.extra_attributes.push(Attribute {
                    name: other.into(),
                    value: value.into_owned().into(),
                }),
            }
        }
        cursor.enter(child)?;
        match cursor.content(child)? {
            Content::Text(text) => this.text = text.into_owned().into(),
            Content::Child(first) => {
                let mut next = Some(first);
                let mut position = 0u32;
                while let Some(item) = next {
                    if item.netex && item.local == "Text" {
                        this.translations.push(Self::read(cursor, &item)?);
                    } else {
                        this.unprocessed.push(cursor.skip(&item, position)?);
                    }
                    position += 1;
                    next = cursor.next_child()?;
                }
            }
        }
        cursor.leave();
        Ok(this)
    }

    fn write(&self, name: &str, out: &mut Emitter) {
        let mut attributes: Vec<(&str, &str)> = Vec::with_capacity(2);
        if let Some(lang) = &self.lang {
            attributes.push(("lang", lang));
        }
        if let Some(text_id_type) = &self.text_id_type {
            attributes.push(("textIdType", text_id_type));
        }
        if self.translations.is_empty() && self.unprocessed.is_empty() {
            out.leaf(name, &attributes, &self.extra_attributes, &self.text);
            return;
        }
        out.start(name, &attributes, &self.extra_attributes);
        let mut merge = Merge::new(&self.unprocessed);
        for translation in &self.translations {
            merge.before_child(out);
            translation.write("Text", out);
        }
        merge.finish(out);
        out.end(name);
    }
}

/// A private code: a text with a `type` attribute saying whose code it is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PrivateCode {
    /// The code.
    pub text: Box<str>,
    /// The `type` attribute.
    pub r#type: Option<Box<str>>,
    /// Attributes beyond `type`, kept as written.
    pub extra_attributes: Vec<Attribute>,
}

impl Element for PrivateCode {
    fn read<'i>(cursor: &mut Cursor<'i>, child: &Child<'i>) -> Result<Self> {
        let mut this = Self::default();
        for attribute in child.attributes() {
            let (key, value) = attribute.map_err(|e| cursor.error_at(e.kind, child.offset))?;
            match key {
                "type" => this.r#type = Some(value.into_owned().into()),
                other => this.extra_attributes.push(Attribute {
                    name: other.into(),
                    value: value.into_owned().into(),
                }),
            }
        }
        cursor.enter(child)?;
        this.text = cursor.text(child)?.into_owned().into();
        cursor.leave();
        Ok(this)
    }

    fn write(&self, name: &str, out: &mut Emitter) {
        let mut attributes: Vec<(&str, &str)> = Vec::with_capacity(1);
        if let Some(kind) = &self.r#type {
            attributes.push(("type", kind));
        }
        out.leaf(name, &attributes, &self.extra_attributes, &self.text);
    }
}

impl Leaf for Box<str> {
    const DATATYPE: &'static str = "xsd:string";

    fn from_text(text: &str) -> Option<Self> {
        Some(text.into())
    }

    fn to_text(&self) -> Cow<'_, str> {
        Cow::Borrowed(self)
    }
}

impl Leaf for bool {
    const DATATYPE: &'static str = "xsd:boolean";

    fn from_text(text: &str) -> Option<Self> {
        match text.trim() {
            "true" | "1" => Some(true),
            "false" | "0" => Some(false),
            _ => None,
        }
    }

    fn to_text(&self) -> Cow<'_, str> {
        Cow::Borrowed(if *self { "true" } else { "false" })
    }
}

impl Leaf for u32 {
    const DATATYPE: &'static str = "xsd:nonNegativeInteger";

    fn from_text(text: &str) -> Option<Self> {
        text.trim().parse().ok()
    }

    fn to_text(&self) -> Cow<'_, str> {
        Cow::Owned(self.to_string())
    }
}

impl Leaf for i64 {
    const DATATYPE: &'static str = "xsd:integer";

    fn from_text(text: &str) -> Option<Self> {
        text.trim().parse().ok()
    }

    fn to_text(&self) -> Cow<'_, str> {
        Cow::Owned(self.to_string())
    }
}

impl Leaf for f64 {
    const DATATYPE: &'static str = "xsd:decimal";

    fn from_text(text: &str) -> Option<Self> {
        text.trim()
            .parse()
            .ok()
            .filter(|value: &f64| value.is_finite())
    }

    fn to_text(&self) -> Cow<'_, str> {
        Cow::Owned(self.to_string())
    }
}

/// Declares a lexical type: a value kept in the form it was written, with
/// what the form denotes read out on demand.
macro_rules! lexical {
    ($(#[$meta:meta])* $name:ident as $datatype:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(pub Box<str>);

        impl $name {
            /// The value as written.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(text: &str) -> Self {
                $name(text.into())
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl Leaf for $name {
            const DATATYPE: &'static str = $datatype;

            fn from_text(text: &str) -> Option<Self> {
                Some($name(text.into()))
            }

            fn to_text(&self) -> Cow<'_, str> {
                Cow::Borrowed(&self.0)
            }
        }
    };
}

lexical! {
    /// An `xsd:dateTime`, kept as written: `2026-09-19T15:04:00`, with or
    /// without a zone offset.
    DateTime as "xsd:dateTime"
}

lexical! {
    /// An `xsd:date`, kept as written: `2026-09-05`.
    Date as "xsd:date"
}

lexical! {
    /// An `xsd:time`, kept as written: `11:00:00`.
    Time as "xsd:time"
}

lexical! {
    /// An `xsd:duration`, kept as written: `PT2H30M`.
    Duration as "xsd:duration"
}

impl DateTime {
    /// The date part, if the form has one.
    pub fn date(&self) -> Option<Date> {
        let (date, _) = self.0.split_once('T')?;
        Date::from(date).ymd().map(|_| Date::from(date))
    }

    /// The time part, if the form has one.
    pub fn time(&self) -> Option<Time> {
        let (_, rest) = self.0.split_once('T')?;
        let end = rest.find(['Z', '+', '-']).unwrap_or(rest.len());
        let time = Time::from(&rest[..end]);
        time.seconds_since_midnight().map(|_| time)
    }

    /// Whether the form states a zone after its time — `Z` or an offset,
    /// `+01:00` —, which makes it an instant rather than a local time.
    pub fn states_zone(&self) -> bool {
        self.0
            .split_once('T')
            .is_some_and(|(_, rest)| rest.contains(['Z', '+', '-']))
    }
}

impl Date {
    /// Year, month and day, if the form is a date.
    pub fn ymd(&self) -> Option<(i32, u8, u8)> {
        let text = self.0.strip_prefix('-').unwrap_or(&self.0);
        let mut parts = text.splitn(3, '-');
        let year: i32 = parts.next()?.parse().ok()?;
        let month: u8 = parts.next()?.parse().ok()?;
        let rest = parts.next()?;
        let day: u8 = rest[..rest.find(['Z', '+', '-']).unwrap_or(rest.len())]
            .parse()
            .ok()?;
        (1..=12).contains(&month).then_some(())?;
        (1..=31).contains(&day).then_some(())?;
        Some((
            if self.0.starts_with('-') { -year } else { year },
            month,
            day,
        ))
    }

    /// The date `days` after 1970-01-01, written `YYYY-MM-DD`: the inverse
    /// of [`days_since_epoch`](Self::days_since_epoch).
    pub fn from_days_since_epoch(days: i64) -> Date {
        // Howard Hinnant's civil_from_days.
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let day_of_era = z.rem_euclid(146_097);
        let year_of_era =
            (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
        let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
        let shifted_month = (5 * day_of_year + 2) / 153;
        let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
        let month = if shifted_month < 10 {
            shifted_month + 3
        } else {
            shifted_month - 9
        };
        let year = year_of_era + era * 400 + i64::from(month <= 2);
        Date::from(format!("{year:04}-{month:02}-{day:02}").as_str())
    }

    /// The number of days since 1970-01-01, if the form is a date.
    pub fn days_since_epoch(&self) -> Option<i64> {
        let (year, month, day) = self.ymd()?;
        // Howard Hinnant's days_from_civil.
        let year = i64::from(year) - i64::from(month <= 2);
        let era = year.div_euclid(400);
        let year_of_era = year - era * 400;
        let month = i64::from(month);
        let day_of_year =
            (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + i64::from(day) - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
        Some(era * 146_097 + day_of_era - 719_468)
    }
}

impl Time {
    /// Seconds since midnight, if the form is a time of day. Fractions of a
    /// second and a zone offset are accepted and ignored.
    pub fn seconds_since_midnight(&self) -> Option<u32> {
        Self::seconds_of(&self.0)
    }

    /// Seconds since midnight of the form `text`, if it is a time of day, as
    /// [`seconds_since_midnight`](Self::seconds_since_midnight) reads it: for
    /// a reader that borrows the form from the document rather than keeping it.
    pub fn seconds_of(text: &str) -> Option<u32> {
        let end = text.find(['Z', '+']).unwrap_or(text.len());
        let end = text[..end]
            .rfind('-')
            .map_or(end, |at| if at > 7 { at } else { end });
        let mut parts = text[..end].splitn(3, ':');
        let hours: u32 = parts.next()?.parse().ok()?;
        let minutes: u32 = parts.next()?.parse().ok()?;
        let seconds = parts.next()?;
        let seconds: u32 = seconds.split('.').next()?.parse().ok()?;
        (hours <= 24 && minutes < 60 && seconds < 60).then_some(())?;
        Some(hours * 3600 + minutes * 60 + seconds)
    }
}

impl Duration {
    /// The duration in whole seconds, a fraction of a second dropped, if the
    /// form is an `xsd:duration` without a year or month part (which have no
    /// fixed length).
    pub fn seconds(&self) -> Option<u64> {
        self.milliseconds().map(|milliseconds| milliseconds / 1000)
    }

    /// The duration in milliseconds, a fraction of a second read to the
    /// millisecond, if the form is an `xsd:duration` without a year or month
    /// part: the Dutch deliveries state run times to the tenth of a second.
    pub fn milliseconds(&self) -> Option<u64> {
        let text = self.0.strip_prefix('P')?;
        let (date_part, time_part) = match text.split_once('T') {
            Some((d, t)) => (d, Some(t)),
            None => (text, None),
        };
        let mut total = 0u64;
        let mut number = String::new();
        for c in date_part.chars() {
            match c {
                '0'..='9' => number.push(c),
                'D' => total += number.parse::<u64>().ok()? * 86_400_000,
                _ => return None,
            }
            if !c.is_ascii_digit() {
                number.clear();
            }
        }
        if !number.is_empty() {
            return None;
        }
        if let Some(time_part) = time_part {
            // The digits after the point, the first three of them: milliseconds.
            let mut fraction: Option<String> = None;
            for c in time_part.chars() {
                match (c, &mut fraction) {
                    ('0'..='9', None) => number.push(c),
                    ('0'..='9', Some(digits)) => {
                        if digits.len() < 3 {
                            digits.push(c);
                        }
                    }
                    ('.', None) => fraction = Some(String::new()),
                    ('H', _) => total += number.parse::<u64>().ok()? * 3_600_000,
                    ('M', _) => total += number.parse::<u64>().ok()? * 60_000,
                    ('S', _) => {
                        let thousandths = fraction.as_deref().map_or(Some(0), |digits| {
                            format!("{digits:0<3}").parse::<u64>().ok()
                        })?;
                        total += number.parse::<u64>().ok()? * 1000 + thousandths;
                    }
                    _ => return None,
                }
                if !c.is_ascii_digit() && c != '.' {
                    number.clear();
                    fraction = None;
                }
            }
            if !number.is_empty() {
                return None;
            }
        }
        Some(total)
    }
}

netex_element! {
    /// A key and its value, in an entity's `keyList`. The German delivery
    /// carries the DHID of a stop here, under the key `GlobalID`.
    pub struct KeyValue {
        attributes {
            /// The `typeOfKey` attribute.
            pub type_of_key: Option<Box<str>> = "typeOfKey",
        }
        children {
            /// The key.
            pub key: Option<Box<str>> = "Key",
            /// The value.
            pub value: Option<Box<str>> = "Value",
        }
    }
}

netex_element! {
    /// An entity's key-value pairs.
    pub struct KeyList {
        attributes {}
        children {
            /// The pairs.
            pub key_values: Vec<KeyValue> = "KeyValue",
        }
    }
}

impl KeyList {
    /// The value under `key`, if the list has it.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.key_values
            .iter()
            .find(|kv| kv.key.as_deref() == Some(key))
            .and_then(|kv| kv.value.as_deref())
    }
}

netex_element! {
    /// The codes an entity is known by in other systems, each with its type:
    /// NeTEx 2.0's `privateCodes`, beside the one `PrivateCode` of 1.x. The
    /// Dutch stop register names its quays and stop places so (`QuayCode`
    /// `NL:Q:…`, `StopPlaceCode` `NL:S:…`).
    pub struct PrivateCodes {
        attributes {}
        children {
            /// The codes.
            pub codes: Vec<PrivateCode> = "PrivateCode",
        }
    }
}

impl PrivateCodes {
    /// The code of type `kind`, if the list has one.
    pub fn get(&self, kind: &str) -> Option<&str> {
        self.codes
            .iter()
            .find(|code| code.r#type.as_deref() == Some(kind))
            .map(|code| &*code.text)
    }
}

netex_element! {
    /// A point on the earth: the schema's `LocationStructure`.
    pub struct Location {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `srsName` attribute: the coordinate reference system, when not the frame's default.
            pub srs_name: Option<Box<str>> = "srsName",
        }
        children {
            /// Longitude in decimal degrees.
            pub longitude: Option<f64> = "Longitude",
            /// Latitude in decimal degrees.
            pub latitude: Option<f64> = "Latitude",
            /// Altitude in metres.
            pub altitude: Option<f64> = "Altitude",
            /// Coordinates in the `srsName` system, as text.
            pub coordinates: Option<Box<str>> = "Coordinates",
            /// Precision of the coordinates, in metres.
            pub precision: Option<f64> = "Precision",
        }
    }
}

netex_element! {
    /// The centre of a zone or site.
    pub struct Centroid {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
        }
        children {
            /// Name of the point.
            pub name: Option<MultilingualString> = "Name",
            /// Where it is.
            pub location: Option<Location> = "Location",
        }
    }
}

netex_element! {
    /// Time zone and language: the schema's `LocaleStructure`.
    pub struct Locale {
        attributes {}
        children {
            /// Offset from UTC, in hours.
            pub time_zone_offset: Option<Box<str>> = "TimeZoneOffset",
            /// The IANA time zone: `Europe/Berlin`.
            pub time_zone: Option<Box<str>> = "TimeZone",
            /// Offset from UTC in summer, in hours.
            pub summer_time_zone_offset: Option<Box<str>> = "SummerTimeZoneOffset",
            /// The IANA time zone in summer.
            pub summer_time_zone: Option<Box<str>> = "SummerTimeZone",
            /// The default language, as an RFC 5646 tag.
            pub default_language: Option<Box<str>> = "DefaultLanguage",
        }
    }
}

netex_element! {
    /// How to reach an organisation.
    pub struct ContactDetails {
        attributes {}
        children {
            /// Who to ask for.
            pub contact_person: Option<MultilingualString> = "ContactPerson",
            /// Email address.
            pub email: Option<Box<str>> = "Email",
            /// Telephone number.
            pub phone: Option<Box<str>> = "Phone",
            /// Fax number.
            pub fax: Option<Box<str>> = "Fax",
            /// Web address.
            pub url: Option<Box<str>> = "Url",
            /// Anything else.
            pub further_details: Option<MultilingualString> = "FurtherDetails",
        }
    }
}

netex_element! {
    /// Another name for an entity, in another language or of another kind.
    pub struct AlternativeName {
        attributes {
            /// The `id` attribute.
            pub id: Option<Id> = "id",
            /// The `version` attribute.
            pub version: Option<Version> = "version",
        }
        children {
            /// The entity named.
            pub named_object_ref: Option<Ref> = "NamedObjectRef",
            /// The language of the name.
            pub lang: Option<Box<str>> = "Lang",
            /// The kind of name: `alias`, `translation`, `other`.
            pub name_type: Option<Box<str>> = "NameType",
            /// The kind of name, as a token: `label`, `translation`, …
            pub type_of_name: Option<Box<str>> = "TypeOfName",
            /// The name.
            pub name: Option<MultilingualString> = "Name",
            /// Its short form.
            pub short_name: Option<MultilingualString> = "ShortName",
            /// Its abbreviation.
            pub abbreviation: Option<MultilingualString> = "Abbreviation",
            /// A qualifier: "Hauptbahnhof" for "Hannover".
            pub qualifier_name: Option<MultilingualString> = "QualifierName",
        }
    }
}

netex_element! {
    /// An entity's alternative names.
    pub struct AlternativeNames {
        attributes {}
        children {
            /// The names.
            pub alternative_names: Vec<AlternativeName> = "AlternativeName",
        }
    }
}

impl Walk for Ref {
    fn walk_unprocessed(&self, _parent: &str, _visit: &mut dyn FnMut(&str, &Unprocessed)) {}
}

impl Walk for PrivateCode {
    fn walk_unprocessed(&self, _parent: &str, _visit: &mut dyn FnMut(&str, &Unprocessed)) {}
}

impl Walk for MultilingualString {
    fn walk_unprocessed(&self, parent: &str, visit: &mut dyn FnMut(&str, &Unprocessed)) {
        for fragment in &self.unprocessed {
            visit(parent, fragment);
        }
        for translation in &self.translations {
            translation.walk_unprocessed("Text", visit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Duration;

    /// A duration read to the millisecond, a fraction of a second as written
    /// to the tenth, the hundredth or beyond; whole seconds as before.
    #[test]
    fn a_duration_reads_to_the_millisecond() {
        let read = |text: &str| {
            let duration = Duration::from(text);
            (duration.milliseconds(), duration.seconds())
        };
        assert_eq!(read("PT1M3.4S"), (Some(63_400), Some(63)));
        assert_eq!(read("PT0.05S"), (Some(50), Some(0)));
        assert_eq!(read("PT1.23456S"), (Some(1_234), Some(1)));
        assert_eq!(read("P1DT2H"), (Some(93_600_000), Some(93_600)));
        assert_eq!(read("PT30S"), (Some(30_000), Some(30)));
        assert_eq!(read("P1M"), (None, None), "a month has no fixed length");
        assert_eq!(read("PT1.2.3S"), (None, None));
        assert_eq!(read("PT.5S"), (None, None));
    }
}
