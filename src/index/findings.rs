//! What the index decided about what a delivery states.
//!
//! Every decision the index takes where a delivery is not as the schema or a
//! reader would have it — a definition repeated with other content, a point a
//! journey passes without stopping, days a line version does not govern — is a
//! finding: counted always, listed with the document and the entity it is
//! about, in an order that does not depend on the order the documents were read
//! in. References to nothing are findings too; they keep their own, fuller
//! list ([`Index::unresolved`](super::Index::unresolved)).
//!
//! What is tolerated is the caller's choice: a [`Policy`] names the counts it
//! accepts of each concern and refuses an index that exceeds them, so that one
//! caller may refuse what another reads on with, over the same findings.

use std::collections::BTreeMap;
use std::fmt;

use super::strings::{Strings, Sym};

/// What a finding is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Concern {
    /// A reference to an identifier no document defines. Listed by
    /// [`Index::unresolved`](super::Index::unresolved), not by
    /// [`Index::findings`](super::Index::findings).
    Unresolved,
    /// An identifier defined again with other content. A document's own
    /// routes, lines, patterns, journeys and calendar are its own — a
    /// reference resolves in the document it stands in first — and a journey
    /// stated otherwise in two documents is two runs; a stop place, quay, stop
    /// point or level is one, kept as the document named first defines it.
    /// The finding names the redefining document and the document whose
    /// definition the delivery answers with.
    RedefinedOtherwise,
    /// A point of a journey pattern the vehicle passes without stopping
    /// (`StopUse` `passthrough`): no call, whatever its boarding and alighting
    /// flags say. The finding names the point and, as what else it names, the
    /// pattern.
    PassedThrough,
    /// Points of a journey's pattern before its first or after its last
    /// passing time: the journey runs part of its pattern, and does not call
    /// there.
    UntimedEnds,
    /// A journey left with fewer than two calls with a time — no pattern, no
    /// passing times, or one alone: left out of the index.
    TooFewTimedCalls,
    /// An operating period stating no first day: it gives no days.
    PeriodWithoutFirstDay,
    /// A UIC operating period with other day bits in number than days from
    /// its first to its last day: the bits are read from its first day as far
    /// as they go.
    DayBitsLengthDiffers,
    /// A journey whose day types and availability conditions give it no
    /// operating day: kept, running on none.
    JourneyWithoutDays,
    /// Days of a journey the version of its line it names does not govern
    /// ([`crate::calendar::governed`]), taken away. Names the line.
    DaysOutsideLineVersion,
    /// A journey none of whose days the version of its line it names
    /// governs: left out. Names the line.
    JourneyOutsideLineVersion,
    /// A journey of a flexible line — a call bus, a shared taxi —: an
    /// on-demand service, not runs at fixed times; left out. Names the line.
    FlexibleLineJourney,
    /// A headway group of a template journey stating no last departure or
    /// no interval: it makes no runs. Names the journey.
    HeadwayGroupWithoutRuns,
    /// A service link whose geometry is written longitude first: its first
    /// position lies near where the link starts read so, not read latitude
    /// first ([`crate::geometry::axis_order`]) — with the `crs` feature, read
    /// in the other order than its reference system states. Read only where
    /// service links are read.
    AxisOrderSwapped,
    /// A service link whose geometry's axis order cannot be told: its first
    /// position lies near where the link starts read neither way, or both, or
    /// the link's start has no position.
    AxisOrderUndecided,
    /// A reference that resolved only once the version it names was set
    /// aside: no definition states that version. Names the version.
    ResolvedIgnoringVersion,
    /// A reference that resolved to an entity of another class than its
    /// element names — an `OperatingPeriodRef` to a `UicOperatingPeriod`, as
    /// every day type assignment of the Austrian and German deliveries.
    /// Names the class found.
    ResolvedAsOtherClass,
    /// A position — of a stop place, quay, stop point or service link —
    /// written in a reference system the crate does not know: left without
    /// a position. Names the system. Made only with the `crs` feature, which
    /// reads positions in other systems than WGS 84 at all.
    UnknownReferenceSystem,
    /// A quay defined beside its stop place that two stop places claim — by
    /// naming it among their quays (`QuayRef`), or by the quay's `SiteRef`
    /// naming another than the one listing it: kept with the stop place
    /// that lists it first, a listing before a `SiteRef`. Names the other
    /// stop place.
    QuayOfTwoStopPlaces,
    /// A journey whose passing times name no point of its pattern — NeTEx
    /// 1.x leaves the reference out, and the French profile with it —, but
    /// are other in number than the points of its pattern: read by position
    /// as far as both go. Names the pattern.
    PassingTimesOtherThanPoints,
    /// A day type whose days are its properties' — a validity and days of
    /// the week, no assignment of its own giving such a day — but whose
    /// properties name more than days of the week (holidays, seasons, weeks of
    /// the month), which the crate does not read: it runs on the days of its
    /// assignments alone.
    PropertiesOfDayUnread,
}

impl Concern {
    /// Every concern, in the order of their `Ord`.
    pub const ALL: &'static [Concern] = &[
        Concern::Unresolved,
        Concern::RedefinedOtherwise,
        Concern::PassedThrough,
        Concern::UntimedEnds,
        Concern::TooFewTimedCalls,
        Concern::PeriodWithoutFirstDay,
        Concern::DayBitsLengthDiffers,
        Concern::JourneyWithoutDays,
        Concern::DaysOutsideLineVersion,
        Concern::JourneyOutsideLineVersion,
        Concern::FlexibleLineJourney,
        Concern::HeadwayGroupWithoutRuns,
        Concern::AxisOrderSwapped,
        Concern::AxisOrderUndecided,
        Concern::ResolvedIgnoringVersion,
        Concern::ResolvedAsOtherClass,
        Concern::UnknownReferenceSystem,
        Concern::QuayOfTwoStopPlaces,
        Concern::PassingTimesOtherThanPoints,
        Concern::PropertiesOfDayUnread,
    ];

    /// The concern's name in reports and on command lines, in lower case with
    /// hyphens: `unresolved`, `redefined-otherwise`, `passed-through`, …
    pub fn code(self) -> &'static str {
        match self {
            Concern::Unresolved => "unresolved",
            Concern::RedefinedOtherwise => "redefined-otherwise",
            Concern::PassedThrough => "passed-through",
            Concern::UntimedEnds => "untimed-ends",
            Concern::TooFewTimedCalls => "too-few-timed-calls",
            Concern::PeriodWithoutFirstDay => "period-without-first-day",
            Concern::DayBitsLengthDiffers => "day-bits-length-differs",
            Concern::JourneyWithoutDays => "journey-without-days",
            Concern::DaysOutsideLineVersion => "days-outside-line-version",
            Concern::JourneyOutsideLineVersion => "journey-outside-line-version",
            Concern::FlexibleLineJourney => "flexible-line-journey",
            Concern::HeadwayGroupWithoutRuns => "headway-group-without-runs",
            Concern::AxisOrderSwapped => "axis-order-swapped",
            Concern::AxisOrderUndecided => "axis-order-undecided",
            Concern::ResolvedIgnoringVersion => "resolved-ignoring-version",
            Concern::ResolvedAsOtherClass => "resolved-as-other-class",
            Concern::UnknownReferenceSystem => "unknown-reference-system",
            Concern::QuayOfTwoStopPlaces => "quay-of-two-stop-places",
            Concern::PassingTimesOtherThanPoints => "passing-times-other-than-points",
            Concern::PropertiesOfDayUnread => "properties-of-day-unread",
        }
    }

    /// The concern named `code` ([`code`](Self::code)).
    pub fn from_code(code: &str) -> Option<Concern> {
        Concern::ALL
            .iter()
            .copied()
            .find(|concern| concern.code() == code)
    }

    /// What a finding of this concern says, before what it is about.
    fn says(self) -> &'static str {
        match self {
            Concern::Unresolved => "reference to nothing",
            Concern::RedefinedOtherwise => "redefined with other content",
            Concern::PassedThrough => "point passed through, no call",
            Concern::UntimedEnds => {
                "pattern points before its first or after its last passing time, no calls"
            }
            Concern::TooFewTimedCalls => "journey without two timed calls, left out",
            Concern::PeriodWithoutFirstDay => "operating period without a first day, no days",
            Concern::DayBitsLengthDiffers => {
                "day bits other than the period's days, read from its first day"
            }
            Concern::JourneyWithoutDays => "journey without an operating day",
            Concern::DaysOutsideLineVersion => "days its line version does not govern, taken away",
            Concern::JourneyOutsideLineVersion => "no day its line version governs, left out",
            Concern::FlexibleLineJourney => {
                "journey of a flexible line, not runs at fixed times: left out"
            }
            Concern::HeadwayGroupWithoutRuns => {
                "headway group without a last departure or an interval, no runs"
            }
            Concern::AxisOrderSwapped => "link geometry written longitude first",
            Concern::AxisOrderUndecided => "link geometry whose axis order cannot be told",
            Concern::ResolvedIgnoringVersion => {
                "reference resolved only by ignoring the version it names"
            }
            Concern::ResolvedAsOtherClass => "reference resolved to another class than it names",
            Concern::UnknownReferenceSystem => {
                "position in a reference system the crate does not know, left without a position"
            }
            Concern::QuayOfTwoStopPlaces => "quay two stop places claim, kept with the first",
            Concern::PassingTimesOtherThanPoints => {
                "passing times naming no point, other in number than the pattern's points"
            }
            Concern::PropertiesOfDayUnread => {
                "day type whose properties name more than days of the week, its assignments' days alone"
            }
        }
    }
}

impl fmt::Display for Concern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.says())
    }
}

/// A finding as the index keeps it: the delivery's strings by number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FindingRecord {
    pub(crate) concern: Concern,
    pub(crate) element: &'static str,
    pub(crate) document: Sym,
    pub(crate) entity: Sym,
    pub(crate) about: Option<Sym>,
}

impl FindingRecord {
    /// The finding with its strings.
    pub(crate) fn resolve<'a>(&self, strings: &'a Strings) -> Finding<'a> {
        Finding {
            concern: self.concern,
            element: self.element,
            document: strings.resolve(self.document),
            entity: strings.resolve(self.entity),
            about: self.about.map(|about| strings.resolve(about)),
        }
    }
}

/// Sorts findings by concern, document, element, entity and what they name —
/// an order of their text, not of the order they were made in; `rank` gives
/// each document's place in the order of the documents' names, which is that
/// of their text without comparing it.
pub(crate) fn sort(findings: &mut [FindingRecord], strings: &Strings, rank: impl Fn(Sym) -> u32) {
    findings.sort_by_cached_key(|finding| {
        (
            finding.concern,
            rank(finding.document),
            finding.element,
            strings.resolve(finding.entity),
            finding.about.map(|about| strings.resolve(about)),
        )
    });
}

/// A decision the index took, about one entity of one document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Finding<'a> {
    /// What the finding is about.
    pub concern: Concern,
    /// The document it is about.
    pub document: &'a str,
    /// The element name of the entity it is about: `ServiceJourney`, `StopPlace`, …
    pub element: &'static str,
    /// The identifier of the entity it is about.
    pub entity: &'a str,
    /// What else it names: for a redefinition, the document whose definition
    /// the delivery answers with; for a point passed through, its pattern or
    /// journey; for days a line version does not govern or a journey of a
    /// flexible line, the line; for a headway group, its journey.
    pub about: Option<&'a str>,
}

impl fmt::Display for Finding<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} {} in {}",
            self.concern.says(),
            self.element,
            self.entity,
            self.document
        )?;
        match (self.concern, self.about) {
            (Concern::RedefinedOtherwise, Some(kept)) => write!(f, ", kept as {kept} defines it"),
            (_, Some(about)) => write!(f, " ({about})"),
            (_, None) => Ok(()),
        }
    }
}

/// What a caller accepts of the findings of an index: per concern, how many it
/// tolerates; every concern it names no limit for, in any number. The counts
/// are made whatever the policy; the policy decides whether an index is
/// refused for them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Policy {
    limits: BTreeMap<Concern, u64>,
}

impl Policy {
    /// A policy tolerating every finding.
    pub fn tolerant() -> Policy {
        Policy::default()
    }

    /// The policy refusing any finding of `concern`.
    pub fn refuse(self, concern: Concern) -> Policy {
        self.at_most(concern, 0)
    }

    /// The policy tolerating at most `limit` findings of `concern`.
    pub fn at_most(mut self, concern: Concern, limit: u64) -> Policy {
        self.limits.insert(concern, limit);
        self
    }

    /// Whether the index's findings stay within the policy; if not, every
    /// concern they exceed it in.
    pub fn check(&self, index: &super::Index) -> Result<(), Refusal> {
        let exceeded: Vec<Exceeded> = self
            .limits
            .iter()
            .map(|(&concern, &limit)| Exceeded {
                concern,
                count: index.finding_count(concern),
                limit,
            })
            .filter(|exceeded| exceeded.count > exceeded.limit)
            .collect();
        if exceeded.is_empty() {
            Ok(())
        } else {
            Err(Refusal { exceeded })
        }
    }
}

/// A concern whose findings exceed what a policy tolerates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Exceeded {
    /// The concern.
    pub concern: Concern,
    /// How many findings of it the index made.
    pub count: u64,
    /// How many the policy tolerates.
    pub limit: u64,
}

/// An index refused by a [`Policy`]: the concerns it exceeds the policy in, in
/// the order of [`Concern`]'s `Ord`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    /// The concerns exceeded.
    pub exceeded: Vec<Exceeded>,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("the delivery is refused: ")?;
        for (position, exceeded) in self.exceeded.iter().enumerate() {
            if position > 0 {
                f.write_str("; ")?;
            }
            write!(
                f,
                "{} findings `{}`, at most {} tolerated",
                exceeded.count, exceeded.concern, exceeded.limit
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for Refusal {}
