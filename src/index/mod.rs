//! The indexes a routing engine needs, built over a dataset of many documents.
//!
//! [`IndexBuilder`] takes documents one at a time; [`Index`] is what it
//! builds: stops by identifier and DHID with their quays and levels, journeys
//! by line with their passing times and the days they run, walks by stop, and
//! interchanges by stop point. Identifiers and names are interned once; the
//! entities are stored as flat arrays of small records, referred to by number.

mod build;
mod calendar;
mod findings;
mod finish;
mod merge;
mod positions;
mod referrers;
mod scope;
mod strings;

use std::fmt;

pub use build::IndexBuilder;
use build::{Key, LineRecord, StopPointRecord, StopRecord};
use calendar::DayBits;
use findings::FindingRecord;
pub use findings::{Concern, Exceeded, Finding, Policy, Refusal};
use referrers::Graph;
pub use referrers::{Object, ObjectId, Reference, Referrers};
use rustc_hash::FxHashMap;
use scope::{Scoped, Table};
use strings::{Strings, Sym};

use crate::dataset::{Entry, Source};
use crate::error::Result;
use crate::model::Date;
use crate::stream::ReadOptions;

/// A stop place or quay in the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StopId(pub u32);

/// A scheduled stop point in the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct StopPointId(pub u32);

/// A line in the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LineId(pub u32);

/// A journey in the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct JourneyId(pub u32);

/// A set of days journeys run on, shared by every journey of the index that
/// runs on those days and no other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CalendarId(pub u32);

/// What kind of stop an index entry is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopKind {
    /// A stop place.
    StopPlace,
    /// A quay of a stop place.
    Quay,
}

/// A stop as the index sees it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stop<'a> {
    /// The NeTEx identifier.
    pub id: &'a str,
    /// The DHID, from the `GlobalID` key, if the stop carries one.
    pub dhid: Option<&'a str>,
    /// The name.
    pub name: &'a str,
    /// Stop place or quay.
    pub kind: StopKind,
    /// Latitude in decimal degrees.
    pub latitude: Option<f64>,
    /// Longitude in decimal degrees.
    pub longitude: Option<f64>,
    /// The stop place a quay belongs to.
    pub parent: Option<StopId>,
    /// The short name of the level a quay is on.
    pub level: Option<&'a str>,
}

/// A walk between two stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Transfer {
    /// Where the walk starts.
    pub from: StopId,
    /// Where it ends.
    pub to: StopId,
    /// How long it takes, in seconds.
    pub seconds: u32,
    /// Whether it can be walked back the same way.
    pub both_ways: bool,
}

/// A planned interchange between two journeys at a stop point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interchange {
    /// The feeder journey.
    pub from_journey: JourneyId,
    /// The distributor journey.
    pub to_journey: JourneyId,
    /// Where the feeder arrives.
    pub from_stop_point: StopPointId,
    /// Where the distributor leaves.
    pub to_stop_point: StopPointId,
    /// Whether passengers stay on the vehicle.
    pub stay_seated: bool,
    /// Whether the distributor waits.
    pub guaranteed: bool,
    /// How long it waits at most, in seconds.
    pub maximum_wait_seconds: Option<u32>,
}

/// When a journey passes one stop of its pattern, in seconds since midnight
/// of its operating day; a time past midnight is past 86 400.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PassingTime {
    /// Arrival, if stated.
    pub arrival: Option<u32>,
    /// Departure, if stated.
    pub departure: Option<u32>,
}

impl PassingTime {
    /// No time stated.
    pub const NONE: PassingTime = PassingTime {
        arrival: None,
        departure: None,
    };
}

/// What a passenger may do at one call of a journey.
///
/// As the pattern states it — `ForBoarding`, `ForAlighting`, both `true` where
/// it states nothing —, but neither at a point whose `StopUse` is
/// `noBoardingOrAlighting`. A point the vehicle passes through is no call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CallUse {
    /// Whether one may board.
    pub boarding: bool,
    /// Whether one may alight.
    pub alighting: bool,
}

/// A group of runs of a template journey (`TemplateServiceJourney`): from the
/// first departure to the last, one every interval, in seconds since midnight
/// of the operating day. Each run calls where the journey calls, its times
/// those of [`Journey::passing_times`] shifted by its departure less the
/// journey's first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Headway {
    /// When the first run leaves.
    pub first_departure: u32,
    /// When the last run leaves.
    pub last_departure: u32,
    /// How long from one run to the next, more than zero.
    pub interval: u32,
}

impl Headway {
    /// The departures of the group's runs, from the first to the last.
    ///
    /// # Panics
    ///
    /// If the interval is zero, which no headway of an index is.
    pub fn departures(&self) -> impl Iterator<Item = u32> {
        let (first, interval) = (self.first_departure, self.interval);
        (first..=self.last_departure).step_by(interval as usize)
    }
}

/// A journey as the index sees it.
///
/// Its calls are the points of its pattern — or the calls it states, as the
/// Swiss delivery writes every journey — the vehicle stops at, from the first
/// with a time to the last: a point passed through is none, and neither is a
/// point before the journey's first or after its last time
/// ([`Concern::PassedThrough`], [`Concern::UntimedEnds`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Journey<'a> {
    /// The NeTEx identifier.
    pub id: &'a str,
    /// The document defining it.
    pub document: &'a str,
    /// The line: the one it names, else the one of the route it names, else
    /// its pattern's — the one the pattern's route view names, else the one
    /// of the route the pattern names.
    pub line: Option<LineId>,
    /// The NeTEx identifier of the journey pattern followed; none for a
    /// journey stating its calls.
    pub pattern: Option<&'a str>,
    /// The stop points called at, in order.
    pub stop_points: &'a [StopPointId],
    /// When each stop point is passed, one per stop point.
    pub passing_times: &'a [PassingTime],
    /// What a passenger may do at each stop point, one per stop point.
    pub uses: &'a [CallUse],
    /// For a template journey, the groups of runs it stands for; empty for
    /// a journey that is one run.
    pub headways: &'a [Headway],
}

/// A reference that did not resolve; ordered by document, referrer, element,
/// identifier and version, the order [`Index::unresolved`] lists them in.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Unresolved {
    /// The document the reference is in.
    pub origin: String,
    /// The element holding the reference.
    pub referrer: String,
    /// The reference's element name.
    pub element: String,
    /// The identifier referred to.
    pub id: String,
    /// The version referred to, if one was.
    pub version: Option<String>,
}

impl fmt::Display for Unresolved {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: <{}> in {} refers to {}",
            self.origin, self.element, self.referrer, self.id
        )?;
        if let Some(version) = &self.version {
            write!(f, " version {version}")?;
        }
        write!(f, ", which no document defines")
    }
}

/// The indexes: stops by identifier and DHID, journeys by line and day,
/// walks by stop, interchanges by stop point.
///
/// What a document defines of its own — lines, journeys and their patterns
/// and calendar — is numbered in the order of the documents' names, within a
/// document in the order it states it; so is every finding. Two indexes are
/// equal when they were built from the same entities with stops and stop
/// points gathered in the same order — which is how the merged builders of a
/// parallel gather are checked against one builder fed every document.
#[derive(Debug, PartialEq)]
pub struct Index {
    strings: Strings,
    documents: u32,
    redefinitions: u64,
    stops: Vec<StopRecord>,
    stop_table: Table,
    stop_by_dhid: FxHashMap<Sym, StopId>,
    stop_points: Vec<StopPointRecord>,
    stop_point_table: Table,
    lines: Vec<LineRecord>,
    /// The lines' definitions, by position, and the line each is.
    line_scope: Scoped,
    line_of_position: Vec<LineId>,
    patterns: Vec<IndexedPattern>,
    /// The calls of every pattern, and of every journey stating its own:
    /// where, and what a passenger may do there.
    call_stop_points: Vec<StopPointId>,
    call_uses: Vec<CallUse>,
    journeys: Vec<IndexedJourney>,
    /// The journeys' definitions, by position, and the journey each is, if
    /// the index keeps it.
    journey_scope: Scoped,
    journey_of_position: Vec<Option<JourneyId>>,
    passing_times: Vec<PassingTime>,
    headways: Vec<Headway>,
    calendars: Vec<DayBits>,
    first_day: i64,
    line_journeys_offsets: Vec<u32>,
    line_journeys: Vec<JourneyId>,
    transfer_offsets: Vec<u32>,
    transfers: Vec<Transfer>,
    interchange_offsets: Vec<u32>,
    interchanges: Vec<Interchange>,
    unresolved: Vec<Unresolved>,
    findings: Vec<FindingRecord>,
    /// Every reference resolved, as a graph, where the builder was asked for
    /// it ([`IndexBuilder::with_referrers`]).
    graph: Option<Graph>,
}

/// A journey pattern as the index keeps it: its key, and its calls, a range
/// of the index's call stop points and uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct IndexedPattern {
    pub(crate) key: Key,
    pub(crate) stop_points: (u32, u32),
}

/// A journey as the index keeps it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct IndexedJourney {
    pub(crate) key: Key,
    /// The document defining it.
    pub(crate) origin: Sym,
    pub(crate) line: Option<LineId>,
    pub(crate) pattern: Option<u32>,
    /// The calls it makes, a range of the index's call stop points and uses.
    pub(crate) calls: (u32, u32),
    pub(crate) calendar: u32,
    /// Its passing times, a range of the index's passing times.
    pub(crate) times: (u32, u32),
    /// A template's headway groups, a range of the index's headways.
    pub(crate) headways: (u32, u32),
}

/// A line as the index sees it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Line<'a> {
    /// The NeTEx identifier.
    pub id: &'a str,
    /// The name.
    pub name: &'a str,
    /// The number passengers see.
    pub public_code: Option<&'a str>,
    /// The mode, as its token.
    pub mode: Option<&'a str>,
}

/// A scheduled stop point as the index sees it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StopPoint<'a> {
    /// The NeTEx identifier.
    pub id: &'a str,
    /// The name.
    pub name: &'a str,
    /// Latitude in decimal degrees.
    pub latitude: Option<f64>,
    /// Longitude in decimal degrees.
    pub longitude: Option<f64>,
    /// The stop it is assigned to: its quay, else its stop place.
    pub stop: Option<StopId>,
}

impl Index {
    /// Builds the index of a whole dataset, lean: every document is read on
    /// every thread of the current rayon pool with [`crate::stream::read`]
    /// under `options`, largest first, each worker gathering into a builder
    /// of its own; the builders are merged and resolved once every document
    /// is in. No document model is held; peak memory is the builders'.
    ///
    /// A document that fails to read fails the build, named in the error;
    /// when several fail, which one is named depends on the order the
    /// workers reached them.
    pub fn build(source: &Source, options: &ReadOptions) -> Result<Index> {
        Self::build_from(source, source.entries()?, options)
    }

    /// Like [`build`](Self::build), from `entries` of the source only — the
    /// documents a [`crate::dataset::Manifest::diff`] says are new or changed,
    /// say.
    pub fn build_from(
        source: &Source,
        entries: Vec<Entry>,
        options: &ReadOptions,
    ) -> Result<Index> {
        let workers = source.read_entries_with(
            entries,
            || (IndexBuilder::new(), None),
            |(builder, failure): &mut (IndexBuilder, Option<crate::Error>), entry, text| {
                if failure.is_some() {
                    return;
                }
                if let Err(error) =
                    text.and_then(|xml| builder.add_streamed(xml, options, &entry.name))
                {
                    *failure = Some(error.in_document(&entry.name));
                }
            },
        )?;
        let mut builders = Vec::with_capacity(workers.len());
        for (builder, failure) in workers {
            if let Some(error) = failure {
                return Err(error);
            }
            builders.push(builder);
        }
        Ok(IndexBuilder::merge_all(builders).finish())
    }

    fn text(&self, sym: Sym) -> &str {
        self.strings.resolve(sym)
    }

    /// How many documents the index was built from.
    pub fn document_count(&self) -> u32 {
        self.documents
    }

    /// How many entity definitions repeated one already in, by identifier and
    /// version, alike or — for a stop place, quay or stop point — stating the
    /// same stop, and were left out. A definition stating its entity otherwise
    /// is a finding ([`Concern::RedefinedOtherwise`]).
    pub fn redefinition_count(&self) -> u64 {
        self.redefinitions
    }

    /// How many distinct identifiers and names are interned.
    pub fn string_count(&self) -> usize {
        self.strings.len()
    }

    /// The stop with this NeTEx identifier, at its highest version.
    pub fn stop_by_id(&self, id: &str) -> Option<StopId> {
        let sym = self.strings.get(id)?;
        self.stop_table.latest_of(sym).map(StopId)
    }

    /// The stop with this DHID; of several stops that carry it, the one whose
    /// identifier sorts first, at its highest version.
    pub fn stop_by_dhid(&self, dhid: &str) -> Option<StopId> {
        let sym = self.strings.get(dhid)?;
        self.stop_by_dhid.get(&sym).copied()
    }

    /// The stop.
    pub fn stop(&self, stop: StopId) -> Stop<'_> {
        let record = &self.stops[stop.0 as usize];
        Stop {
            id: self.text(record.key.id),
            dhid: record.dhid.map(|d| self.text(d)),
            name: self.text(record.name),
            kind: record.kind,
            latitude: record.latitude,
            longitude: record.longitude,
            parent: record.parent,
            level: record.level.map(|l| self.text(l)),
        }
    }

    /// How many stops the index holds.
    pub fn stop_count(&self) -> usize {
        self.stops.len()
    }

    /// Every stop.
    pub fn stops(&self) -> impl Iterator<Item = StopId> {
        (0..u32::try_from(self.stops.len()).expect("fewer than 2^32 stops")).map(StopId)
    }

    /// The stop point with this NeTEx identifier, at its highest version.
    pub fn stop_point_by_id(&self, id: &str) -> Option<StopPointId> {
        let sym = self.strings.get(id)?;
        self.stop_point_table.latest_of(sym).map(StopPointId)
    }

    /// The stop point.
    pub fn stop_point(&self, stop_point: StopPointId) -> StopPoint<'_> {
        let record = &self.stop_points[stop_point.0 as usize];
        StopPoint {
            id: self.text(record.key.id),
            name: self.text(record.name),
            latitude: record.latitude,
            longitude: record.longitude,
            stop: record.stop,
        }
    }

    /// How many stop points the index holds.
    pub fn stop_point_count(&self) -> usize {
        self.stop_points.len()
    }

    /// The stop a stop point is assigned to: its quay, else its stop place.
    pub fn stop_of_stop_point(&self, stop_point: StopPointId) -> Option<StopId> {
        self.stop_points.get(stop_point.0 as usize)?.stop
    }

    /// The line with this identifier, at its highest version; of the
    /// documents defining it, as the one named first defines it.
    pub fn line_by_id(&self, id: &str) -> Option<LineId> {
        let sym = self.strings.get(id)?;
        let position = self.line_scope.latest_of_delivery(sym)?;
        Some(self.line_of_position[position as usize])
    }

    /// The line.
    pub fn line(&self, line: LineId) -> Line<'_> {
        let record = &self.lines[line.0 as usize];
        Line {
            id: self.text(record.key.id),
            name: self.text(record.name),
            public_code: record.public_code.map(|c| self.text(c)),
            mode: record.mode.map(|m| self.text(m)),
        }
    }

    /// How many lines the index holds.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// The journeys of a line.
    pub fn journeys_of_line(&self, line: LineId) -> &[JourneyId] {
        let start = self.line_journeys_offsets[line.0 as usize] as usize;
        let end = self.line_journeys_offsets[line.0 as usize + 1] as usize;
        &self.line_journeys[start..end]
    }

    /// The journey with this identifier, at its highest version; of the
    /// documents defining it, as the one named first defines it.
    pub fn journey_by_id(&self, id: &str) -> Option<JourneyId> {
        let sym = self.strings.get(id)?;
        let position = self.journey_scope.latest_of_delivery(sym)?;
        self.journey_of_position[position as usize]
    }

    /// The journey with this identifier, at its highest version, as
    /// `document` defines it: a reference in `document` resolves to it.
    pub fn journey_in_document(&self, id: &str, document: &str) -> Option<JourneyId> {
        let id = self.strings.get(id)?;
        let document = self.strings.get(document)?;
        let journey = match self.journey_scope.latest_in(id, document) {
            Some(position) => self.journey_of_position[position as usize]?,
            None => {
                let position = self.journey_scope.latest_of_delivery(id)?;
                let journey = self.journey_of_position[position as usize]?;
                if self.journeys[journey.0 as usize].origin != document {
                    return None;
                }
                journey
            }
        };
        Some(journey)
    }

    /// The journey.
    pub fn journey(&self, journey: JourneyId) -> Journey<'_> {
        let record = &self.journeys[journey.0 as usize];
        let calls = record.calls.0 as usize..record.calls.1 as usize;
        Journey {
            id: self.text(record.key.id),
            document: self.text(record.origin),
            line: record.line,
            pattern: record
                .pattern
                .map(|p| self.text(self.patterns[p as usize].key.id)),
            stop_points: &self.call_stop_points[calls.clone()],
            passing_times: &self.passing_times[record.times.0 as usize..record.times.1 as usize],
            uses: &self.call_uses[calls],
            headways: &self.headways[record.headways.0 as usize..record.headways.1 as usize],
        }
    }

    /// Whether a journey runs on a day.
    pub fn runs_on(&self, journey: JourneyId, day: &Date) -> bool {
        let Some(day) = day.days_since_epoch() else {
            return false;
        };
        let Ok(offset) = usize::try_from(day - self.first_day) else {
            return false;
        };
        let calendar = self.journeys[journey.0 as usize].calendar;
        self.calendars[calendar as usize].get(offset)
    }

    /// The days a journey runs on, as days since 1970-01-01
    /// ([`Date::days_since_epoch`]), ascending.
    pub fn days_of(&self, journey: JourneyId) -> impl Iterator<Item = i64> + '_ {
        self.calendar_days(self.calendar_of(journey))
    }

    /// The calendar a journey runs on: the same as every journey's of the
    /// same days.
    pub fn calendar_of(&self, journey: JourneyId) -> CalendarId {
        CalendarId(self.journeys[journey.0 as usize].calendar)
    }

    /// The days of a calendar, as days since 1970-01-01
    /// ([`Date::days_since_epoch`]), ascending.
    pub fn calendar_days(&self, calendar: CalendarId) -> impl Iterator<Item = i64> + '_ {
        let first = self.first_day;
        self.calendars[calendar.0 as usize]
            .days()
            .map(move |offset| first + offset as i64)
    }

    /// The journeys of a line that run on a day.
    pub fn journeys_of_line_on<'a>(
        &'a self,
        line: LineId,
        day: &Date,
    ) -> impl Iterator<Item = JourneyId> + 'a {
        let day = day
            .days_since_epoch()
            .and_then(|d| usize::try_from(d - self.first_day).ok());
        self.journeys_of_line(line)
            .iter()
            .copied()
            .filter(move |journey| {
                day.is_some_and(|offset| {
                    self.calendars[self.journeys[journey.0 as usize].calendar as usize].get(offset)
                })
            })
    }

    /// How many journeys the index holds.
    pub fn journey_count(&self) -> usize {
        self.journeys.len()
    }

    /// How many passing times the index holds.
    pub fn passing_time_count(&self) -> usize {
        self.passing_times.len()
    }

    /// How many distinct calendars the journeys share, numbered from zero
    /// ([`CalendarId`]).
    pub fn calendar_count(&self) -> usize {
        self.calendars.len()
    }

    /// The walks starting at a stop.
    pub fn transfers_from(&self, stop: StopId) -> &[Transfer] {
        let start = self.transfer_offsets[stop.0 as usize] as usize;
        let end = self.transfer_offsets[stop.0 as usize + 1] as usize;
        &self.transfers[start..end]
    }

    /// How many walks the index holds, a two-way walk counting twice.
    pub fn transfer_count(&self) -> usize {
        self.transfers.len()
    }

    /// The interchanges at a stop point, by where the feeder arrives.
    pub fn interchanges_at(&self, stop_point: StopPointId) -> &[Interchange] {
        let start = self.interchange_offsets[stop_point.0 as usize] as usize;
        let end = self.interchange_offsets[stop_point.0 as usize + 1] as usize;
        &self.interchanges[start..end]
    }

    /// How many interchanges the index holds.
    pub fn interchange_count(&self) -> usize {
        self.interchanges.len()
    }

    /// The references that did not resolve, in the order of [`Unresolved`]'s
    /// `Ord` — the same list whatever order the documents were read in.
    pub fn unresolved(&self) -> &[Unresolved] {
        &self.unresolved
    }

    /// What the index decided about what the delivery states, in the order
    /// of [`Finding`]'s `Ord` — the same list whatever order the documents
    /// were read in. References to nothing are listed by
    /// [`unresolved`](Self::unresolved).
    pub fn findings(&self) -> impl Iterator<Item = Finding<'_>> + '_ {
        self.findings
            .iter()
            .map(|finding| finding.resolve(&self.strings))
    }

    /// What refers to each object of the delivery, and what nothing refers
    /// to: every reference the index resolved, read backwards — where the
    /// builder was asked for them ([`IndexBuilder::with_referrers`]); none
    /// otherwise.
    pub fn referrers(&self) -> Option<Referrers<'_>> {
        self.graph.as_ref().map(|graph| Referrers::new(self, graph))
    }

    /// How many findings of `concern` the index made.
    pub fn finding_count(&self, concern: Concern) -> u64 {
        if concern == Concern::Unresolved {
            return self.unresolved.len() as u64;
        }
        self.findings
            .iter()
            .filter(|finding| finding.concern == concern)
            .count() as u64
    }
}
