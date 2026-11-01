//! Gathering the index: the entities and references of each document, as they
//! come; [`finish`](super::finish) resolves them once every document is in.

use std::collections::hash_map::Entry;
use std::hash::{Hash, Hasher};

use rustc_hash::{FxHashMap, FxHasher};

use super::findings::{Concern, FindingRecord};
use super::positions::{self, FrameSystems};
use super::scope::Table;
use super::strings::{Strings, Sym};
use super::{CallUse, Index, PassingTime, StopId, StopKind, Unresolved};
use crate::calendar::{assignment_rank, period_days, PeriodDays, Properties, Span};
use crate::geometry::first_position;
use crate::model::{
    AssignedDayType, AssignedDays, AvailabilityCondition, Date, DateTime, DayType, Frame,
    GeneralFrameMember, JourneyPatternItem, JourneyPatternRef, JourneyRef, LineRef, OperatingDay,
    OperatingDayRef, OperatingPeriod, OperatingPeriodItem, PeriodEnd, PeriodStart,
    PointInPatternRef, Quay, QuayItem, Ref, RouteRefOrView, ServiceJourney, ServiceJourneyPattern,
    SiteConnectionEnd, StopPlace, StopPointRef, StopUse, Time, ValidBetween, ValidityConditionItem,
    VehicleJourney,
};
use crate::stream::{self, Entity, ReadOptions, Scope, Sink, Skipped};
use crate::xml::{Child, Cursor};
use crate::Document;

/// An entity's key: its identifier and, when it states one, its version.
///
/// The schema's `VersionIdType`: "Consecutive VERSION numbers should be
/// strictly increasing." A reference resolves to the entity with the version it
/// names; a reference naming none resolves to the highest version, or, when
/// the builder is given an instant, to the version valid then.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Key {
    pub(crate) id: Sym,
    pub(crate) version: Option<Sym>,
}

/// The element a reference stands in: its name and its identifier.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Referrer {
    pub(crate) element: &'static str,
    pub(crate) id: Sym,
}

/// A reference as gathered: what it names, and where it stands — the
/// document it resolves in first, and what a finding about it names.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Pending {
    pub(crate) key: Key,
    /// The `versionRef` fallback, which the German delivery uses on some references.
    pub(crate) version_ref: Option<Sym>,
    pub(crate) element: &'static str,
    pub(crate) referrer: Referrer,
    pub(crate) origin: Sym,
}

/// The days an entity is valid, from its frame's `ValidBetween`, as days since
/// 1970-01-01; open ends are the extremes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Validity {
    pub(crate) from: i64,
    pub(crate) to: i64,
}

impl Validity {
    pub(crate) const ALWAYS: Validity = Validity {
        from: i64::MIN,
        to: i64::MAX,
    };

    fn of(valid_between: &[ValidBetween], inherited: Validity) -> Validity {
        let Some(first) = valid_between.first() else {
            return inherited;
        };
        let day = |date: &Option<DateTime>| {
            date.as_ref()
                .and_then(|d| d.date())
                .and_then(|d| d.days_since_epoch())
        };
        Validity {
            from: day(&first.from_date).unwrap_or(inherited.from),
            to: day(&first.to_date).unwrap_or(inherited.to),
        }
    }

    /// The validity of an entity within the frames of `scope`, outermost first.
    fn within(scope: &Scope) -> Validity {
        scope
            .frames()
            .iter()
            .fold(Validity::ALWAYS, |inherited, frame| {
                Validity::of(&frame.valid_between, inherited)
            })
    }

    pub(crate) fn covers(&self, day: i64) -> bool {
        self.from <= day && day <= self.to
    }
}

/// A fingerprint of what a definition states, over its text: two definitions
/// of one stop, stop point or level from different documents say the same
/// exactly when their fingerprints are equal, whichever builder gathered them.
fn fingerprint(content: impl Hash) -> u64 {
    let mut hasher = FxHasher::default();
    content.hash(&mut hasher);
    hasher.finish()
}

/// The fingerprint of a level: its name.
pub(crate) fn level_fingerprint(name: &str) -> u64 {
    fingerprint(name)
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StopRecord {
    pub(crate) key: Key,
    /// The document the kept definition is from.
    pub(crate) origin: Sym,
    /// The fingerprint of the kept definition.
    pub(crate) content: u64,
    pub(crate) dhid: Option<Sym>,
    pub(crate) name: Sym,
    pub(crate) kind: StopKind,
    pub(crate) latitude: Option<f64>,
    pub(crate) longitude: Option<f64>,
    pub(crate) parent: Option<StopId>,
    pub(crate) level_ref: Option<Pending>,
    pub(crate) level: Option<Sym>,
    pub(crate) validity: Validity,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StopPointRecord {
    pub(crate) key: Key,
    /// The document the kept definition is from.
    pub(crate) origin: Sym,
    /// The fingerprint of the kept definition.
    pub(crate) content: u64,
    pub(crate) name: Sym,
    pub(crate) latitude: Option<f64>,
    pub(crate) longitude: Option<f64>,
    pub(crate) stop: Option<StopId>,
    pub(crate) validity: Validity,
}

/// A level of a stop place: the name a quay on it is shown with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LevelRecord {
    pub(crate) name: Sym,
    /// The document the kept definition is from.
    pub(crate) origin: Sym,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LineRecord {
    pub(crate) key: Key,
    pub(crate) origin: Sym,
    pub(crate) name: Sym,
    pub(crate) public_code: Option<Sym>,
    pub(crate) mode: Option<Sym>,
    /// Whether it is a flexible line, served on demand.
    pub(crate) flexible: bool,
    /// The days the version is valid on, its frames' validity inherited.
    pub(crate) validity: Validity,
    /// The days the version states itself valid on: what it governs.
    pub(crate) span: Span,
}

impl LineRecord {
    /// Whether two definitions state the same line.
    pub(crate) fn says_the_same_as(&self, other: &LineRecord) -> bool {
        (
            self.name,
            self.public_code,
            self.mode,
            self.flexible,
            self.validity,
            self.span,
        ) == (
            other.name,
            other.public_code,
            other.mode,
            other.flexible,
            other.validity,
            other.span,
        )
    }
}

/// A route: the line it names, the one thing of it the index uses — the
/// line of a pattern or journey that names the route rather than a line.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RouteRecord {
    pub(crate) key: Key,
    pub(crate) origin: Sym,
    pub(crate) line: Option<Pending>,
    pub(crate) validity: Validity,
}

/// What a pattern states about stopping at one of its points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PointUse {
    /// Whether the vehicle passes the point without stopping.
    pub(crate) passes: bool,
    /// What a passenger may do there.
    pub(crate) call: CallUse,
}

impl PointUse {
    /// The use of a point stating `stop_use`, `ForBoarding` and `ForAlighting`
    /// as given: a point passed through is no call, one used for neither
    /// boarding nor alighting allows neither, whatever the flags say; and
    /// boarding and alighting are allowed where the point does not say.
    pub(crate) fn of(
        stop_use: Option<&StopUse>,
        boarding: Option<bool>,
        alighting: Option<bool>,
    ) -> PointUse {
        let neither = matches!(stop_use, Some(StopUse::NoBoardingOrAlighting));
        PointUse {
            passes: matches!(stop_use, Some(StopUse::Passthrough)),
            call: CallUse {
                boarding: !neither && boarding.unwrap_or(true),
                alighting: !neither && alighting.unwrap_or(true),
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PatternRecord {
    pub(crate) key: Key,
    pub(crate) origin: Sym,
    /// The line its route view names.
    pub(crate) line: Option<Pending>,
    /// The route it names, by reference or in its route view.
    pub(crate) route: Option<Pending>,
    /// The pattern's points: a range of the builder's `pattern_point_refs`,
    /// `pattern_point_keys` and `pattern_point_uses`, one entry per point.
    pub(crate) points: (u32, u32),
    pub(crate) validity: Validity,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct JourneyRecord {
    pub(crate) key: Key,
    pub(crate) origin: Sym,
    /// The line it names, by `LineRef` or `FlexibleLineRef`.
    pub(crate) line: Option<Pending>,
    /// Whether it names its line by `FlexibleLineRef`: a journey of a
    /// flexible line, whether the line resolves or not.
    pub(crate) names_flexible_line: bool,
    /// The route it names.
    pub(crate) route: Option<Pending>,
    pub(crate) pattern: Option<Pending>,
    /// The day types it names: a range of the builder's `journey_day_types`.
    pub(crate) day_types: (u32, u32),
    /// The availability conditions it names or states: a range of the
    /// builder's `journey_conditions`.
    pub(crate) conditions: (u32, u32),
    /// The calls it states, in calling order: a range of the builder's `raw_calls`.
    pub(crate) calls: (u32, u32),
    /// A template's headway groups: a range of the builder's `raw_headways`.
    pub(crate) headways: (u32, u32),
    /// Passing times as gathered: one per `TimetabledPassingTime`, with the key
    /// of the point it refers to; a range of the builder's `raw_times`.
    pub(crate) raw_times: (u32, u32),
    pub(crate) validity: Validity,
}

/// A passing time as gathered: the key of the point it refers to, and the
/// time. A hundred million of these are gathered over the German delivery,
/// so nothing else rides along; a finding about one is made from its journey.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RawTime {
    /// The point of the pattern it names; none where it names none, and
    /// stands for the point at its position.
    pub(crate) point: Option<Key>,
    pub(crate) time: PassingTime,
}

/// A call as gathered: its identifier, the stop point it is at, what it states
/// about stopping there, and its times.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RawCall {
    pub(crate) id: Sym,
    pub(crate) stop_point: Pending,
    pub(crate) point_use: PointUse,
    pub(crate) time: PassingTime,
}

/// A headway group as gathered: its identifier, its first and last departure
/// in seconds since the operating day's start, and its interval in seconds,
/// each where it states it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct RawHeadway {
    pub(crate) id: Sym,
    pub(crate) first: Option<u32>,
    pub(crate) last: Option<u32>,
    pub(crate) interval: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DayTypeRecord {
    pub(crate) key: Key,
    pub(crate) origin: Sym,
    /// What its properties say of its days.
    pub(crate) properties: Properties,
    /// Its own validity, first and last day, where it states both.
    pub(crate) validity: Option<(i64, i64)>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DayTypeAssignmentRecord {
    pub(crate) day_type: Pending,
    pub(crate) days: AssignedDaysRecord,
    pub(crate) available: bool,
    /// Where it applies among the assignments of its document
    /// ([`assignment_rank`]).
    pub(crate) rank: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AssignedDaysRecord {
    Period(Pending),
    OperatingDay(Pending),
    Date(i64),
}

/// An operating period, or an availability condition, which states its days
/// alike.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PeriodRecord {
    pub(crate) key: Key,
    pub(crate) origin: Sym,
    /// `OperatingPeriod`, `UicOperatingPeriod` or `AvailabilityCondition`.
    pub(crate) element: &'static str,
    pub(crate) from: Option<i64>,
    pub(crate) to: Option<i64>,
    /// The operating days an operating period names as its first and last
    /// day in place of dates (`FromOperatingDayRef`, `ToOperatingDayRef`):
    /// their dates become `from` and `to` once every document is in.
    pub(crate) bounds: [Option<Pending>; 2],
    pub(crate) bits: Option<Box<str>>,
    /// Whether its days are available (`IsAvailable`); an operating period's are.
    pub(crate) available: bool,
}

impl PeriodRecord {
    /// The days of the period, and what was off about it.
    pub(crate) fn days(&self) -> PeriodDays {
        period_days(self.from, self.to, self.bits.as_deref())
    }

    /// The first and last day the period spans, as far as its bits go where
    /// it states no last day; none without a first day.
    pub(crate) fn span(&self) -> Option<(i64, i64)> {
        let from = self.from?;
        let to = self
            .to
            .or_else(|| Some(from + self.bits.as_ref()?.len() as i64 - 1))
            .unwrap_or(from);
        Some((from, to.max(from)))
    }

    /// Whether two definitions state the same days.
    pub(crate) fn says_the_same_as(&self, other: &PeriodRecord) -> bool {
        (self.from, self.to, &self.bits, self.available)
            == (other.from, other.to, &other.bits, other.available)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OperatingDayRecord {
    pub(crate) key: Key,
    pub(crate) origin: Sym,
    pub(crate) day: i64,
}

/// A dated journey that dates a journey rather than stating one, as the
/// Nordic profile writes every `DatedServiceJourney`: the journey it names,
/// the operating day it names, and whether it runs.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DatingRecord {
    pub(crate) journey: Pending,
    pub(crate) day: Pending,
    /// Whether the dated journey runs: not where it is cancelled, or
    /// replaced by another.
    pub(crate) runs: bool,
}

/// A service link with a geometry, as gathered: where it starts, and the two
/// readings of its geometry's first position as latitude and longitude — as
/// its reference system states it, and the other way round.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LinkRecord {
    pub(crate) id: Sym,
    pub(crate) origin: Sym,
    pub(crate) from: Option<Pending>,
    pub(crate) readings: [(f64, f64); 2],
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TransferRecord {
    pub(crate) from: Vec<Pending>,
    pub(crate) to: Vec<Pending>,
    pub(crate) seconds: u32,
    pub(crate) both_ways: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InterchangeRecord {
    pub(crate) from_journey: Pending,
    pub(crate) to_journey: Pending,
    pub(crate) from_stop_point: Pending,
    pub(crate) to_stop_point: Pending,
    pub(crate) stay_seated: bool,
    pub(crate) guaranteed: bool,
    pub(crate) maximum_wait_seconds: Option<u32>,
}

/// The contents an entity gathered once is defined with, for an entity some
/// documents define otherwise: each content by its fingerprint, with the
/// document named first among those stating it.
///
/// What the index keeps of such an entity is the definition of the document
/// named first; every other content is a finding naming the document named
/// first among those stating it. Neither depends on the order the documents
/// were read in.
#[derive(Debug)]
pub(crate) struct Variants<K> {
    pub(crate) of: FxHashMap<K, Vec<(u64, Sym)>>,
}

impl<K> Default for Variants<K> {
    fn default() -> Self {
        Variants {
            of: FxHashMap::default(),
        }
    }
}

impl<K: Hash + Eq> Variants<K> {
    /// Notes that `entity`, held as `held`, is defined as `other` too: a
    /// content and the document stating it, each.
    pub(crate) fn note(
        &mut self,
        strings: &Strings,
        entity: K,
        held: (u64, Sym),
        other: (u64, Sym),
    ) {
        let list = match self.of.entry(entity) {
            Entry::Occupied(list) => list.into_mut(),
            Entry::Vacant(_) if held.0 == other.0 => return,
            Entry::Vacant(slot) => slot.insert(Vec::with_capacity(2)),
        };
        for (content, document) in [held, other] {
            add_content(strings, list, content, document);
        }
    }

    /// Takes in what `entity` was noted with elsewhere.
    pub(crate) fn absorb(&mut self, strings: &Strings, entity: K, contents: Vec<(u64, Sym)>) {
        let list = self.of.entry(entity).or_default();
        for (content, document) in contents {
            add_content(strings, list, content, document);
        }
    }
}

/// Adds a content stated by `document` to `list`, keeping for each content the
/// document named first.
fn add_content(strings: &Strings, list: &mut Vec<(u64, Sym)>, content: u64, document: Sym) {
    match list.iter_mut().find(|(known, _)| *known == content) {
        Some((_, first)) => {
            if precedes(strings, document, *first) {
                *first = document;
            }
        }
        None => list.push((content, document)),
    }
}

/// Whether the definition from document `a` takes precedence over the one
/// from document `b`: of the definitions that several documents repeat, the
/// one from the document whose name sorts first is kept, so that where the
/// documents contradict each other the index does not depend on the order
/// they were read in.
pub(crate) fn precedes(strings: &Strings, a: Sym, b: Sym) -> bool {
    a != b && strings.resolve(a) < strings.resolve(b)
}

/// Whether version `a` is higher than `b`: numerically when both are numbers,
/// else as text; a version beats none.
pub(crate) fn version_is_higher(strings: &Strings, a: Option<Sym>, b: Option<Sym>) -> bool {
    match (a, b) {
        (None, _) => false,
        (Some(_), None) => true,
        (Some(a), Some(b)) => {
            let (a, b) = (strings.resolve(a), strings.resolve(b));
            match (a.parse::<u64>(), b.parse::<u64>()) {
                (Ok(a), Ok(b)) => a > b,
                _ => a > b,
            }
        }
    }
}

/// Builds an [`Index`] from documents, one at a time.
///
/// What a document defines is gathered as it comes. A stop place, quay, stop
/// point or level is gathered once by identifier and version — the German
/// delivery repeats its shared stops in every line file that serves them —:
/// where the repetitions contradict each other, the definition from the
/// document whose name sorts first is kept, whatever order the documents come
/// in, and every other content is a finding; a stop place's quays are the
/// quays of all its definitions. Everything else — routes, lines, journey
/// patterns, journeys and their calendar — is the defining document's own,
/// and a reference resolves in its own document first (see
/// [`finish`](Self::finish)).
///
/// References are gathered as they come too and resolved by
/// [`finish`](Self::finish), once every document is in; each that does not
/// resolve is a finding.
#[derive(Debug, Default)]
pub struct IndexBuilder {
    pub(crate) strings: Strings,
    pub(crate) at: Option<i64>,
    /// Whether the index keeps what refers to what ([`Index::referrers`]).
    pub(crate) referrers: bool,
    /// The names of the documents added, in the order they came.
    pub(crate) document_names: Vec<Sym>,
    pub(crate) redefinitions: u64,
    pub(crate) stops: Vec<StopRecord>,
    pub(crate) stop_table: Table,
    pub(crate) stop_variants: Variants<u32>,
    pub(crate) levels: FxHashMap<Key, LevelRecord>,
    pub(crate) level_variants: Variants<Key>,
    pub(crate) stop_points: Vec<StopPointRecord>,
    pub(crate) stop_point_table: Table,
    pub(crate) stop_point_variants: Variants<u32>,
    pub(crate) assignments: Vec<(Pending, Option<Pending>, Option<Pending>)>,
    /// The quays a stop place names by reference among its quays, with the
    /// stop place: the quays defined beside it, as the French profile writes them.
    pub(crate) quay_listings: Vec<(StopId, Pending)>,
    /// The stop place a quay defined on its own names as its site.
    pub(crate) quay_sites: Vec<(StopId, Pending)>,
    pub(crate) routes: Vec<RouteRecord>,
    pub(crate) lines: Vec<LineRecord>,
    pub(crate) patterns: Vec<PatternRecord>,
    pub(crate) pattern_point_refs: Vec<Pending>,
    pub(crate) pattern_point_keys: Vec<Key>,
    pub(crate) pattern_point_uses: Vec<PointUse>,
    pub(crate) journeys: Vec<JourneyRecord>,
    pub(crate) journey_day_types: Vec<Pending>,
    pub(crate) journey_conditions: Vec<Pending>,
    pub(crate) raw_times: Vec<RawTime>,
    pub(crate) raw_calls: Vec<RawCall>,
    pub(crate) raw_headways: Vec<RawHeadway>,
    pub(crate) day_types: Vec<DayTypeRecord>,
    pub(crate) day_type_assignments: Vec<DayTypeAssignmentRecord>,
    pub(crate) periods: Vec<PeriodRecord>,
    pub(crate) conditions: Vec<PeriodRecord>,
    pub(crate) operating_days: Vec<OperatingDayRecord>,
    pub(crate) datings: Vec<DatingRecord>,
    pub(crate) links: Vec<LinkRecord>,
    pub(crate) transfers: Vec<TransferRecord>,
    pub(crate) interchanges: Vec<InterchangeRecord>,
    pub(crate) unresolved: Vec<Unresolved>,
    /// The findings made as the documents come in: positions in a reference
    /// system the crate does not know.
    pub(crate) gathered: Vec<FindingRecord>,
}

impl IndexBuilder {
    /// An empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Resolves references without a version to the version valid on `day`,
    /// rather than to the highest version.
    pub fn at(mut self, day: &Date) -> Self {
        self.at = day.days_since_epoch();
        self
    }

    /// Keeps every reference the index resolves, so that the index answers
    /// what refers to each object of the delivery ([`Index::referrers`]).
    ///
    /// The graph costs memory and time in proportion to the references —
    /// the German delivery's are some thirty million —, so it is built only
    /// when asked for. Builders merged keep it if any of them was asked for it.
    pub fn with_referrers(mut self) -> Self {
        self.referrers = true;
        self
    }

    /// How many distinct identifiers and names the builder has interned.
    pub fn string_count(&self) -> usize {
        self.strings.len()
    }

    /// Adds every entity of `document`; `origin` names it in findings.
    pub fn add(&mut self, document: &Document, origin: &str) {
        let origin = self.begin(origin);
        let mut visit = Visit {
            builder: self,
            origin,
            default_system: None,
        };
        visit.frames(
            document
                .delivery
                .data_objects
                .as_ref()
                .map_or(&[][..], |f| &f.items),
            Validity::ALWAYS,
        );
    }

    /// Adds every entity of the document `xml`, read lean with `options`
    /// (see [`stream::read`]); `origin` names it in findings.
    ///
    /// The index built this way is the index [`add`](Self::add) builds from
    /// the full model of the same document, when the document's containers
    /// stand in schema order and the selection holds service links — whose
    /// geometries are checked ([`Concern::AxisOrderSwapped`](super::Concern))
    /// where they are read, and which [`Selection::routing`](crate::stream::Selection::routing)
    /// does not hold.
    pub fn add_streamed(
        &mut self,
        xml: &str,
        options: &ReadOptions,
        origin: &str,
    ) -> crate::error::Result<()> {
        /// A sink taking nothing: the builder's own walk.
        struct Nothing;
        impl Sink<'_> for Nothing {
            fn entity(&mut self, _: Entity, _: &Scope) -> crate::error::Result<()> {
                Ok(())
            }
        }
        self.add_streamed_with(xml, options, origin, &mut Nothing)
    }

    /// Adds every entity of the document `xml` as [`add_streamed`](Self::add_streamed)
    /// does, and hands each to `sink` once the builder has taken it in — and
    /// every element the walk skips ([`Sink::skipped`]) — so that a caller
    /// reads what the index does not hold in the same walk of the document:
    /// the operators of the lines, say, or the destination displays.
    ///
    /// An error of the sink ends the read and is returned as it is; what the
    /// builder took of the document before stays in.
    pub fn add_streamed_with<'i, S: Sink<'i>>(
        &mut self,
        xml: &'i str,
        options: &ReadOptions,
        origin: &str,
        sink: &mut S,
    ) -> crate::error::Result<()> {
        let origin = self.begin(origin);
        let mut both = Both {
            visit: Visit {
                builder: self,
                origin,
                default_system: None,
            },
            systems: FrameSystems::default(),
            sink,
        };
        stream::read_with(xml, options, &mut both)
    }

    /// Counts a document in and interns its name.
    fn begin(&mut self, origin: &str) -> Sym {
        let origin = self.strings.intern(origin);
        self.document_names.push(origin);
        origin
    }

    fn key(
        &mut self,
        id: &Option<crate::model::Id>,
        version: &Option<crate::model::Version>,
    ) -> Key {
        Key {
            id: self
                .strings
                .intern(id.as_ref().map_or("", |id| id.as_str())),
            version: version.as_ref().map(|v| self.strings.intern(v.as_str())),
        }
    }

    fn pending(
        &mut self,
        reference: &Ref,
        element: &'static str,
        referrer: Referrer,
        origin: Sym,
    ) -> Pending {
        Pending {
            key: Key {
                id: self
                    .strings
                    .intern(reference.r#ref.as_ref().map_or("", |id| id.as_str())),
                version: reference
                    .version
                    .as_ref()
                    .map(|v| self.strings.intern(v.as_str())),
            },
            version_ref: reference
                .version_ref
                .as_ref()
                .map(|v| self.strings.intern(v)),
            element,
            referrer,
            origin,
        }
    }

    /// Resolves every reference and builds the index.
    pub fn finish(self) -> Index {
        super::finish::finish(self)
    }
}

/// The visit of one document's frames.
struct Visit<'a> {
    builder: &'a mut IndexBuilder,
    origin: Sym,
    /// The reference system the frames being visited name as their default
    /// (`DefaultLocationSystem`).
    default_system: Option<Box<str>>,
}

/// The sink of [`IndexBuilder::add_streamed_with`]: each entity to the visit,
/// then to the caller's sink; every element skipped to the caller's sink,
/// the frame defaults read by the visit as well.
struct Both<'a, 's, S> {
    visit: Visit<'a>,
    systems: FrameSystems,
    sink: &'s mut S,
}

impl<'i, S: Sink<'i>> Sink<'i> for Both<'_, '_, S> {
    fn entity(&mut self, entity: Entity, scope: &Scope) -> crate::error::Result<()> {
        let system = self.systems.of(scope);
        if self.visit.default_system.as_deref() != system {
            self.visit.default_system = system.map(Box::from);
        }
        self.visit.entity(&entity, Validity::within(scope));
        self.sink.entity(entity, scope)
    }

    fn skipped(
        &mut self,
        cursor: &mut Cursor<'i>,
        element: &Child<'i>,
        place: Skipped<'i>,
        scope: &Scope,
    ) -> crate::error::Result<()> {
        #[cfg(feature = "crs")]
        if matches!(place, Skipped::Frame(_)) && element.netex && element.local == "FrameDefaults" {
            self.systems.note(cursor, element, scope)?;
        }
        self.sink.skipped(cursor, element, place, scope)
    }
}

impl Visit<'_> {
    fn frames(&mut self, frames: &[Frame], inherited: Validity) {
        for frame in frames {
            let inherited_system = self.default_system.clone();
            if let Some(system) =
                frame_defaults(frame).and_then(|d| d.default_location_system.as_ref())
            {
                self.default_system = Some(system.clone());
            }
            match frame {
                Frame::Composite(composite) => {
                    let validity = Validity::of(&composite.valid_between, inherited);
                    self.conditions(composite.validity_conditions.as_ref());
                    if let Some(inner) = &composite.frames {
                        self.frames(&inner.items, validity);
                    }
                }
                Frame::Site(site) => {
                    let validity = Validity::of(&site.valid_between, inherited);
                    self.conditions(site.validity_conditions.as_ref());
                    for item in site.stop_places.iter().flat_map(|s| &s.items) {
                        if let crate::model::StopPlaceItem::StopPlace(place) = item {
                            self.stop_place(place, validity);
                        }
                    }
                }
                // The containers are visited in schema order, which is the
                // order the lean mode meets them in a document, so that both
                // paths build the same index.
                Frame::Service(service) => {
                    let validity = Validity::of(&service.valid_between, inherited);
                    self.conditions(service.validity_conditions.as_ref());
                    for item in service.routes.iter().flat_map(|r| &r.items) {
                        if let crate::model::RouteItem::Route(route) = item {
                            self.route(route, validity);
                        }
                    }
                    for item in service.lines.iter().flat_map(|l| &l.items) {
                        match item {
                            crate::model::LineItem::Line(line) => self.line(line, false, validity),
                            crate::model::LineItem::FlexibleLine(line) => {
                                self.line(line, true, validity)
                            }
                            crate::model::LineItem::Unprocessed(_) => {}
                        }
                    }
                    for item in service.scheduled_stop_points.iter().flat_map(|s| &s.items) {
                        if let crate::model::ScheduledStopPointItem::ScheduledStopPoint(point) =
                            item
                        {
                            self.stop_point(point, validity);
                        }
                    }
                    for item in service.service_links.iter().flat_map(|l| &l.items) {
                        if let crate::model::ServiceLinkItem::ServiceLink(link) = item {
                            self.service_link(link);
                        }
                    }
                    for item in service.connections.iter().flat_map(|c| &c.items) {
                        if let crate::model::ConnectionItem::SiteConnection(connection) = item {
                            self.connection(connection);
                        }
                    }
                    for item in service.stop_assignments.iter().flat_map(|s| &s.items) {
                        if let crate::model::StopAssignmentItem::PassengerStopAssignment(
                            assignment,
                        ) = item
                        {
                            self.assignment(assignment);
                        }
                    }
                    for item in service.journey_patterns.iter().flat_map(|j| &j.items) {
                        match item {
                            JourneyPatternItem::ServiceJourneyPattern(pattern)
                            | JourneyPatternItem::JourneyPattern(pattern) => {
                                self.pattern(pattern, validity)
                            }
                            JourneyPatternItem::Unprocessed(_) => {}
                        }
                    }
                }
                Frame::ServiceCalendar(calendar) => {
                    self.conditions(calendar.validity_conditions.as_ref());
                    if let Some(inner) = &calendar.service_calendar {
                        self.calendar_parts(
                            inner.day_types.as_ref(),
                            inner.operating_days.as_ref(),
                            inner.operating_periods.as_ref(),
                            inner.day_type_assignments.as_ref(),
                        );
                    }
                    self.calendar_parts(
                        calendar.day_types.as_ref(),
                        calendar.operating_days.as_ref(),
                        calendar.operating_periods.as_ref(),
                        calendar.day_type_assignments.as_ref(),
                    );
                }
                Frame::Timetable(timetable) => {
                    let validity = Validity::of(&timetable.valid_between, inherited);
                    self.conditions(timetable.validity_conditions.as_ref());
                    for item in timetable.vehicle_journeys.iter().flat_map(|v| &v.items) {
                        match item {
                            VehicleJourney::ServiceJourney(journey)
                            | VehicleJourney::TemplateServiceJourney(journey) => {
                                self.journey(journey, validity)
                            }
                            VehicleJourney::DatedServiceJourney(journey) => {
                                self.dated_journey(journey, validity)
                            }
                            VehicleJourney::Unprocessed(_) => {}
                        }
                    }
                    for item in timetable.journey_interchanges.iter().flat_map(|j| &j.items) {
                        if let crate::model::JourneyInterchangeItem::ServiceJourneyInterchange(
                            interchange,
                        ) = item
                        {
                            self.interchange(interchange);
                        }
                    }
                }
                Frame::Resource(resource) => {
                    self.conditions(resource.validity_conditions.as_ref());
                }
                // The members in document order, which is the order the lean
                // mode meets them in.
                Frame::General(general) => {
                    let validity = Validity::of(&general.valid_between, inherited);
                    self.conditions(general.validity_conditions.as_ref());
                    for member in general.members.iter().flat_map(|m| &m.items) {
                        self.member(member, validity);
                    }
                }
                Frame::Unprocessed(_) => {}
            }
            self.default_system = inherited_system;
        }
    }

    /// Latitude and longitude of `location`, the location of the entity
    /// `element` `id`: a finding where it is written in a reference system the
    /// crate does not know.
    fn position(
        &mut self,
        location: Option<&crate::model::Location>,
        element: &'static str,
        id: Sym,
    ) -> (Option<f64>, Option<f64>) {
        match positions::position(location, self.default_system.as_deref()) {
            Ok(position) => position,
            Err(system) => {
                self.unknown_system(element, id, &system);
                (None, None)
            }
        }
    }

    /// The finding of a position in `system`, which the crate does not know.
    fn unknown_system(&mut self, element: &'static str, id: Sym, system: &str) {
        let system = self.builder.strings.intern(system);
        self.builder.gathered.push(FindingRecord {
            concern: Concern::UnknownReferenceSystem,
            element,
            document: self.origin,
            entity: id,
            about: Some(system),
        });
    }

    /// One entity of the lean stream, valid within `validity`.
    fn entity(&mut self, entity: &Entity, validity: Validity) {
        match entity {
            Entity::StopPlace(place) => self.stop_place(place, validity),
            Entity::Quay(quay) => self.quay_on_its_own(quay, validity),
            Entity::Route(route) => self.route(route, validity),
            Entity::Line(line) => self.line(line, false, validity),
            Entity::FlexibleLine(line) => self.line(line, true, validity),
            Entity::ScheduledStopPoint(point) => self.stop_point(point, validity),
            Entity::ServiceLink(link) => self.service_link(link),
            Entity::SiteConnection(connection) => self.connection(connection),
            Entity::PassengerStopAssignment(assignment) => self.assignment(assignment),
            Entity::ServiceJourneyPattern(pattern) | Entity::JourneyPattern(pattern) => {
                self.pattern(pattern, validity)
            }
            Entity::DayType(day_type) => self.day_type(day_type),
            Entity::OperatingDay(day) => self.operating_day(day),
            Entity::OperatingPeriod(period) => self.period(period, "OperatingPeriod"),
            Entity::UicOperatingPeriod(period) => self.period(period, "UicOperatingPeriod"),
            Entity::DayTypeAssignment(assignment) => self.day_type_assignment(assignment),
            Entity::AvailabilityCondition(condition) => {
                self.condition(condition);
            }
            Entity::ServiceJourney(journey) | Entity::TemplateServiceJourney(journey) => {
                self.journey(journey, validity)
            }
            Entity::DatedServiceJourney(journey) => self.dated_journey(journey, validity),
            Entity::ServiceJourneyInterchange(interchange) => self.interchange(interchange),
            _ => {}
        }
    }

    /// One member of a general frame of the full model, valid within
    /// `validity`: taken as [`entity`](Self::entity) takes the same entity of
    /// the lean stream.
    fn member(&mut self, member: &GeneralFrameMember, validity: Validity) {
        match member {
            GeneralFrameMember::StopPlace(place) => self.stop_place(place, validity),
            GeneralFrameMember::Quay(quay) => self.quay_on_its_own(quay, validity),
            GeneralFrameMember::Route(route) => self.route(route, validity),
            GeneralFrameMember::Line(line) => self.line(line, false, validity),
            GeneralFrameMember::FlexibleLine(line) => self.line(line, true, validity),
            GeneralFrameMember::ScheduledStopPoint(point) => self.stop_point(point, validity),
            GeneralFrameMember::ServiceLink(link) => self.service_link(link),
            GeneralFrameMember::SiteConnection(connection) => self.connection(connection),
            GeneralFrameMember::PassengerStopAssignment(assignment) => self.assignment(assignment),
            GeneralFrameMember::ServiceJourneyPattern(pattern)
            | GeneralFrameMember::JourneyPattern(pattern) => self.pattern(pattern, validity),
            GeneralFrameMember::DayType(day_type) => self.day_type(day_type),
            GeneralFrameMember::OperatingDay(day) => self.operating_day(day),
            GeneralFrameMember::OperatingPeriod(period) => self.period(period, "OperatingPeriod"),
            GeneralFrameMember::UicOperatingPeriod(period) => {
                self.period(period, "UicOperatingPeriod")
            }
            GeneralFrameMember::DayTypeAssignment(assignment) => {
                self.day_type_assignment(assignment)
            }
            GeneralFrameMember::AvailabilityCondition(condition) => {
                self.condition(condition);
            }
            GeneralFrameMember::ServiceJourney(journey)
            | GeneralFrameMember::TemplateServiceJourney(journey) => {
                self.journey(journey, validity)
            }
            GeneralFrameMember::DatedServiceJourney(journey) => {
                self.dated_journey(journey, validity)
            }
            GeneralFrameMember::ServiceJourneyInterchange(interchange) => {
                self.interchange(interchange)
            }
            _ => {}
        }
    }

    fn referrer(&mut self, element: &'static str, id: &Option<crate::model::Id>) -> Referrer {
        Referrer {
            element,
            id: self
                .builder
                .strings
                .intern(id.as_ref().map_or("", |id| id.as_str())),
        }
    }

    /// A stop place with its levels and quays.
    ///
    /// A second definition of a stop place is left out, but not its quays:
    /// line files that serve a stop place at different quays each write the
    /// place with the quays they serve, so a quay only a later definition
    /// names is a quay of the place that was kept. Whichever definition comes
    /// first, the index holds every quay of the place.
    fn stop_place(&mut self, place: &StopPlace, inherited: Validity) {
        let validity = Validity::of(&place.valid_between, inherited);
        let key = self.builder.key(&place.id, &place.version);
        let record = self.stop_place_record(place, key, validity);
        let parent = self.gather_stop(record);
        self.parts_of_stop_place(place, parent, validity);
    }

    /// Gathers a stop place or quay: a new one appended, a repeated one put in
    /// the place of the definition it takes precedence over, its content noted
    /// where it differs. The stop as the index numbers it.
    fn gather_stop(&mut self, record: StopRecord) -> StopId {
        let builder = &mut *self.builder;
        let index = u32::try_from(builder.stops.len()).expect("fewer than 2^32 stops");
        match builder
            .stop_table
            .insert_or_kept(&builder.strings, record.key, index)
        {
            Ok(()) => {
                builder.stops.push(record);
                StopId(index)
            }
            Err(kept) => {
                builder.redefinitions += 1;
                let held = &builder.stops[kept as usize];
                builder.stop_variants.note(
                    &builder.strings,
                    kept,
                    (held.content, held.origin),
                    (record.content, record.origin),
                );
                if precedes(&builder.strings, record.origin, held.origin) {
                    builder.stops[kept as usize] = record;
                }
                StopId(kept)
            }
        }
    }

    fn stop_place_record(&mut self, place: &StopPlace, key: Key, validity: Validity) -> StopRecord {
        let location = place.centroid.as_ref().and_then(|c| c.location.as_ref());
        let dhid = place.key_list.as_ref().and_then(|k| k.get("GlobalID"));
        let name = place.name.as_ref().map_or("", |n| n.as_str());
        let (latitude, longitude) = self.position(location, "StopPlace", key.id);
        StopRecord {
            key,
            origin: self.origin,
            content: fingerprint((
                name,
                dhid,
                latitude.map(f64::to_bits),
                longitude.map(f64::to_bits),
            )),
            dhid: dhid.map(|d| self.builder.strings.intern(d)),
            name: self.builder.strings.intern(name),
            kind: StopKind::StopPlace,
            latitude,
            longitude,
            parent: None,
            level_ref: None,
            level: None,
            validity,
        }
    }

    /// The levels and quays of a stop place, the quays as quays of `parent`.
    fn parts_of_stop_place(&mut self, place: &StopPlace, parent: StopId, validity: Validity) {
        for level in place.levels.iter().flat_map(|l| &l.levels) {
            let key = self.builder.key(&level.id, &level.version);
            let name = level
                .short_name
                .as_ref()
                .map(|n| n.as_str())
                .or(level.public_code.as_deref())
                .or(level.name.as_ref().map(|n| n.as_str()))
                .unwrap_or("");
            let content = level_fingerprint(name);
            let level = LevelRecord {
                name: self.builder.strings.intern(name),
                origin: self.origin,
            };
            let builder = &mut *self.builder;
            match builder.levels.entry(key) {
                Entry::Vacant(slot) => {
                    slot.insert(level);
                }
                Entry::Occupied(mut kept) => {
                    let held = *kept.get();
                    let held_content = level_fingerprint(builder.strings.resolve(held.name));
                    builder.level_variants.note(
                        &builder.strings,
                        key,
                        (held_content, held.origin),
                        (content, level.origin),
                    );
                    if precedes(&builder.strings, level.origin, held.origin) {
                        kept.insert(level);
                    }
                }
            }
        }
        for item in place.quays.iter().flat_map(|q| &q.items) {
            match item {
                QuayItem::Quay(quay) => {
                    let place_id = place.id.as_ref().map(|id| id.as_str());
                    self.quay(quay, Some(parent), place_id, validity);
                }
                QuayItem::QuayRef(reference) => {
                    let referrer = self.referrer("StopPlace", &place.id);
                    let pending = self
                        .builder
                        .pending(reference, "QuayRef", referrer, self.origin);
                    self.builder.quay_listings.push((parent, pending));
                }
                QuayItem::Unprocessed(_) => {}
            }
        }
    }

    /// A quay defined beside its stop place: its stop place is found once
    /// every document is in, as the one naming it among its quays, else the
    /// one its `SiteRef` names.
    fn quay_on_its_own(&mut self, quay: &Quay, inherited: Validity) {
        let site = quay.site_ref.as_ref();
        let site_id = site.and_then(|r| r.r#ref.as_ref()).map(|id| id.as_str());
        let stop = self.quay(quay, None, site_id, inherited);
        if let Some(site) = site {
            let referrer = self.referrer("Quay", &quay.id);
            let pending = self.builder.pending(site, "SiteRef", referrer, self.origin);
            self.builder.quay_sites.push((stop, pending));
        }
    }

    /// A quay of the stop place `place_id`, `parent` where the stop place is
    /// known as it is read; the quay as the index numbers it.
    fn quay(
        &mut self,
        quay: &Quay,
        parent: Option<StopId>,
        place_id: Option<&str>,
        inherited: Validity,
    ) -> StopId {
        let validity = Validity::of(&quay.valid_between, inherited);
        let key = self.builder.key(&quay.id, &quay.version);
        let location = quay.centroid.as_ref().and_then(|c| c.location.as_ref());
        let (latitude, longitude) = self.position(location, "Quay", key.id);
        let referrer = self.referrer("Quay", &quay.id);
        let origin = self.origin;
        let level_ref = quay
            .level_ref
            .as_ref()
            .map(|r| self.builder.pending(r, "LevelRef", referrer, origin));
        let dhid = quay.key_list.as_ref().and_then(|k| k.get("GlobalID"));
        let name = quay.name.as_ref().map_or("", |n| n.as_str());
        let record = StopRecord {
            key,
            origin,
            content: fingerprint((
                name,
                dhid,
                latitude.map(f64::to_bits),
                longitude.map(f64::to_bits),
                place_id,
                quay.level_ref
                    .as_ref()
                    .and_then(|r| r.r#ref.as_ref())
                    .map(|id| id.as_str()),
            )),
            dhid: dhid.map(|d| self.builder.strings.intern(d)),
            name: self.builder.strings.intern(name),
            kind: StopKind::Quay,
            latitude,
            longitude,
            parent,
            level_ref,
            level: None,
            validity,
        };
        self.gather_stop(record)
    }

    fn route(&mut self, route: &crate::model::Route, inherited: Validity) {
        let validity = Validity::of(&route.valid_between, inherited);
        let key = self.builder.key(&route.id, &route.version);
        let referrer = self.referrer("Route", &route.id);
        let line = self.line_ref(route.line_ref.as_ref(), referrer);
        self.builder.routes.push(RouteRecord {
            key,
            origin: self.origin,
            line,
            validity,
        });
    }

    /// The line `line_ref` names, a line or a flexible line, as a reference
    /// of `referrer` to resolve.
    fn line_ref(&mut self, line_ref: Option<&LineRef>, referrer: Referrer) -> Option<Pending> {
        let (r, element) = match line_ref? {
            LineRef::Line(r) => (r, "LineRef"),
            LineRef::FlexibleLine(r) => (r, "FlexibleLineRef"),
        };
        Some(self.builder.pending(r, element, referrer, self.origin))
    }

    /// A line, or a flexible line: a line served on demand.
    fn line(&mut self, line: &crate::model::Line, flexible: bool, inherited: Validity) {
        let validity = Validity::of(&line.valid_between, inherited);
        let key = self.builder.key(&line.id, &line.version);
        let record = LineRecord {
            key,
            origin: self.origin,
            name: self
                .builder
                .strings
                .intern(line.name.as_ref().map_or("", |n| n.as_str())),
            public_code: line
                .public_code
                .as_deref()
                .map(|c| self.builder.strings.intern(c)),
            mode: line
                .transport_mode
                .as_ref()
                .map(|m| self.builder.strings.intern(m.as_str())),
            flexible,
            validity,
            span: Span::of(&line.valid_between),
        };
        self.builder.lines.push(record);
    }

    fn stop_point(&mut self, point: &crate::model::ScheduledStopPoint, inherited: Validity) {
        let validity = Validity::of(&point.valid_between, inherited);
        let key = self.builder.key(&point.id, &point.version);
        let name = point.name.as_ref().map_or("", |n| n.as_str());
        let (latitude, longitude) =
            self.position(point.location.as_ref(), "ScheduledStopPoint", key.id);
        let record = StopPointRecord {
            key,
            origin: self.origin,
            content: fingerprint((
                name,
                latitude.map(f64::to_bits),
                longitude.map(f64::to_bits),
            )),
            name: self.builder.strings.intern(name),
            latitude,
            longitude,
            stop: None,
            validity,
        };
        let builder = &mut *self.builder;
        let index = u32::try_from(builder.stop_points.len()).expect("fewer than 2^32 stop points");
        match builder
            .stop_point_table
            .insert_or_kept(&builder.strings, key, index)
        {
            Ok(()) => builder.stop_points.push(record),
            Err(kept) => {
                builder.redefinitions += 1;
                let held = &builder.stop_points[kept as usize];
                builder.stop_point_variants.note(
                    &builder.strings,
                    kept,
                    (held.content, held.origin),
                    (record.content, record.origin),
                );
                if precedes(&builder.strings, record.origin, held.origin) {
                    builder.stop_points[kept as usize] = record;
                }
            }
        }
    }

    fn assignment(&mut self, assignment: &crate::model::PassengerStopAssignment) {
        let Some(point) = &assignment.scheduled_stop_point_ref else {
            return;
        };
        let referrer = self.referrer("PassengerStopAssignment", &assignment.id);
        let origin = self.origin;
        let point = self
            .builder
            .pending(point, "ScheduledStopPointRef", referrer, origin);
        let quay = assignment
            .quay_ref
            .as_ref()
            .map(|r| self.builder.pending(r, "QuayRef", referrer, origin));
        let place = assignment
            .stop_place_ref
            .as_ref()
            .map(|r| self.builder.pending(r, "StopPlaceRef", referrer, origin));
        self.builder.assignments.push((point, quay, place));
    }

    fn pattern(&mut self, pattern: &ServiceJourneyPattern, inherited: Validity) {
        let validity = Validity::of(&pattern.valid_between, inherited);
        let key = self.builder.key(&pattern.id, &pattern.version);
        let referrer = self.referrer("ServiceJourneyPattern", &pattern.id);
        let origin = self.origin;
        let (line, route) = match &pattern.route {
            Some(RouteRefOrView::RouteView(view)) => (
                self.line_ref(view.line_ref.as_ref(), referrer),
                view.route_ref.as_ref(),
            ),
            Some(RouteRefOrView::RouteRef(r)) => (None, Some(r)),
            None => (None, None),
        };
        let route = route.map(|r| self.builder.pending(r, "RouteRef", referrer, origin));
        let start = u32::try_from(self.builder.pattern_point_refs.len())
            .expect("fewer than 2^32 pattern points");
        for item in pattern.points_in_sequence.iter().flat_map(|p| &p.items) {
            if let crate::model::PointInSequenceItem::StopPointInJourneyPattern(point) = item {
                let point_key = self.builder.key(&point.id, &point.version);
                let stop_point = match &point.scheduled_stop_point_ref {
                    Some(
                        StopPointRef::ScheduledStopPoint(r)
                        | StopPointRef::FareScheduledStopPoint(r),
                    ) => self
                        .builder
                        .pending(r, "ScheduledStopPointRef", referrer, origin),
                    None => continue,
                };
                self.builder.pattern_point_refs.push(stop_point);
                self.builder.pattern_point_keys.push(point_key);
                self.builder.pattern_point_uses.push(PointUse::of(
                    point.stop_use.as_ref(),
                    point.for_boarding,
                    point.for_alighting,
                ));
            }
        }
        let end = u32::try_from(self.builder.pattern_point_refs.len())
            .expect("fewer than 2^32 pattern points");
        self.builder.patterns.push(PatternRecord {
            key,
            origin,
            line,
            route,
            points: (start, end),
            validity,
        });
    }

    fn journey(&mut self, journey: &ServiceJourney, inherited: Validity) {
        let validity = Validity::of(&journey.valid_between, inherited);
        let key = self.builder.key(&journey.id, &journey.version);
        let referrer = self.referrer("ServiceJourney", &journey.id);
        let origin = self.origin;
        let line = self.line_ref(journey.line_ref.as_ref(), referrer);
        let names_flexible_line = matches!(journey.line_ref, Some(LineRef::FlexibleLine(_)));
        let route = journey
            .route_ref
            .as_ref()
            .map(|r| self.builder.pending(r, "RouteRef", referrer, origin));
        let pattern = match &journey.journey_pattern_ref {
            Some(
                JourneyPatternRef::JourneyPattern(r)
                | JourneyPatternRef::ServiceJourneyPattern(r)
                | JourneyPatternRef::DeadRunJourneyPattern(r)
                | JourneyPatternRef::ServicePattern(r),
            ) => Some(
                self.builder
                    .pending(r, "JourneyPatternRef", referrer, origin),
            ),
            None => None,
        };
        let day_types_start = u32::try_from(self.builder.journey_day_types.len())
            .expect("fewer than 2^32 day type refs");
        for item in journey.day_types.iter().flat_map(|d| &d.items) {
            if let crate::model::DayTypeRefItem::DayTypeRef(r) = item {
                let pending = self.builder.pending(r, "DayTypeRef", referrer, origin);
                self.builder.journey_day_types.push(pending);
            }
        }
        let day_types_end = u32::try_from(self.builder.journey_day_types.len())
            .expect("fewer than 2^32 day type refs");
        let conditions = self.journey_conditions(journey, referrer);
        let calls = self.calls(journey, referrer);
        let headways = self.headways(journey);
        let times_start =
            u32::try_from(self.builder.raw_times.len()).expect("fewer than 2^32 passing times");
        for passing in journey.passing_times.iter().flat_map(|p| &p.passing_times) {
            let point = match &passing.point_ref {
                Some(
                    PointInPatternRef::StopPointInJourneyPattern(r)
                    | PointInPatternRef::TimingPointInJourneyPattern(r)
                    | PointInPatternRef::PointInJourneyPattern(r)
                    | PointInPatternRef::FarePointInPattern(r)
                    | PointInPatternRef::PointInSingleJourneyPath(r),
                ) => Some(Key {
                    id: self
                        .builder
                        .strings
                        .intern(r.r#ref.as_ref().map_or("", |id| id.as_str())),
                    version: r
                        .version
                        .as_ref()
                        .map(|v| self.builder.strings.intern(v.as_str())),
                }),
                None => None,
            };
            let time = PassingTime {
                arrival: seconds(&passing.arrival_time, passing.arrival_day_offset),
                departure: seconds(&passing.departure_time, passing.departure_day_offset),
            };
            self.builder.raw_times.push(RawTime { point, time });
        }
        let times_end =
            u32::try_from(self.builder.raw_times.len()).expect("fewer than 2^32 passing times");
        self.builder.journeys.push(JourneyRecord {
            key,
            origin,
            line,
            names_flexible_line,
            route,
            pattern,
            day_types: (day_types_start, day_types_end),
            conditions,
            calls,
            headways,
            raw_times: (times_start, times_end),
            validity,
        });
    }

    /// A dated journey. One naming a service journey and an operating day
    /// and stating no pattern, passing times or calls of its own — as the
    /// Nordic profile writes every one — dates the journey it names: the
    /// journey runs on that day, unless the dated journey is cancelled or
    /// replaced (`ServiceAlteration`), when it gives no day. Any other is a
    /// journey of its own.
    fn dated_journey(&mut self, journey: &ServiceJourney, inherited: Validity) {
        let states_its_own = journey.journey_pattern_ref.is_some()
            || journey.passing_times.is_some()
            || journey.calls.is_some();
        let (
            false,
            Some(JourneyRef::ServiceJourney(dated)),
            Some(OperatingDayRef::OperatingDayRef(day)),
        ) = (
            states_its_own,
            &journey.journey_ref,
            &journey.operating_day_ref,
        )
        else {
            self.journey(journey, inherited);
            return;
        };
        let referrer = self.referrer("DatedServiceJourney", &journey.id);
        let origin = self.origin;
        let dating = DatingRecord {
            journey: self
                .builder
                .pending(dated, "ServiceJourneyRef", referrer, origin),
            day: self
                .builder
                .pending(day, "OperatingDayRef", referrer, origin),
            runs: !matches!(
                journey.service_alteration.as_deref(),
                Some("cancellation" | "replaced")
            ),
        };
        self.builder.datings.push(dating);
    }

    /// The availability conditions a journey names, and those it states in
    /// place — each a condition of the document as well, named from the
    /// journey: a range of the builder's `journey_conditions`.
    fn journey_conditions(&mut self, journey: &ServiceJourney, referrer: Referrer) -> (u32, u32) {
        let origin = self.origin;
        let start = len_of(&self.builder.journey_conditions);
        for item in journey.validity_conditions.iter().flat_map(|c| &c.items) {
            let pending = match item {
                ValidityConditionItem::AvailabilityConditionRef(r) => {
                    self.builder
                        .pending(r, "AvailabilityConditionRef", referrer, origin)
                }
                ValidityConditionItem::AvailabilityCondition(condition) => Pending {
                    key: self.condition(condition),
                    version_ref: None,
                    element: "AvailabilityCondition",
                    referrer,
                    origin,
                },
                ValidityConditionItem::ValidBetween(_) | ValidityConditionItem::Unprocessed(_) => {
                    continue
                }
            };
            self.builder.journey_conditions.push(pending);
        }
        (start, len_of(&self.builder.journey_conditions))
    }

    /// The calls a journey states, in the order of their `order` — the order
    /// written where they state none: a range of the builder's `raw_calls`.
    fn calls(&mut self, journey: &ServiceJourney, referrer: Referrer) -> (u32, u32) {
        let origin = self.origin;
        let start = len_of(&self.builder.raw_calls);
        let mut calls: Vec<(u32, RawCall)> = Vec::new();
        for item in journey.calls.iter().flat_map(|c| &c.items) {
            let crate::model::CallItem::Call(call) = item else {
                continue;
            };
            let stop_point = match &call.scheduled_stop_point_ref {
                Some(
                    StopPointRef::ScheduledStopPoint(r) | StopPointRef::FareScheduledStopPoint(r),
                ) => self
                    .builder
                    .pending(r, "ScheduledStopPointRef", referrer, origin),
                None => continue,
            };
            let (arrival, departure) = (call.arrival.as_ref(), call.departure.as_ref());
            let raw = RawCall {
                id: self
                    .builder
                    .strings
                    .intern(call.id.as_ref().map_or("", |id| id.as_str())),
                stop_point,
                point_use: PointUse::of(
                    call.stop_use.as_ref(),
                    departure.and_then(|d| d.for_boarding),
                    arrival.and_then(|a| a.for_alighting),
                ),
                time: PassingTime {
                    arrival: arrival.and_then(|a| seconds(&a.time, a.day_offset)),
                    departure: departure.and_then(|d| seconds(&d.time, d.day_offset)),
                },
            };
            let order = call
                .order
                .as_ref()
                .and_then(|o| o.value())
                .unwrap_or(u32::MAX);
            calls.push((order, raw));
        }
        calls.sort_by_key(|(order, _)| *order);
        self.builder
            .raw_calls
            .extend(calls.into_iter().map(|(_, call)| call));
        (start, len_of(&self.builder.raw_calls))
    }

    /// A template journey's headway groups: a range of the builder's `raw_headways`.
    fn headways(&mut self, journey: &ServiceJourney) -> (u32, u32) {
        let start = len_of(&self.builder.raw_headways);
        for item in journey.frequency_groups.iter().flat_map(|g| &g.items) {
            let crate::model::FrequencyGroupItem::HeadwayJourneyGroup(group) = item else {
                continue;
            };
            let raw = RawHeadway {
                id: self
                    .builder
                    .strings
                    .intern(group.id.as_ref().map_or("", |id| id.as_str())),
                first: seconds(&group.first_departure_time, group.first_day_offset),
                last: seconds(&group.last_departure_time, group.last_day_offset),
                interval: group
                    .scheduled_headway_interval
                    .as_ref()
                    .and_then(|d| d.seconds())
                    .and_then(|s| u32::try_from(s).ok()),
            };
            self.builder.raw_headways.push(raw);
        }
        (start, len_of(&self.builder.raw_headways))
    }

    /// The availability conditions a frame defines.
    fn conditions(&mut self, conditions: Option<&crate::model::ValidityConditions>) {
        for item in conditions.iter().flat_map(|c| &c.items) {
            if let ValidityConditionItem::AvailabilityCondition(condition) = item {
                self.condition(condition);
            }
        }
    }

    /// An availability condition of the document; its key.
    fn condition(&mut self, condition: &AvailabilityCondition) -> Key {
        let key = self.builder.key(&condition.id, &condition.version);
        let day = |date: &Option<DateTime>| {
            date.as_ref()
                .and_then(|d| d.date())
                .and_then(|d| d.days_since_epoch())
        };
        self.builder.conditions.push(PeriodRecord {
            key,
            origin: self.origin,
            element: "AvailabilityCondition",
            from: day(&condition.from_date),
            to: day(&condition.to_date),
            bounds: [None, None],
            bits: condition.valid_day_bits.clone(),
            available: condition.is_available.unwrap_or(true),
        });
        key
    }

    /// The containers of a calendar, in schema order.
    fn calendar_parts(
        &mut self,
        day_types: Option<&crate::model::DayTypes>,
        days: Option<&crate::model::OperatingDays>,
        periods: Option<&crate::model::OperatingPeriods>,
        assignments: Option<&crate::model::DayTypeAssignments>,
    ) {
        for item in day_types.iter().flat_map(|d| &d.items) {
            if let crate::model::DayTypeItem::DayType(day_type) = item {
                self.day_type(day_type);
            }
        }
        for item in days.iter().flat_map(|d| &d.items) {
            if let crate::model::OperatingDayItem::OperatingDay(day) = item {
                self.operating_day(day);
            }
        }
        for item in periods.iter().flat_map(|p| &p.items) {
            match item {
                OperatingPeriodItem::OperatingPeriod(period) => {
                    self.period(period, "OperatingPeriod")
                }
                OperatingPeriodItem::UicOperatingPeriod(period) => {
                    self.period(period, "UicOperatingPeriod")
                }
                OperatingPeriodItem::Unprocessed(_) => {}
            }
        }
        for item in assignments.iter().flat_map(|a| &a.items) {
            if let crate::model::DayTypeAssignmentItem::DayTypeAssignment(assignment) = item {
                self.day_type_assignment(assignment);
            }
        }
    }

    fn day_type(&mut self, day_type: &DayType) {
        let key = self.builder.key(&day_type.id, &day_type.version);
        let properties = day_type
            .properties
            .as_ref()
            .map_or(&[][..], |p| &p.properties);
        let span = Span::of(&day_type.valid_between);
        self.builder.day_types.push(DayTypeRecord {
            key,
            origin: self.origin,
            properties: crate::calendar::properties(properties),
            validity: span.from.zip(span.to),
        });
    }

    fn operating_day(&mut self, day: &OperatingDay) {
        let key = self.builder.key(&day.id, &day.version);
        if let Some(calendar_day) = day
            .calendar_date
            .as_ref()
            .and_then(|d| d.days_since_epoch())
        {
            self.builder.operating_days.push(OperatingDayRecord {
                key,
                origin: self.origin,
                day: calendar_day,
            });
        }
    }

    /// An operating period, its first and last day given as dates or as
    /// operating days.
    fn period(&mut self, period: &OperatingPeriod, element: &'static str) {
        let key = self.builder.key(&period.id, &period.version);
        let referrer = self.referrer(element, &period.id);
        let origin = self.origin;
        let date = |date: &DateTime| date.date().and_then(|d| d.days_since_epoch());
        let (from, first) = match &period.start {
            Some(PeriodStart::FromDate(start)) => (date(start), None),
            Some(PeriodStart::FromOperatingDayRef(r)) => (
                None,
                Some(
                    self.builder
                        .pending(r, "FromOperatingDayRef", referrer, origin),
                ),
            ),
            None => (None, None),
        };
        let (to, last) = match &period.end {
            Some(PeriodEnd::ToDate(end)) => (date(end), None),
            Some(PeriodEnd::ToOperatingDayRef(r)) => (
                None,
                Some(
                    self.builder
                        .pending(r, "ToOperatingDayRef", referrer, origin),
                ),
            ),
            None => (None, None),
        };
        self.builder.periods.push(PeriodRecord {
            key,
            origin,
            element,
            from,
            to,
            bounds: [first, last],
            bits: period.valid_day_bits.clone(),
            available: true,
        });
    }

    fn day_type_assignment(&mut self, assignment: &crate::model::DayTypeAssignment) {
        let referrer = self.referrer("DayTypeAssignment", &assignment.id);
        let origin = self.origin;
        let day_type = match &assignment.day_type {
            Some(AssignedDayType::DayTypeRef(r) | AssignedDayType::FareDayTypeRef(r)) => {
                self.builder.pending(r, "DayTypeRef", referrer, origin)
            }
            None => return,
        };
        let days = match &assignment.days {
            Some(AssignedDays::OperatingPeriodRef(r)) => AssignedDaysRecord::Period(
                self.builder
                    .pending(r, "OperatingPeriodRef", referrer, origin),
            ),
            Some(AssignedDays::UicOperatingPeriodRef(r)) => AssignedDaysRecord::Period(
                self.builder
                    .pending(r, "UicOperatingPeriodRef", referrer, origin),
            ),
            Some(AssignedDays::OperatingDayRef(r)) => AssignedDaysRecord::OperatingDay(
                self.builder.pending(r, "OperatingDayRef", referrer, origin),
            ),
            Some(AssignedDays::Date(date)) => match date.days_since_epoch() {
                Some(day) => AssignedDaysRecord::Date(day),
                None => return,
            },
            None => return,
        };
        self.builder
            .day_type_assignments
            .push(DayTypeAssignmentRecord {
                day_type,
                days,
                available: assignment.is_available.unwrap_or(true),
                rank: assignment_rank(assignment.order.as_ref()),
            });
    }

    /// A service link, where it states a geometry: the readings of its first
    /// position, to be judged against where the link starts.
    fn service_link(&mut self, link: &crate::model::ServiceLink) {
        let Some((first, geometry)) = link
            .unprocessed
            .iter()
            .filter(|fragment| &*fragment.name == "LineString")
            .find_map(|fragment| Some((first_position(&fragment.xml)?, &fragment.xml)))
        else {
            return;
        };
        let referrer = self.referrer("ServiceLink", &link.id);
        let readings = match positions::readings(first, geometry, self.default_system.as_deref()) {
            Ok(readings) => readings,
            Err(system) => {
                self.unknown_system("ServiceLink", referrer.id, &system);
                return;
            }
        };
        let origin = self.origin;
        let from = link
            .from_point_ref
            .as_ref()
            .map(|r| self.builder.pending(r, "FromPointRef", referrer, origin));
        self.builder.links.push(LinkRecord {
            id: referrer.id,
            origin,
            from,
            readings,
        });
    }

    fn connection(&mut self, connection: &crate::model::SiteConnection) {
        let Some(seconds) = connection
            .walk_transfer_duration
            .as_ref()
            .or(connection.transfer_duration.as_ref())
            .and_then(|d| d.default_duration.as_ref())
            .and_then(|d| d.seconds())
        else {
            let entity = self.referrer("SiteConnection", &connection.id).id;
            self.builder.gathered.push(FindingRecord {
                concern: Concern::WalkWithoutDuration,
                element: "SiteConnection",
                document: self.origin,
                entity,
                about: None,
            });
            return;
        };
        let (Some(from), Some(to)) = (&connection.from, &connection.to) else {
            return;
        };
        let referrer = self.referrer("SiteConnection", &connection.id);
        let from = self.connection_end(from, referrer);
        let to = self.connection_end(to, referrer);
        if from.is_empty() || to.is_empty() {
            return;
        }
        self.builder.transfers.push(TransferRecord {
            from,
            to,
            seconds: u32::try_from(seconds).unwrap_or(u32::MAX),
            both_ways: connection.both_ways.unwrap_or(false),
        });
    }

    /// The stops an end names, most specific first: the quay, then the stop place.
    fn connection_end(&mut self, end: &SiteConnectionEnd, referrer: Referrer) -> Vec<Pending> {
        let origin = self.origin;
        let mut out = Vec::with_capacity(2);
        if let Some(quay) = &end.quay_ref {
            out.push(self.builder.pending(quay, "QuayRef", referrer, origin));
        }
        if let Some(place) = &end.stop_place_ref {
            out.push(
                self.builder
                    .pending(place, "StopPlaceRef", referrer, origin),
            );
        }
        out
    }

    fn interchange(&mut self, interchange: &crate::model::ServiceJourneyInterchange) {
        let (Some(from_journey), Some(to_journey), Some(from_point), Some(to_point)) = (
            &interchange.from_journey_ref,
            &interchange.to_journey_ref,
            &interchange.from_point_ref,
            &interchange.to_point_ref,
        ) else {
            return;
        };
        let referrer = self.referrer("ServiceJourneyInterchange", &interchange.id);
        let origin = self.origin;
        let record = InterchangeRecord {
            from_journey: self
                .builder
                .pending(from_journey, "FromJourneyRef", referrer, origin),
            to_journey: self
                .builder
                .pending(to_journey, "ToJourneyRef", referrer, origin),
            from_stop_point: self
                .builder
                .pending(from_point, "FromPointRef", referrer, origin),
            to_stop_point: self
                .builder
                .pending(to_point, "ToPointRef", referrer, origin),
            stay_seated: interchange.stay_seated.unwrap_or(false),
            guaranteed: interchange.guaranteed.unwrap_or(false),
            maximum_wait_seconds: interchange
                .maximum_wait_time
                .as_ref()
                .and_then(|d| d.seconds())
                .map(|s| u32::try_from(s).unwrap_or(u32::MAX)),
        };
        self.builder.interchanges.push(record);
    }
}

/// The length of a list the builder ranges over, as a range's end.
fn len_of<T>(list: &[T]) -> u32 {
    u32::try_from(list.len()).expect("fewer than 2^32 entries of a kind")
}

/// A time of day plus a day offset, in seconds since midnight of the operating day.
fn seconds(time: &Option<Time>, day_offset: Option<i64>) -> Option<u32> {
    let seconds = time.as_ref()?.seconds_since_midnight()?;
    let offset = day_offset.unwrap_or(0);
    let total = i64::from(seconds) + offset * 86_400;
    u32::try_from(total).ok()
}

/// The defaults a frame states.
fn frame_defaults(frame: &Frame) -> Option<&crate::model::FrameDefaults> {
    match frame {
        Frame::Composite(frame) => frame.frame_defaults.as_ref(),
        Frame::Resource(frame) => frame.frame_defaults.as_ref(),
        Frame::Site(frame) => frame.frame_defaults.as_ref(),
        Frame::Service(frame) => frame.frame_defaults.as_ref(),
        Frame::ServiceCalendar(frame) => frame.frame_defaults.as_ref(),
        Frame::Timetable(frame) => frame.frame_defaults.as_ref(),
        Frame::General(frame) => frame.frame_defaults.as_ref(),
        Frame::Unprocessed(_) => None,
    }
}
