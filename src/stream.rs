//! The lean mode: a document read entity by entity, typing only what the
//! caller asks for.
//!
//! [`read`] walks the frames of a `<PublicationDelivery>` and hands every
//! entity of a selected [`Kind`] to a sink as soon as it is read, typed with
//! the same readers the full model uses — a stop place comes with its quays, a
//! journey with its passing times. Every element outside the selection is
//! skipped at the tokenizer: still checked for well-formedness and the limits,
//! never typed, never copied. Nothing of the document is kept: what the sink
//! does not hold on to is gone when it returns.
//!
//! The sink also sees the frames the entity stands in — their identifiers and
//! `ValidBetween` periods — which is what an importer needs to know when an
//! entity is valid.
//!
//! An importer that needs more of a document than entities of a kind — the
//! time zone of a frame's `FrameDefaults`, a container the crate does not
//! type, an entity read its own way — reads it in the same walk with
//! [`read_with`]: every element the walk would skip is handed to its [`Sink`]
//! first, with where it stands ([`Skipped`]), to be read with the cursor or
//! skipped. The document is tokenized once either way.

use crate::error::{Kind as ErrorKind, Result};
use crate::model::{self, Element, Id, ValidBetween, Version};
use crate::xml::{Child, Cursor, Limits};

/// Declares the kinds of entity the lean mode can yield: one per element name
/// a frame's containers hold, each read into the type the full model uses.
macro_rules! entities {
    ($($(#[$meta:meta])* $variant:ident($ty:ty) = $element:literal,)*) => {
        /// A kind of entity: an element name a frame's containers hold.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[non_exhaustive]
        pub enum Kind {
            $(
                #[doc = concat!("A `<", $element, ">`.")]
                $variant,
            )*
        }

        impl Kind {
            /// Every kind, in declaration order.
            pub const ALL: &'static [Kind] = &[$(Kind::$variant),*];

            /// The element name.
            pub fn element(self) -> &'static str {
                match self {
                    $(Kind::$variant => $element,)*
                }
            }
        }

        /// One entity of a document, typed.
        #[derive(Debug, Clone, PartialEq)]
        #[non_exhaustive]
        pub enum Entity {
            $(
                $(#[$meta])*
                $variant($ty),
            )*
        }

        impl Entity {
            /// What kind of entity this is.
            pub fn kind(&self) -> Kind {
                match self {
                    $(Entity::$variant(_) => Kind::$variant,)*
                }
            }
        }

        fn read_entity<'i>(kind: Kind, cursor: &mut Cursor<'i>, child: &Child<'i>) -> Result<Entity> {
            match kind {
                $(Kind::$variant => <$ty as Element>::read(cursor, child).map(Entity::$variant),)*
            }
        }
    };
}

entities! {
    /// A town, district or region.
    TopographicPlace(model::TopographicPlace) = "TopographicPlace",
    /// A group of stop places.
    GroupOfStopPlaces(model::GroupOfStopPlaces) = "GroupOfStopPlaces",
    /// A stop place, with its quays, levels and entrances.
    StopPlace(model::StopPlace) = "StopPlace",
    /// A data source.
    DataSource(model::DataSource) = "DataSource",
    /// A responsibility set.
    ResponsibilitySet(model::ResponsibilitySet) = "ResponsibilitySet",
    /// An operator.
    Operator(model::Organisation) = "Operator",
    /// An authority.
    Authority(model::Organisation) = "Authority",
    /// A vehicle type.
    VehicleType(model::VehicleType) = "VehicleType",
    /// A direction of travel.
    Direction(model::Direction) = "Direction",
    /// A point a route goes through.
    RoutePoint(model::RoutePoint) = "RoutePoint",
    /// A route.
    Route(model::Route) = "Route",
    /// A line.
    Line(model::Line) = "Line",
    /// A destination display.
    DestinationDisplay(model::DestinationDisplay) = "DestinationDisplay",
    /// A scheduled stop point.
    ScheduledStopPoint(model::ScheduledStopPoint) = "ScheduledStopPoint",
    /// A link between stop points.
    ServiceLink(model::ServiceLink) = "ServiceLink",
    /// A walk between sites.
    SiteConnection(model::SiteConnection) = "SiteConnection",
    /// Which stop place or quay a stop point is.
    PassengerStopAssignment(model::PassengerStopAssignment) = "PassengerStopAssignment",
    /// A service journey pattern, with its points in sequence.
    ServiceJourneyPattern(model::ServiceJourneyPattern) = "ServiceJourneyPattern",
    /// A journey pattern, with its points in sequence.
    JourneyPattern(model::ServiceJourneyPattern) = "JourneyPattern",
    /// A notice.
    Notice(model::Notice) = "Notice",
    /// Where a notice applies, when assigned at frame level.
    NoticeAssignment(model::NoticeAssignment) = "NoticeAssignment",
    /// A day type.
    DayType(model::DayType) = "DayType",
    /// An operating day.
    OperatingDay(model::OperatingDay) = "OperatingDay",
    /// An operating period.
    OperatingPeriod(model::OperatingPeriod) = "OperatingPeriod",
    /// An operating period with a bit per day.
    UicOperatingPeriod(model::OperatingPeriod) = "UicOperatingPeriod",
    /// Which day type a day is.
    DayTypeAssignment(model::DayTypeAssignment) = "DayTypeAssignment",
    /// The days an entity is available on, defined in a frame's `validityConditions`.
    AvailabilityCondition(model::AvailabilityCondition) = "AvailabilityCondition",
    /// A journey, with its passing times.
    ServiceJourney(model::ServiceJourney) = "ServiceJourney",
    /// A journey on one day, with its passing times.
    DatedServiceJourney(model::ServiceJourney) = "DatedServiceJourney",
    /// A journey standing for runs at a headway, with its calls or passing times.
    TemplateServiceJourney(model::ServiceJourney) = "TemplateServiceJourney",
    /// A train number journeys name.
    TrainNumber(model::TrainNumber) = "TrainNumber",
    /// A planned interchange between journeys.
    ServiceJourneyInterchange(model::ServiceJourneyInterchange) = "ServiceJourneyInterchange",
    /// A rule for interchanges.
    InterchangeRule(model::InterchangeRule) = "InterchangeRule",
}

/// The kinds of entity a read yields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Selection {
    /// One bit per [`Kind`], at the kind's position in [`Kind::ALL`].
    bits: u64,
}

const _: () = assert!(
    Kind::ALL.len() <= u64::BITS as usize,
    "a selection holds one bit per kind"
);

impl Selection {
    /// No kind at all: the document is checked and nothing is yielded.
    pub const fn none() -> Self {
        Selection { bits: 0 }
    }

    /// Every kind.
    pub fn all() -> Self {
        Kind::ALL
            .iter()
            .fold(Selection::none(), |selection, &kind| selection.with(kind))
    }

    /// What a routing importer reads: stops with their quays, routes and
    /// lines, stop points and their assignments, journey patterns, journeys
    /// with their passing times or calls, template journeys, the calendar with
    /// its availability conditions, walks, interchanges, and notices.
    pub fn routing() -> Self {
        [
            Kind::StopPlace,
            Kind::Route,
            Kind::Line,
            Kind::ScheduledStopPoint,
            Kind::PassengerStopAssignment,
            Kind::ServiceJourneyPattern,
            Kind::JourneyPattern,
            Kind::SiteConnection,
            Kind::Notice,
            Kind::NoticeAssignment,
            Kind::DayType,
            Kind::OperatingDay,
            Kind::OperatingPeriod,
            Kind::UicOperatingPeriod,
            Kind::DayTypeAssignment,
            Kind::AvailabilityCondition,
            Kind::ServiceJourney,
            Kind::DatedServiceJourney,
            Kind::TemplateServiceJourney,
            Kind::ServiceJourneyInterchange,
        ]
        .into_iter()
        .fold(Selection::none(), Selection::with)
    }

    /// The selection with `kind` added.
    pub const fn with(self, kind: Kind) -> Self {
        Selection {
            bits: self.bits | 1 << kind as u32,
        }
    }

    /// The selection with `kind` taken out.
    pub const fn without(self, kind: Kind) -> Self {
        Selection {
            bits: self.bits & !(1 << kind as u32),
        }
    }

    /// Whether `kind` is selected.
    pub const fn contains(self, kind: Kind) -> bool {
        self.bits & (1 << kind as u32) != 0
    }

    /// Whether any of `kinds` is selected.
    fn contains_any(self, kinds: &[Kind]) -> bool {
        kinds.iter().any(|&kind| self.contains(kind))
    }
}

/// How a document is read lean: within which limits, yielding which kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadOptions {
    /// The limits the document must stay within.
    pub limits: Limits,
    /// The kinds of entity yielded.
    pub selection: Selection,
}

impl ReadOptions {
    /// Default limits, yielding `selection`.
    pub fn new(selection: Selection) -> Self {
        ReadOptions {
            limits: Limits::default(),
            selection,
        }
    }

    /// The same options within `limits`.
    pub fn with_limits(mut self, limits: Limits) -> Self {
        self.limits = limits;
        self
    }
}

impl Default for ReadOptions {
    /// Default limits, every kind.
    fn default() -> Self {
        ReadOptions::new(Selection::all())
    }
}

/// A kind of frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FrameKind {
    /// A `<CompositeFrame>`.
    Composite,
    /// A `<ResourceFrame>`.
    Resource,
    /// A `<SiteFrame>`.
    Site,
    /// A `<ServiceFrame>`.
    Service,
    /// A `<ServiceCalendarFrame>`.
    ServiceCalendar,
    /// A `<TimetableFrame>`.
    Timetable,
}

impl FrameKind {
    fn of(element: &str) -> Option<FrameKind> {
        Some(match element {
            "CompositeFrame" => FrameKind::Composite,
            "ResourceFrame" => FrameKind::Resource,
            "SiteFrame" => FrameKind::Site,
            "ServiceFrame" => FrameKind::Service,
            "ServiceCalendarFrame" => FrameKind::ServiceCalendar,
            "TimetableFrame" => FrameKind::Timetable,
            _ => return None,
        })
    }
}

/// A frame an entity stands in: what of it is known before its entities.
#[derive(Debug, Clone, PartialEq)]
pub struct FrameHeader {
    /// What kind of frame.
    pub kind: FrameKind,
    /// The `id` attribute.
    pub id: Option<Id>,
    /// The `version` attribute.
    pub version: Option<Version>,
    /// The frame's `ValidBetween` periods, those written before its entities.
    pub valid_between: Vec<ValidBetween>,
}

/// The frames an entity stands in, outermost first.
#[derive(Debug, Default)]
pub struct Scope {
    frames: Vec<FrameHeader>,
}

impl Scope {
    /// The frames, outermost first; the last is the entity's own.
    pub fn frames(&self) -> &[FrameHeader] {
        &self.frames
    }
}

/// The containers of a frame, each with the kinds of entity it holds.
type Containers = &'static [(&'static str, &'static [Kind])];

/// A frame's validity conditions: the Swiss and Austrian deliveries define
/// their availability conditions there.
const CONDITIONS: (&str, &[Kind]) = ("validityConditions", &[Kind::AvailabilityCondition]);

const COMPOSITE: Containers = &[CONDITIONS];

const RESOURCE: Containers = &[
    CONDITIONS,
    ("dataSources", &[Kind::DataSource]),
    ("responsibilitySets", &[Kind::ResponsibilitySet]),
    ("organisations", &[Kind::Operator, Kind::Authority]),
    ("vehicleTypes", &[Kind::VehicleType]),
];

const SITE: Containers = &[
    CONDITIONS,
    ("topographicPlaces", &[Kind::TopographicPlace]),
    ("groupsOfStopPlaces", &[Kind::GroupOfStopPlaces]),
    ("stopPlaces", &[Kind::StopPlace]),
];

const SERVICE: Containers = &[
    CONDITIONS,
    ("directions", &[Kind::Direction]),
    ("routePoints", &[Kind::RoutePoint]),
    ("routes", &[Kind::Route]),
    ("lines", &[Kind::Line]),
    ("destinationDisplays", &[Kind::DestinationDisplay]),
    ("scheduledStopPoints", &[Kind::ScheduledStopPoint]),
    ("serviceLinks", &[Kind::ServiceLink]),
    ("connections", &[Kind::SiteConnection]),
    ("stopAssignments", &[Kind::PassengerStopAssignment]),
    (
        "journeyPatterns",
        &[Kind::ServiceJourneyPattern, Kind::JourneyPattern],
    ),
    ("notices", &[Kind::Notice]),
    ("noticeAssignments", &[Kind::NoticeAssignment]),
];

/// The calendar's containers: those of a `<ServiceCalendarFrame>` and of the
/// `<ServiceCalendar>` it may wrap.
const CALENDAR: Containers = &[
    CONDITIONS,
    ("dayTypes", &[Kind::DayType]),
    ("operatingDays", &[Kind::OperatingDay]),
    (
        "operatingPeriods",
        &[Kind::OperatingPeriod, Kind::UicOperatingPeriod],
    ),
    ("dayTypeAssignments", &[Kind::DayTypeAssignment]),
];

const TIMETABLE: Containers = &[
    CONDITIONS,
    (
        "vehicleJourneys",
        &[
            Kind::ServiceJourney,
            Kind::DatedServiceJourney,
            Kind::TemplateServiceJourney,
        ],
    ),
    ("trainNumbers", &[Kind::TrainNumber]),
    ("notices", &[Kind::Notice]),
    ("noticeAssignments", &[Kind::NoticeAssignment]),
    ("journeyInterchanges", &[Kind::ServiceJourneyInterchange]),
    ("interchangeRules", &[Kind::InterchangeRule]),
    ("vehicleTypes", &[Kind::VehicleType]),
];

/// Where the walk of [`read_with`] met an element it does not read itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Skipped<'a> {
    /// A child of the `<PublicationDelivery>` other than its `dataObjects`: its
    /// timestamp, its participant, its description.
    Delivery,
    /// A child of a list of frames — the root's `dataObjects`, a composite
    /// frame's `frames` — that is no frame the walk reads: a `GeneralFrame`, a
    /// `FareFrame`, an element of another namespace.
    Frames,
    /// A child of a frame of this kind that the walk does not read: anything
    /// but the frame's `ValidBetween`, a composite frame's `frames`, a calendar
    /// frame's `ServiceCalendar` and the containers holding a selected kind —
    /// among them the frame's `validityConditions`, holding availability
    /// conditions — the frame's `FrameDefaults`, say, or a container of no
    /// selected kind.
    Frame(FrameKind),
    /// A child of a calendar frame's `<ServiceCalendar>` that is no container
    /// holding a selected kind.
    Calendar,
    /// An item of no selected kind in the container of this name.
    Item(&'a str),
}

/// What the walk of [`read_with`] hands what it reads to: the entities of the
/// selected kinds, and every element it does not read itself.
pub trait Sink<'i> {
    /// Takes an entity of a selected kind, with the frames it stands in.
    ///
    /// An error ends the read and is returned as it is.
    fn entity(&mut self, entity: Entity, scope: &Scope) -> Result<()>;

    /// Takes an element the walk would skip, in document order: the cursor
    /// standing at the element's start tag, the element, where it stands, and
    /// the frames it stands in.
    ///
    /// The sink reads the element with the cursor — an
    /// [`Element`] reader, or [`Cursor::next_child`] and
    /// [`Cursor::text`] over its children between [`Cursor::enter`] and
    /// [`Cursor::leave`] — or skips it with [`Cursor::skip_over`], as it does
    /// unless it says otherwise; either way the cursor must stand past the
    /// element's end tag when it returns, as after any reader. An error ends
    /// the read and is returned as it is.
    fn skipped(
        &mut self,
        cursor: &mut Cursor<'i>,
        element: &Child<'i>,
        place: Skipped<'i>,
        scope: &Scope,
    ) -> Result<()> {
        let _ = (place, scope);
        cursor.skip_over(element)
    }
}

/// A closure taking the entities, as the sink of [`read`].
struct Entities<F>(F);

impl<F> Sink<'_> for Entities<F>
where
    F: FnMut(Entity, &Scope) -> Result<()>,
{
    fn entity(&mut self, entity: Entity, scope: &Scope) -> Result<()> {
        (self.0)(entity, scope)
    }
}

/// Reads `xml` lean: every entity of a kind in `options.selection` goes to
/// `sink` with the frames it stands in, as it is read.
///
/// An error of the sink ends the read and is returned as it is.
pub fn read<F>(xml: &str, options: &ReadOptions, sink: F) -> Result<()>
where
    F: FnMut(Entity, &Scope) -> Result<()>,
{
    read_with(xml, options, &mut Entities(sink))
}

/// Reads `xml` lean as [`read`] does, and hands every element the walk would
/// skip to the sink as well ([`Sink::skipped`]), so that what the crate does
/// not yield as an entity is read in the same walk.
///
/// ```
/// use netex_rs::model::{Element, FrameDefaults};
/// use netex_rs::stream::{self, Entity, Kind, ReadOptions, Scope, Selection, Sink, Skipped};
/// use netex_rs::xml::{Child, Cursor};
///
/// /// Counts the lines, and keeps the time zone of the frame defaults.
/// #[derive(Default)]
/// struct Lines {
///     lines: usize,
///     zone: Option<Box<str>>,
/// }
///
/// impl<'i> Sink<'i> for Lines {
///     fn entity(&mut self, _: Entity, _: &Scope) -> netex_rs::Result<()> {
///         self.lines += 1;
///         Ok(())
///     }
///
///     fn skipped(&mut self, cursor: &mut Cursor<'i>, element: &Child<'i>, place: Skipped<'i>, _: &Scope) -> netex_rs::Result<()> {
///         match place {
///             Skipped::Frame(_) if element.local == "FrameDefaults" => {
///                 let defaults = FrameDefaults::read(cursor, element)?;
///                 self.zone = defaults.default_locale.and_then(|locale| locale.time_zone);
///                 Ok(())
///             }
///             _ => cursor.skip_over(element),
///         }
///     }
/// }
///
/// let xml = r#"<PublicationDelivery xmlns="http://www.netex.org.uk/netex" version="1.10">
///   <dataObjects><CompositeFrame id="c" version="1">
///     <FrameDefaults><DefaultLocale><TimeZone>Europe/Vienna</TimeZone></DefaultLocale></FrameDefaults>
///     <frames><ServiceFrame id="s" version="1"><lines><Line id="l:1" version="1"><Name>U1</Name></Line></lines></ServiceFrame></frames>
///   </CompositeFrame></dataObjects>
/// </PublicationDelivery>"#;
/// let mut lines = Lines::default();
/// stream::read_with(xml, &ReadOptions::new(Selection::none().with(Kind::Line)), &mut lines)?;
/// assert_eq!((lines.lines, lines.zone.as_deref()), (1, Some("Europe/Vienna")));
/// # Ok::<(), netex_rs::Error>(())
/// ```
pub fn read_with<'i, S: Sink<'i>>(xml: &'i str, options: &ReadOptions, sink: &mut S) -> Result<()> {
    let mut cursor = Cursor::new(xml, options.limits);
    let root = cursor.root()?;
    if !(root.netex && root.local == "PublicationDelivery") {
        return Err(cursor.error_at(
            ErrorKind::NotNetex {
                root: root.local.to_owned(),
            },
            root.offset,
        ));
    }
    let mut walker = Walker {
        cursor,
        selection: options.selection,
        scope: Scope::default(),
        sink,
    };
    walker.delivery(&root)?;
    walker.cursor.finish()
}

/// The walk down a document: frames, their containers, their entities.
struct Walker<'i, 's, S> {
    cursor: Cursor<'i>,
    selection: Selection,
    scope: Scope,
    sink: &'s mut S,
}

impl<'i, S: Sink<'i>> Walker<'i, '_, S> {
    /// Hands an element the walk does not read to the sink.
    fn skip(&mut self, element: &Child<'i>, place: Skipped<'i>) -> Result<()> {
        self.sink
            .skipped(&mut self.cursor, element, place, &self.scope)
    }

    fn delivery(&mut self, root: &Child<'i>) -> Result<()> {
        self.cursor.enter(root)?;
        if !root.empty {
            while let Some(child) = self.cursor.next_child()? {
                if child.netex && child.local == "dataObjects" {
                    self.frames(&child)?;
                } else {
                    self.skip(&child, Skipped::Delivery)?;
                }
            }
        }
        self.cursor.leave();
        Ok(())
    }

    /// A list of frames: the root's `dataObjects` or a composite's `frames`.
    fn frames(&mut self, list: &Child<'i>) -> Result<()> {
        self.cursor.enter(list)?;
        if !list.empty {
            while let Some(child) = self.cursor.next_child()? {
                match FrameKind::of(child.local).filter(|_| child.netex) {
                    Some(kind) => self.frame(kind, &child)?,
                    None => self.skip(&child, Skipped::Frames)?,
                }
            }
        }
        self.cursor.leave();
        Ok(())
    }

    fn frame(&mut self, kind: FrameKind, frame: &Child<'i>) -> Result<()> {
        let mut header = FrameHeader {
            kind,
            id: None,
            version: None,
            valid_between: Vec::new(),
        };
        for attribute in frame.attributes() {
            let (key, value) = attribute.map_err(|e| self.cursor.error_at(e.kind, frame.offset))?;
            match key {
                "id" => header.id = Some(Id::from(value.as_ref())),
                "version" => header.version = Some(Version::from(value.as_ref())),
                _ => {}
            }
        }
        self.scope.frames.push(header);
        self.cursor.enter(frame)?;
        if !frame.empty {
            while let Some(child) = self.cursor.next_child()? {
                if !child.netex {
                    self.skip(&child, Skipped::Frame(kind))?;
                    continue;
                }
                let place = Skipped::Frame(kind);
                match (kind, child.local) {
                    (_, "ValidBetween") => {
                        let period = ValidBetween::read(&mut self.cursor, &child)?;
                        self.scope
                            .frames
                            .last_mut()
                            .expect("the frame was pushed above")
                            .valid_between
                            .push(period);
                    }
                    (FrameKind::Composite, "frames") => self.frames(&child)?,
                    (FrameKind::ServiceCalendar, "ServiceCalendar") => {
                        self.containers(&child, CALENDAR)?;
                    }
                    (FrameKind::Resource, _) => self.container(&child, RESOURCE, place)?,
                    (FrameKind::Site, _) => self.container(&child, SITE, place)?,
                    (FrameKind::Service, _) => self.container(&child, SERVICE, place)?,
                    (FrameKind::ServiceCalendar, _) => self.container(&child, CALENDAR, place)?,
                    (FrameKind::Timetable, _) => self.container(&child, TIMETABLE, place)?,
                    (FrameKind::Composite, _) => self.container(&child, COMPOSITE, place)?,
                }
            }
        }
        self.cursor.leave();
        self.scope.frames.pop();
        Ok(())
    }

    /// An element holding containers — the `<ServiceCalendar>` a calendar
    /// frame may wrap.
    fn containers(&mut self, element: &Child<'i>, containers: Containers) -> Result<()> {
        self.cursor.enter(element)?;
        if !element.empty {
            while let Some(child) = self.cursor.next_child()? {
                if child.netex {
                    self.container(&child, containers, Skipped::Calendar)?;
                } else {
                    self.skip(&child, Skipped::Calendar)?;
                }
            }
        }
        self.cursor.leave();
        Ok(())
    }

    /// A child of a frame, or of a `<ServiceCalendar>`, standing at `place`:
    /// one of `containers`, whose selected entities are yielded and whose other
    /// items are handed over, or anything else, which is handed over whole.
    fn container(
        &mut self,
        child: &Child<'i>,
        containers: Containers,
        place: Skipped<'i>,
    ) -> Result<()> {
        let kinds = containers
            .iter()
            .find(|(name, _)| *name == child.local)
            .map(|(_, kinds)| *kinds)
            .filter(|kinds| self.selection.contains_any(kinds));
        let Some(kinds) = kinds else {
            return self.skip(child, place);
        };
        self.cursor.enter(child)?;
        if !child.empty {
            while let Some(item) = self.cursor.next_child()? {
                let kind = kinds
                    .iter()
                    .copied()
                    .find(|kind| item.netex && item.local == kind.element())
                    .filter(|&kind| self.selection.contains(kind));
                match kind {
                    Some(kind) => {
                        let entity = read_entity(kind, &mut self.cursor, &item)?;
                        self.sink.entity(entity, &self.scope)?;
                    }
                    None => self.skip(&item, Skipped::Item(child.local))?,
                }
            }
        }
        self.cursor.leave();
        Ok(())
    }
}
