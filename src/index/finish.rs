//! Resolving the index once every document is in.
//!
//! What a document defines of its own — routes, lines, journey patterns,
//! journeys, the calendar — is taken in the order of the documents' names,
//! within a document in the order it states it: the index numbers it so, and
//! of the definitions several documents state, the delivery answers with the
//! one of the document named first ([`Scoped`]). A reference resolves in the
//! document it stands in first. A definition repeated alike is one entity;
//! repeated with other content, it is the document's own, and a finding.

use std::borrow::Cow;
use std::collections::hash_map::Entry;
use std::collections::HashMap;

use rustc_hash::FxHashMap;

use super::build::{
    level_fingerprint, precedes, version_is_higher, AssignedDaysRecord, IndexBuilder,
    JourneyRecord, Key, LevelRecord, Pending, PeriodRecord, RawCall, RawHeadway, RawTime, Validity,
};
use super::calendar::DayBits;
use super::findings::{self, Concern, FindingRecord};
use super::referrers::{Kind, Recorder};
use super::scope::{resolve, Lookup, Resolved, Scoped};
use super::strings::{Strings, Sym};
use super::{
    CallUse, Headway, Index, IndexedJourney, IndexedPattern, Interchange, JourneyId, LineId,
    PassingTime, StopId, StopKind, StopPointId, Transfer, Unresolved,
};
use crate::calendar::{governed, DaysOfWeek, Governed, PeriodDays, PeriodIssue, Properties, Span};
use crate::geometry::{reading_order, AxisOrder};

/// Builds the index; see [`IndexBuilder::finish`].
pub(super) fn finish(builder: IndexBuilder) -> Index {
    let ranks = Ranks::of(&builder.strings, &builder.document_names);
    let mut builder = builder;
    let gathered = std::mem::take(&mut builder.gathered);
    let mut work = Finish {
        b: builder,
        journey_order: Vec::new(),
        level_order: Vec::new(),
        findings: gathered,
        recorder: None,
    };
    work.sort_by_document(&ranks);
    if work.b.referrers {
        work.recorder = Some(work.definitions());
    }
    work.resolve_levels();
    work.resolve_quay_places();
    work.resolve_assignments();
    work.check_links();
    work.variant_findings();
    let scopes = work.scopes();
    let lines = work.lines(&scopes);
    let routes = work.routes(&scopes);
    let mut patterns = work.patterns(&scopes, &routes);
    let calendar = work.calendar(&scopes);
    let dated = work.dated_days(&scopes);
    let journeys = work.journeys(
        &scopes,
        (&lines, &routes),
        &mut patterns,
        (&calendar, &dated),
    );
    let (line_journeys_offsets, line_journeys) = by_line(&journeys.records, lines.kept.len());
    let (transfer_offsets, transfers) = work.transfers();
    let (interchange_offsets, interchanges) = work.interchanges(&scopes, &journeys);
    let stop_by_dhid = work.stops_by_dhid();

    let Finish {
        mut b,
        mut findings,
        recorder,
        ..
    } = work;
    // An element that refers is the definition it is, where the index holds
    // one: the quay naming its level, the pattern naming its stop points, the
    // journey naming its pattern; it is found as its references were, in its
    // own document first.
    let graph = recorder.map(|recorder| {
        recorder.finish(|element, id, document| match element {
            "StopPlace" | "Quay" => b.stop_table.latest_of(id).map(|stop| (Kind::Stop, stop)),
            "Route" => scopes
                .routes
                .table
                .latest(id, document)
                .map(|position| (Kind::Route, position)),
            "ServiceJourneyPattern" => scopes
                .patterns
                .table
                .latest(id, document)
                .map(|position| (Kind::Pattern, position)),
            "ServiceJourney" => scopes
                .journeys
                .table
                .latest(id, document)
                .map(|position| (Kind::Journey, position)),
            _ => None,
        })
    });
    let mut taken: Vec<Option<_>> = std::mem::take(&mut b.lines).into_iter().map(Some).collect();
    let kept_lines = lines
        .kept
        .iter()
        .map(|&position| {
            taken[position as usize]
                .take()
                .expect("a line is kept once")
        })
        .collect();
    // Findings in a fixed order, not the order the documents came in.
    b.unresolved.sort_unstable();
    findings::sort(&mut findings, &b.strings, |document| {
        ranks.of_document(document)
    });
    Index {
        documents: u32::try_from(b.document_names.len()).expect("fewer than 2^32 documents"),
        redefinitions: b.redefinitions,
        stops: b.stops,
        stop_table: b.stop_table,
        stop_by_dhid,
        stop_points: b.stop_points,
        stop_point_table: b.stop_point_table,
        lines: kept_lines,
        line_scope: scopes.lines.table,
        line_of_position: lines.of_position,
        patterns: patterns.indexed,
        call_stop_points: patterns.calls.stop_points,
        call_uses: patterns.calls.uses,
        journeys: journeys.records,
        journey_scope: scopes.journeys.table,
        journey_of_position: journeys.of_position,
        passing_times: journeys.passing_times,
        headways: journeys.headways,
        calendars: journeys.calendars,
        first_day: calendar.first_day,
        line_journeys_offsets,
        line_journeys,
        transfer_offsets,
        transfers,
        interchange_offsets,
        interchanges,
        unresolved: b.unresolved,
        findings,
        graph,
        strings: b.strings,
    }
}

/// The work of [`IndexBuilder::finish`]: the builder, the order its journeys
/// are taken in, and the findings made on the way.
struct Finish {
    b: IndexBuilder,
    /// The builder's journeys, by position: in the order of their documents'
    /// names, within a document in the order it states them.
    journey_order: Vec<u32>,
    /// The builder's levels, by position, where references are recorded: in
    /// the order of their identifiers and versions.
    level_order: Vec<Key>,
    findings: Vec<FindingRecord>,
    /// The references as they resolve, where the builder was asked for them
    /// ([`IndexBuilder::with_referrers`]).
    recorder: Option<Recorder>,
}

/// The definitions of one kind a document's references resolve in, by
/// position, and the days each is valid on where a reference may be resolved
/// by them ([`IndexBuilder::at`]).
struct Scope {
    kind: Kind,
    table: Scoped,
    validity: Vec<Validity>,
}

impl Scope {
    fn validity(&self, position: u32) -> Validity {
        self.validity
            .get(position as usize)
            .copied()
            .unwrap_or(Validity::ALWAYS)
    }
}

/// The scopes of everything a document defines of its own.
struct Scopes {
    routes: Scope,
    lines: Scope,
    patterns: Scope,
    day_types: Scope,
    periods: Scope,
    conditions: Scope,
    operating_days: Scope,
    journeys: Scope,
}

/// The lines: the line each definition is in the index, the definitions
/// kept, by position, in the order of the index's numbering, and the versions
/// of each line — the definitions of its identifier in one document, by
/// position, in the order the document states them.
struct Lines {
    of_position: Vec<LineId>,
    kept: Vec<u32>,
    versions: FxHashMap<(Sym, Sym), Vec<u32>>,
}

/// The patterns by position: what the index keeps of each; the calls of all,
/// and the point of the builder each pattern call is; the range of each
/// pattern's points in the builder; and the line definition each names, by
/// position.
struct Patterns {
    indexed: Vec<IndexedPattern>,
    calls: CallLists,
    points_of_calls: Vec<u32>,
    points: Vec<(u32, u32)>,
    lines: Vec<Option<u32>>,
}

/// The calls of every pattern, and of every journey stating its own: where,
/// and what a passenger may do there. Journeys stating the same calls share
/// one run of the lists.
#[derive(Default)]
struct CallLists {
    stop_points: Vec<StopPointId>,
    uses: Vec<CallUse>,
    of_journeys: HashMap<Vec<(StopPointId, CallUse)>, u32>,
}

impl CallLists {
    /// The range of the lists holding `calls`, appended where no journey
    /// stated them before.
    fn of(&mut self, calls: &[(StopPointId, CallUse)]) -> (u32, u32) {
        let start = match self.of_journeys.get(calls) {
            Some(&start) => start,
            None => {
                let start = position_of(self.stop_points.len());
                self.stop_points
                    .extend(calls.iter().map(|(point, _)| *point));
                self.uses.extend(calls.iter().map(|(_, call)| *call));
                self.of_journeys.insert(calls.to_vec(), start);
                start
            }
        };
        (start, start + position_of(calls.len()))
    }
}

/// A day type assignment as it applies: the day type and the days it names
/// resolved, and whether it gives them or takes them away.
struct ResolvedAssignment {
    day_type: u32,
    days: AssignmentDays,
    available: bool,
}

/// The days an assignment names: one day — a date or an operating day's —,
/// or an operating period's, by position.
#[derive(Clone, Copy)]
enum AssignmentDays {
    Day(i64),
    Period(u32),
}

/// The days of the calendar's span from `from` to `to`, both included.
fn span_bits(first_day: i64, span: usize, from: i64, to: i64) -> DayBits {
    let mut bits = DayBits::new(span);
    let last = first_day + span as i64 - 1;
    for day in from.max(first_day)..=to.min(last) {
        bits.set((day - first_day) as usize, true);
    }
    bits
}

/// The days of the calendar's span falling on given days of the week, made
/// once for each set of days of the week asked for.
struct WeekBits {
    first_day: i64,
    span: usize,
    made: FxHashMap<DaysOfWeek, DayBits>,
}

impl WeekBits {
    fn new(first_day: i64, span: usize) -> Self {
        WeekBits {
            first_day,
            span,
            made: FxHashMap::default(),
        }
    }

    /// The days of the span on `days`.
    fn of(&mut self, days: DaysOfWeek) -> &DayBits {
        let (first_day, span) = (self.first_day, self.span);
        self.made.entry(days).or_insert_with(|| {
            let mut bits = DayBits::new(span);
            for offset in 0..span {
                if days.contains(first_day + offset as i64) {
                    bits.set(offset, true);
                }
            }
            bits
        })
    }
}

/// The calendar of the dataset: one bit set per day type definition and per
/// availability condition, by position, over the dataset's span of days, and
/// the day the span starts.
struct Calendar {
    first_day: i64,
    span: usize,
    day_types: Vec<DayBits>,
    conditions: Vec<DayBits>,
}

/// The calendars the journeys run on, found by what gives them — day types,
/// availability conditions and line version — and by their days; and the
/// days each line definition governs, where it does not govern every day.
#[derive(Default)]
struct Calendars {
    of_parts: HashMap<CalendarParts, Calendared>,
    of_days: HashMap<DayBits, u32>,
    governed: FxHashMap<u32, Option<DayBits>>,
}

/// What a journey's calendar is made of, resolved: its day types and
/// availability conditions by position, its line definition, and the days
/// dated journeys give it.
type CalendarParts = (Vec<u32>, Vec<u32>, Option<u32>, Vec<i64>);

/// What gives a journey its days: the day types and the availability
/// conditions it names, the line definition it runs in, and the days dated
/// journeys give it.
struct Days<'a> {
    day_types: &'a [Pending],
    conditions: &'a [Pending],
    line: Option<u32>,
    dated: &'a [i64],
}

/// The days dated journeys give the journeys they name, by the position of
/// the journey, in order.
type DatedDays = FxHashMap<u32, Vec<i64>>;

/// What a journey states of its own, as gathered: the references to its
/// calendar, its passing times or calls, and a template's headway groups,
/// each journey holding a range of each.
struct Raw {
    day_types: Vec<Pending>,
    conditions: Vec<Pending>,
    times: Vec<RawTime>,
    calls: Vec<RawCall>,
    headways: Vec<RawHeadway>,
}

/// A journey's calls as laid out, before they are cut to its timed ones:
/// where its pattern's calls start, or the calls it states itself; and the
/// time at each.
#[derive(Default)]
struct LaidOut {
    pattern_calls: Option<u32>,
    calls: Vec<(StopPointId, CallUse)>,
    times: Vec<PassingTime>,
}

impl LaidOut {
    fn clear(&mut self) {
        self.pattern_calls = None;
        self.calls.clear();
        self.times.clear();
    }
}

/// A journey's calendar, and how many days its day types give it before and
/// after its line version took those it does not govern.
#[derive(Debug, Clone, Copy)]
struct Calendared {
    index: u32,
    every: usize,
    kept: usize,
}

/// The journeys as resolved: the index's records, the journey each definition
/// is in the index — none for one left out —, the passing times, and the
/// calendars they run on.
struct Journeys {
    records: Vec<IndexedJourney>,
    of_position: Vec<Option<JourneyId>>,
    passing_times: Vec<PassingTime>,
    headways: Vec<Headway>,
    calendars: Vec<DayBits>,
}

impl Finish {
    /// Records the finding of how `pending` resolved, if it has one, and
    /// yields the entity.
    fn note(
        &mut self,
        pending: &Pending,
        kind: Kind,
        resolution: Result<Resolved, Unresolved>,
    ) -> Option<u32> {
        match resolution {
            Ok(Resolved {
                index,
                version_set_aside,
            }) => {
                if let Some(recorder) = &mut self.recorder {
                    recorder.record(pending, kind, index);
                }
                if let Some(version) = version_set_aside {
                    self.found(
                        Concern::ResolvedIgnoringVersion,
                        pending.element,
                        (pending.origin, pending.key.id),
                        Some(version),
                    );
                }
                Some(index)
            }
            Err(unresolved) => {
                self.b.unresolved.push(unresolved);
                None
            }
        }
    }

    /// A finding about the entity `element` `entity` of `document`, naming `about`.
    fn found(
        &mut self,
        concern: Concern,
        element: &'static str,
        (document, entity): (Sym, Sym),
        about: Option<Sym>,
    ) {
        self.findings.push(FindingRecord {
            concern,
            element,
            document,
            entity,
            about,
        });
    }

    /// A definition of `entity` in `document` with other content than the one
    /// of `kept`, which the delivery answers with.
    fn redefined(&mut self, element: &'static str, entity: Sym, document: Sym, kept: Sym) {
        self.found(
            Concern::RedefinedOtherwise,
            element,
            (document, entity),
            Some(kept),
        );
    }

    /// Resolves `pending` in `scope`, as the document it stands in sees it.
    fn resolve_in(&mut self, scope: &Scope, pending: &Pending) -> Option<u32> {
        let resolution = resolve(
            &self.b.strings,
            &scope.table,
            |position| scope.validity(position),
            self.b.at,
            pending,
        );
        self.note(pending, scope.kind, resolution)
    }

    fn resolve_stop(&mut self, pending: &Pending) -> Option<u32> {
        let resolution = resolve(
            &self.b.strings,
            &self.b.stop_table,
            |i| self.b.stops[i as usize].validity,
            self.b.at,
            pending,
        );
        self.note(pending, Kind::Stop, resolution)
    }

    fn resolve_stop_point(&mut self, pending: &Pending) -> Option<u32> {
        let resolution = resolve(
            &self.b.strings,
            &self.b.stop_point_table,
            |i| self.b.stop_points[i as usize].validity,
            self.b.at,
            pending,
        );
        self.note(pending, Kind::StopPoint, resolution)
    }

    /// Records the reference of a point the vehicle passes through to its
    /// stop point, where references are recorded: no call, so not resolved
    /// for the index — nothing is found about it —, but written all the same.
    fn record_passed(&mut self, pending: &Pending) {
        let Some(recorder) = &mut self.recorder else {
            return;
        };
        let resolution = resolve(
            &self.b.strings,
            &self.b.stop_point_table,
            |i| self.b.stop_points[i as usize].validity,
            self.b.at,
            pending,
        );
        if let Ok(resolved) = resolution {
            recorder.record(pending, Kind::StopPoint, resolved.index);
        }
    }

    /// A finding where `pending`, a reference whose element names the class
    /// it refers to (`OperatingPeriodRef`), resolved to an entity of another
    /// class, `found` (`UicOperatingPeriod`).
    fn check_class(&mut self, pending: &Pending, found: &'static str) {
        let named = pending
            .element
            .strip_suffix("Ref")
            .unwrap_or(pending.element);
        if named != found {
            let found = self.b.strings.intern(found);
            self.found(
                Concern::ResolvedAsOtherClass,
                pending.element,
                (pending.origin, pending.key.id),
                Some(found),
            );
        }
    }

    /// The axis order of every service link's geometry, judged against the
    /// position of the stop point it starts at — the point's own, else its
    /// stop's, else that stop's stop place's.
    fn check_links(&mut self) {
        let links = std::mem::take(&mut self.b.links);
        for link in &links {
            let start = match &link.from {
                Some(pending) => self
                    .resolve_stop_point(pending)
                    .and_then(|point| self.position_of_stop_point(point)),
                None => None,
            };
            let order = start.map_or(AxisOrder::Undecided, |start| {
                reading_order(link.readings, start)
            });
            let concern = match order {
                AxisOrder::LatitudeFirst => continue,
                AxisOrder::LongitudeFirst => Concern::AxisOrderSwapped,
                AxisOrder::Undecided => Concern::AxisOrderUndecided,
            };
            self.found(concern, "ServiceLink", (link.origin, link.id), None);
        }
    }

    /// Where a stop point is, as latitude and longitude: its own position,
    /// else its stop's, else that stop's stop place's.
    fn position_of_stop_point(&self, point: u32) -> Option<(f64, f64)> {
        let at = |latitude: Option<f64>, longitude: Option<f64>| latitude.zip(longitude);
        let record = &self.b.stop_points[point as usize];
        at(record.latitude, record.longitude).or_else(|| {
            let stop = &self.b.stops[record.stop?.0 as usize];
            at(stop.latitude, stop.longitude).or_else(|| {
                let place = &self.b.stops[stop.parent?.0 as usize];
                at(place.latitude, place.longitude)
            })
        })
    }

    /// Levels of quays: the level named, else the highest version of its identifier.
    fn resolve_levels(&mut self) {
        for index in 0..self.b.stops.len() {
            let Some(pending) = self.b.stops[index].level_ref else {
                continue;
            };
            let level = self
                .b
                .levels
                .get_key_value(&pending.key)
                .or_else(|| {
                    self.b
                        .levels
                        .iter()
                        .filter(|(key, _)| key.id == pending.key.id)
                        .reduce(|highest, candidate| {
                            if version_is_higher(
                                &self.b.strings,
                                candidate.0.version,
                                highest.0.version,
                            ) {
                                candidate
                            } else {
                                highest
                            }
                        })
                })
                .map(|(&key, level)| (key, level.name));
            self.b.stops[index].level = level.map(|(_, name)| name);
            if let (Some(recorder), Some((key, _))) = (&mut self.recorder, level) {
                let position = self
                    .level_order
                    .binary_search_by(|probe| level_order(&self.b.strings, probe, &key))
                    .expect("every level is among the levels in order");
                recorder.record(&pending, Kind::Level, position_of(position));
            }
        }
    }

    /// The stop place of every quay defined beside it: the stop place naming
    /// it among its quays, else the one its `SiteRef` names. Each reference
    /// resolves as any other, a `QuayRef` to nothing a reference to nothing; a
    /// `QuayRef` naming a stop place, or a `SiteRef` naming a quay, resolves to
    /// another class than it names.
    fn resolve_quay_places(&mut self) {
        for (place, pending) in std::mem::take(&mut self.b.quay_listings) {
            if let Some(quay) = self.resolve_stop(&pending) {
                match self.b.stops[quay as usize].kind {
                    StopKind::Quay => self.claim(quay, place),
                    StopKind::StopPlace => self.check_class(&pending, "StopPlace"),
                }
            }
        }
        for (quay, pending) in std::mem::take(&mut self.b.quay_sites) {
            if let Some(site) = self.resolve_stop(&pending) {
                match self.b.stops[site as usize].kind {
                    StopKind::StopPlace => self.claim(quay.0, StopId(site)),
                    StopKind::Quay => self.check_class(&pending, "Quay"),
                }
            }
        }
    }

    /// Makes `place` the stop place of `quay`, unless another one claimed it
    /// first: a finding then, naming `place`.
    fn claim(&mut self, quay: u32, place: StopId) {
        let record = &mut self.b.stops[quay as usize];
        match record.parent {
            None => record.parent = Some(place),
            Some(kept) if kept == place => {}
            Some(_) => {
                let (id, origin) = (record.key.id, record.origin);
                let other = self.b.stops[place.0 as usize].key.id;
                self.found(
                    Concern::QuayOfTwoStopPlaces,
                    "Quay",
                    (origin, id),
                    Some(other),
                );
            }
        }
    }

    /// Stop points → stops, quay before stop place. A stop point assigned in
    /// several documents takes the assignment of the document named first;
    /// within one document, the last.
    fn resolve_assignments(&mut self) {
        let assignments = std::mem::take(&mut self.b.assignments);
        let mut assigned_by: Vec<Option<Sym>> = vec![None; self.b.stop_points.len()];
        for (point, quay, place) in &assignments {
            let Some(point_index) = self.resolve_stop_point(point) else {
                continue;
            };
            let stop = match (quay, place) {
                (Some(quay), _) => self.resolve_stop(quay),
                (None, Some(place)) => self.resolve_stop(place),
                (None, None) => continue,
            };
            if let Some(stop) = stop {
                let by = &mut assigned_by[point_index as usize];
                if by.is_none_or(|current| !precedes(&self.b.strings, current, point.origin)) {
                    *by = Some(point.origin);
                    self.b.stop_points[point_index as usize].stop = Some(StopId(stop));
                }
            }
        }
    }

    /// The findings of entities gathered once that some document defines
    /// otherwise: every content but the kept one, named by the document named
    /// first among those stating it.
    fn variant_findings(&mut self) {
        for (stop, contents) in std::mem::take(&mut self.b.stop_variants.of) {
            let record = &self.b.stops[stop as usize];
            let element = match record.kind {
                StopKind::StopPlace => "StopPlace",
                StopKind::Quay => "Quay",
            };
            let (kept, id, origin) = (record.content, record.key.id, record.origin);
            for (content, document) in contents {
                if content != kept {
                    self.redefined(element, id, document, origin);
                }
            }
        }
        for (point, contents) in std::mem::take(&mut self.b.stop_point_variants.of) {
            let record = &self.b.stop_points[point as usize];
            let (kept, id, origin) = (record.content, record.key.id, record.origin);
            for (content, document) in contents {
                if content != kept {
                    self.redefined("ScheduledStopPoint", id, document, origin);
                }
            }
        }
        for (key, contents) in std::mem::take(&mut self.b.level_variants.of) {
            let LevelRecord { name, origin } = self.b.levels[&key];
            let kept = level_fingerprint(self.b.strings.resolve(name));
            for (content, document) in contents {
                if content != kept {
                    self.redefined("Level", key.id, document, origin);
                }
            }
        }
    }

    /// Puts what the documents define of their own in the order of the
    /// documents' names, keeping the order within one.
    fn sort_by_document(&mut self, ranks: &Ranks) {
        ranks.sort(&mut self.b.routes, |r| r.origin);
        ranks.sort(&mut self.b.lines, |r| r.origin);
        ranks.sort(&mut self.b.patterns, |r| r.origin);
        ranks.sort(&mut self.b.day_types, |r| r.origin);
        ranks.sort(&mut self.b.periods, |r| r.origin);
        ranks.sort(&mut self.b.conditions, |r| r.origin);
        ranks.sort(&mut self.b.operating_days, |r| r.origin);
        ranks.sort(&mut self.b.quay_listings, |(_, pending)| pending.origin);
        ranks.sort(&mut self.b.quay_sites, |(_, pending)| pending.origin);
        // Assignments apply in the order of their documents' names, within a
        // document in their order.
        self.b
            .day_type_assignments
            .sort_by_key(|r| (ranks.of_document(r.day_type.origin), r.rank));
        let journeys = &self.b.journeys;
        self.journey_order =
            (0..u32::try_from(journeys.len()).expect("fewer than 2^32 journeys")).collect();
        self.journey_order
            .sort_by_key(|&i| ranks.of_document(journeys[i as usize].origin));
    }

    /// The recorder of references, every definition numbered: kind by kind,
    /// each in the order the index takes it in, the levels in the order of
    /// their identifiers and versions.
    fn definitions(&mut self) -> Recorder {
        let mut recorder = Recorder::default();
        let b = &self.b;
        recorder.begin(Kind::Stop);
        for stop in &b.stops {
            let element = match stop.kind {
                StopKind::StopPlace => "StopPlace",
                StopKind::Quay => "Quay",
            };
            recorder.define(element, stop.key.id, None);
        }
        recorder.begin(Kind::StopPoint);
        for point in &b.stop_points {
            recorder.define("ScheduledStopPoint", point.key.id, None);
        }
        let mut levels: Vec<Key> = b.levels.keys().copied().collect();
        levels.sort_unstable_by(|a, b| level_order(&self.b.strings, a, b));
        recorder.begin(Kind::Level);
        for key in &levels {
            recorder.define("Level", key.id, None);
        }
        self.level_order = levels;
        let b = &self.b;
        recorder.begin(Kind::Route);
        for route in &b.routes {
            recorder.define("Route", route.key.id, Some(route.origin));
        }
        recorder.begin(Kind::Line);
        for line in &b.lines {
            let element = if line.flexible {
                "FlexibleLine"
            } else {
                "Line"
            };
            recorder.define(element, line.key.id, Some(line.origin));
        }
        recorder.begin(Kind::Pattern);
        for pattern in &b.patterns {
            recorder.define(
                "ServiceJourneyPattern",
                pattern.key.id,
                Some(pattern.origin),
            );
        }
        recorder.begin(Kind::Journey);
        for &journey in &self.journey_order {
            let journey = &b.journeys[journey as usize];
            recorder.define("ServiceJourney", journey.key.id, Some(journey.origin));
        }
        recorder.begin(Kind::DayType);
        for day_type in &b.day_types {
            recorder.define("DayType", day_type.key.id, Some(day_type.origin));
        }
        recorder.begin(Kind::Period);
        for period in &b.periods {
            recorder.define(period.element, period.key.id, Some(period.origin));
        }
        recorder.begin(Kind::Condition);
        for condition in &b.conditions {
            recorder.define(
                "AvailabilityCondition",
                condition.key.id,
                Some(condition.origin),
            );
        }
        recorder.begin(Kind::OperatingDay);
        for day in &b.operating_days {
            recorder.define("OperatingDay", day.key.id, Some(day.origin));
        }
        recorder
    }

    /// The scopes, over the definitions in the order of their documents' names.
    fn scopes(&self) -> Scopes {
        let b = &self.b;
        // Validity counts only where references are resolved by the day.
        let dated = b.at.is_some();
        let scope = |kind: Kind, definitions: Vec<(Key, Sym)>, validity: Vec<Validity>| Scope {
            kind,
            table: Scoped::build(&b.strings, &definitions),
            validity: if dated { validity } else { Vec::new() },
        };
        let journeys = self.journey_order.iter().map(|&i| &b.journeys[i as usize]);
        Scopes {
            routes: scope(
                Kind::Route,
                b.routes.iter().map(|r| (r.key, r.origin)).collect(),
                b.routes.iter().map(|r| r.validity).collect(),
            ),
            lines: scope(
                Kind::Line,
                b.lines.iter().map(|r| (r.key, r.origin)).collect(),
                b.lines.iter().map(|r| r.validity).collect(),
            ),
            patterns: scope(
                Kind::Pattern,
                b.patterns.iter().map(|r| (r.key, r.origin)).collect(),
                b.patterns.iter().map(|r| r.validity).collect(),
            ),
            day_types: scope(
                Kind::DayType,
                b.day_types.iter().map(|r| (r.key, r.origin)).collect(),
                Vec::new(),
            ),
            periods: scope(
                Kind::Period,
                b.periods.iter().map(|r| (r.key, r.origin)).collect(),
                Vec::new(),
            ),
            conditions: scope(
                Kind::Condition,
                b.conditions.iter().map(|r| (r.key, r.origin)).collect(),
                Vec::new(),
            ),
            operating_days: scope(
                Kind::OperatingDay,
                b.operating_days.iter().map(|r| (r.key, r.origin)).collect(),
                Vec::new(),
            ),
            journeys: scope(
                Kind::Journey,
                journeys.clone().map(|r| (r.key, r.origin)).collect(),
                journeys.map(|r| r.validity).collect(),
            ),
        }
    }

    /// The line each definition is in the index: a definition repeating an
    /// earlier one of its key alike is that line, one stating it otherwise a
    /// line of its own and a finding.
    fn lines(&mut self, scopes: &Scopes) -> Lines {
        let mut lines = Lines {
            of_position: Vec::with_capacity(self.b.lines.len()),
            kept: Vec::new(),
            versions: FxHashMap::default(),
        };
        for (position, record) in self.b.lines.iter().enumerate() {
            lines
                .versions
                .entry((record.key.id, record.origin))
                .or_default()
                .push(position_of(position));
        }
        let mut variants: FxHashMap<Key, Vec<LineId>> = FxHashMap::default();
        for position in 0..self.b.lines.len() {
            let record = &self.b.lines[position];
            let first = first_of(&scopes.lines.table, record.key);
            if first != position {
                let candidates = variants
                    .entry(record.key)
                    .or_insert_with(|| vec![lines.of_position[first]]);
                let kept = &lines.kept;
                let defined = &self.b.lines;
                let same = candidates
                    .iter()
                    .copied()
                    .find(|line| defined[kept[line.0 as usize] as usize].says_the_same_as(record));
                if let Some(same) = same {
                    self.b.redefinitions += 1;
                    lines.of_position.push(same);
                    continue;
                }
                candidates.push(next_id(lines.kept.len(), LineId));
                let (id, origin, about) =
                    (record.key.id, record.origin, self.b.lines[first].origin);
                self.redefined("Line", id, origin, about);
            }
            lines.of_position.push(next_id(lines.kept.len(), LineId));
            lines.kept.push(position_of(position));
        }
        lines
    }

    /// The line definition each route names, by position, resolved in the
    /// route's document first. A route another document defines naming
    /// another line is a finding.
    fn routes(&mut self, scopes: &Scopes) -> Vec<Option<u32>> {
        let mut lines = Vec::with_capacity(self.b.routes.len());
        for position in 0..self.b.routes.len() {
            let record = self.b.routes[position].clone();
            let first = first_of(&scopes.routes.table, record.key);
            if first != position {
                let other = &self.b.routes[first];
                if record.line.map(|p| p.key) != other.line.map(|p| p.key) {
                    let about = other.origin;
                    self.redefined("Route", record.key.id, record.origin, about);
                }
            }
            lines.push(match &record.line {
                Some(pending) => self.resolve_in(&scopes.lines, pending),
                None => None,
            });
        }
        lines
    }

    /// The line definition, by position, of the route `pending` resolves to,
    /// in its own document first.
    fn line_of_route(
        &mut self,
        scopes: &Scopes,
        routes: &[Option<u32>],
        pending: &Pending,
    ) -> Option<u32> {
        let route = self.resolve_in(&scopes.routes, pending)?;
        routes[route as usize]
    }

    /// The patterns with their calls resolved and the line each names: the
    /// line its route view names, else the line of the route it names. A
    /// point the vehicle passes through is no call and a finding; a point at a
    /// stop point no document defines is no call either, its reference the
    /// finding. A pattern a document states otherwise than the one the
    /// delivery answers with is a finding.
    fn patterns(&mut self, scopes: &Scopes, routes: &[Option<u32>]) -> Patterns {
        let count = self.b.patterns.len();
        let points = self.b.pattern_point_refs.len();
        let mut patterns = Patterns {
            indexed: Vec::with_capacity(count),
            calls: CallLists {
                stop_points: Vec::with_capacity(points),
                uses: Vec::with_capacity(points),
                of_journeys: HashMap::new(),
            },
            points_of_calls: Vec::with_capacity(points),
            points: Vec::with_capacity(count),
            lines: Vec::with_capacity(count),
        };
        for position in 0..count {
            let record = self.b.patterns[position].clone();
            let first = first_of(&scopes.patterns.table, record.key);
            if first != position {
                let other = &self.b.patterns[first];
                let points = |(start, end): (u32, u32)| {
                    let range = start as usize..end as usize;
                    let refs = self.b.pattern_point_refs[range.clone()]
                        .iter()
                        .map(|p| (p.key, p.version_ref));
                    refs.zip(&self.b.pattern_point_keys[range.clone()])
                        .zip(&self.b.pattern_point_uses[range])
                        .collect::<Vec<_>>()
                };
                let same = record.line.map(|p| p.key) == other.line.map(|p| p.key)
                    && record.route.map(|p| p.key) == other.route.map(|p| p.key)
                    && points(record.points) == points(other.points);
                if same {
                    self.b.redefinitions += 1;
                } else {
                    let about = other.origin;
                    self.redefined("ServiceJourneyPattern", record.key.id, record.origin, about);
                }
            }
            let start = position_of(patterns.calls.stop_points.len());
            for point in record.points.0..record.points.1 {
                let point_use = self.b.pattern_point_uses[point as usize];
                let pending = self.b.pattern_point_refs[point as usize];
                if point_use.passes {
                    let id = self.b.pattern_point_keys[point as usize].id;
                    self.found(
                        Concern::PassedThrough,
                        "StopPointInJourneyPattern",
                        (record.origin, id),
                        Some(record.key.id),
                    );
                    self.record_passed(&pending);
                    continue;
                }
                if let Some(stop_point) = self.resolve_stop_point(&pending) {
                    patterns.calls.stop_points.push(StopPointId(stop_point));
                    patterns.calls.uses.push(point_use.call);
                    patterns.points_of_calls.push(point);
                }
            }
            let end = position_of(patterns.calls.stop_points.len());
            let line = match (&record.line, &record.route) {
                (Some(pending), _) => self.resolve_in(&scopes.lines, pending),
                (None, Some(route)) => self.line_of_route(scopes, routes, route),
                (None, None) => None,
            };
            patterns.lines.push(line);
            patterns.points.push(record.points);
            patterns.indexed.push(IndexedPattern {
                key: record.key,
                stop_points: (start, end),
            });
        }
        patterns
    }

    /// The days of every day type definition: those its assignments give it
    /// — each resolved in the assignment's document first — and, where its
    /// properties decide its days ([`crate::calendar`]), the days of its
    /// validity on its days of the week, the assignments adding to them and
    /// taking away; and the days of every availability condition. A period,
    /// condition, operating day or day type stated otherwise by another
    /// document is a finding.
    fn calendar(&mut self, scopes: &Scopes) -> Calendar {
        self.period_bounds(scopes);
        let assignments = self.resolved_assignments(scopes);
        let by_properties = self.decided_by_properties(&assignments);
        let named_conditions = self.conditions_named_by_journeys(scopes);
        let (first_day, span) = self.span_of_days(&assignments, &by_properties, &named_conditions);
        let periods = bits_of_periods(&self.b.periods, (first_day, span), &mut self.findings);
        let conditions = bits_of_periods(&self.b.conditions, (first_day, span), &mut self.findings);
        let mut week = WeekBits::new(first_day, span);
        let mut bits: Vec<DayBits> = self
            .b
            .day_types
            .iter()
            .zip(&by_properties)
            .map(
                |(record, &decided)| match (decided, record.properties, record.validity) {
                    (true, Properties::DaysOfWeek(days), Some((from, to))) => {
                        let mut own = week.of(days).clone();
                        own.intersect(&span_bits(first_day, span, from, to));
                        own
                    }
                    _ => DayBits::new(span),
                },
            )
            .collect();
        for assignment in &assignments {
            let day_type = assignment.day_type as usize;
            let period = match assignment.days {
                AssignmentDays::Day(day) => {
                    if let Ok(offset) = usize::try_from(day - first_day) {
                        bits[day_type].set(offset, assignment.available);
                    }
                    continue;
                }
                AssignmentDays::Period(period) => period as usize,
            };
            // A plain period gives the days of the week the day type names.
            let named = match (
                &self.b.periods[period].bits,
                self.b.day_types[day_type].properties,
            ) {
                (None, Properties::DaysOfWeek(named)) => Some(named),
                _ => None,
            };
            let days = match named {
                Some(named) => {
                    let mut days = periods[period].clone();
                    days.intersect(week.of(named));
                    Cow::Owned(days)
                }
                None => Cow::Borrowed(&periods[period]),
            };
            if assignment.available {
                bits[day_type].union(&days);
            } else {
                bits[day_type].subtract(&days);
            }
        }
        // Definitions another document states otherwise.
        redefined_periods(&self.b.periods, &scopes.periods.table, &mut self.findings);
        redefined_periods(
            &self.b.conditions,
            &scopes.conditions.table,
            &mut self.findings,
        );
        for position in 0..self.b.operating_days.len() {
            let record = &self.b.operating_days[position];
            let first = first_of(&scopes.operating_days.table, record.key);
            if first != position && record.day != self.b.operating_days[first].day {
                let (id, origin) = (record.key.id, record.origin);
                let about = self.b.operating_days[first].origin;
                self.redefined("OperatingDay", id, origin, about);
            }
        }
        for position in 0..self.b.day_types.len() {
            let record = &self.b.day_types[position];
            let first = first_of(&scopes.day_types.table, record.key);
            if first != position && bits[position] != bits[first] {
                let (id, origin) = (record.key.id, record.origin);
                let about = self.b.day_types[first].origin;
                self.redefined("DayType", id, origin, about);
            }
        }
        Calendar {
            first_day,
            span,
            day_types: bits,
            conditions,
        }
    }

    /// The first and last day of each operating period naming them as
    /// operating days, the references resolved in the period's document
    /// first; one that does not resolve leaves the period without that day.
    fn period_bounds(&mut self, scopes: &Scopes) {
        for position in 0..self.b.periods.len() {
            let [first, last] = self.b.periods[position].bounds;
            let day_of = |work: &mut Finish, bound: Option<Pending>| {
                let index = work.resolve_in(&scopes.operating_days, &bound?)?;
                Some(work.b.operating_days[index as usize].day)
            };
            if first.is_some() {
                self.b.periods[position].from = day_of(self, first);
            }
            if last.is_some() {
                self.b.periods[position].to = day_of(self, last);
            }
        }
    }

    /// The day type assignments, in the order they apply, their references
    /// resolved — the day type first, then the period or operating day —;
    /// one whose references do not resolve gives no day.
    fn resolved_assignments(&mut self, scopes: &Scopes) -> Vec<ResolvedAssignment> {
        let assignments = std::mem::take(&mut self.b.day_type_assignments);
        let mut resolved = Vec::with_capacity(assignments.len());
        for assignment in &assignments {
            let Some(day_type) = self.resolve_in(&scopes.day_types, &assignment.day_type) else {
                continue;
            };
            let days = match &assignment.days {
                AssignedDaysRecord::Date(day) => AssignmentDays::Day(*day),
                AssignedDaysRecord::OperatingDay(pending) => {
                    match self.resolve_in(&scopes.operating_days, pending) {
                        Some(index) => {
                            AssignmentDays::Day(self.b.operating_days[index as usize].day)
                        }
                        None => continue,
                    }
                }
                AssignedDaysRecord::Period(pending) => {
                    match self.resolve_in(&scopes.periods, pending) {
                        Some(index) => {
                            self.check_class(pending, self.b.periods[index as usize].element);
                            AssignmentDays::Period(index)
                        }
                        None => continue,
                    }
                }
            };
            resolved.push(ResolvedAssignment {
                day_type,
                days,
                available: assignment.available,
            });
        }
        resolved
    }

    /// Whether the properties of each day type definition decide its days: a
    /// validity of a first and a last day and days of the week, and no
    /// assignment giving it a period or a day of its validity on one of those
    /// days. A day type whose properties would decide its days but name more
    /// than days of the week — a validity, and no assignment giving it a day —,
    /// or that is assigned a period without day bits, is a finding: its
    /// assignments' days stand as they are.
    fn decided_by_properties(&mut self, assignments: &[ResolvedAssignment]) -> Vec<bool> {
        let day_types = &self.b.day_types;
        let mut given_own_day = vec![false; day_types.len()];
        let mut given_any_day = vec![false; day_types.len()];
        let mut assigned_plain_period = vec![false; day_types.len()];
        for assignment in assignments {
            let day_type = assignment.day_type as usize;
            let record = &day_types[day_type];
            if let AssignmentDays::Period(period) = assignment.days {
                if self.b.periods[period as usize].bits.is_none() {
                    assigned_plain_period[day_type] = true;
                }
            }
            if !assignment.available {
                continue;
            }
            given_any_day[day_type] = true;
            given_own_day[day_type] |= match assignment.days {
                AssignmentDays::Period(_) => true,
                AssignmentDays::Day(day) => match (record.properties, record.validity) {
                    (Properties::DaysOfWeek(named), Some((from, to))) => {
                        (from..=to).contains(&day) && named.contains(day)
                    }
                    _ => false,
                },
            };
        }
        let mut decided = Vec::with_capacity(day_types.len());
        let mut unread = Vec::new();
        for (position, record) in day_types.iter().enumerate() {
            decided.push(match record.properties {
                Properties::DaysOfWeek(_) => record.validity.is_some() && !given_own_day[position],
                Properties::Unread => {
                    let relied_on = record.validity.is_some() && !given_any_day[position];
                    if relied_on || assigned_plain_period[position] {
                        unread.push((record.origin, record.key.id));
                    }
                    false
                }
                Properties::Absent => false,
            });
        }
        for entity in unread {
            self.found(Concern::PropertiesOfDayUnread, "DayType", entity, None);
        }
        decided
    }

    /// The availability conditions some journey names or states, by
    /// position, each as the journey's document resolves it. The references
    /// are looked up here without a finding: the journey's own resolution
    /// makes it.
    fn conditions_named_by_journeys(&self, scopes: &Scopes) -> Vec<u32> {
        let scope = &scopes.conditions;
        let mut named: Vec<u32> = self
            .b
            .journey_conditions
            .iter()
            .filter_map(|pending| {
                let validity = |position| scope.validity(position);
                resolve(&self.b.strings, &scope.table, validity, self.b.at, pending).ok()
            })
            .map(|resolved| resolved.index)
            .collect();
        named.sort_unstable();
        named.dedup();
        named
    }

    /// The first day of the dataset's calendar and how many days it spans:
    /// from the first to the last day that can give a journey a day — of a
    /// period an assignment names, of a condition a journey names
    /// (`named_conditions`, by position), an operating day, a day assigned, a
    /// validity of a day type its properties decide (`by_properties`, by
    /// position). A period or condition nothing names gives no journey a day,
    /// however far it reaches: the frames of one delivery state conditions
    /// up to the year 9999, which would make every calendar millions of days
    /// long.
    fn span_of_days(
        &self,
        assignments: &[ResolvedAssignment],
        by_properties: &[bool],
        named_conditions: &[u32],
    ) -> (i64, usize) {
        let mut first_day = i64::MAX;
        let mut last_day = i64::MIN;
        let mut extend = |first: i64, last: i64| {
            first_day = first_day.min(first);
            last_day = last_day.max(last);
        };
        let conditions = named_conditions
            .iter()
            .map(|&position| &self.b.conditions[position as usize]);
        for period in conditions {
            if let Some((first, last)) = period.span() {
                extend(first, last);
            }
        }
        for assignment in assignments {
            match assignment.days {
                AssignmentDays::Day(day) => extend(day, day),
                AssignmentDays::Period(period) => {
                    if let Some((first, last)) = self.b.periods[period as usize].span() {
                        extend(first, last);
                    }
                }
            }
        }
        for day in &self.b.operating_days {
            extend(day.day, day.day);
        }
        for (record, &decided) in self.b.day_types.iter().zip(by_properties) {
            if let (true, Some((first, last))) = (decided, record.validity) {
                extend(first, last);
            }
        }
        if first_day <= last_day {
            let span = usize::try_from(last_day - first_day + 1).unwrap_or(0);
            (first_day, span)
        } else {
            (0, 0)
        }
    }

    /// The days the dated journeys give the journeys they name, each dated
    /// journey's references resolved in its own document first. A dated
    /// journey that is cancelled or replaced gives no day, and takes none
    /// away: the Nordic deliveries date a journey running on a day anew
    /// beside the dated journey they replace for that day. One whose
    /// references do not both resolve gives no day either.
    fn dated_days(&mut self, scopes: &Scopes) -> DatedDays {
        let datings = std::mem::take(&mut self.b.datings);
        let mut dated = DatedDays::default();
        for dating in &datings {
            let journey = self.resolve_in(&scopes.journeys, &dating.journey);
            let day = self.resolve_in(&scopes.operating_days, &dating.day);
            if let (true, Some(journey), Some(day)) = (dating.runs, journey, day) {
                let day = self.b.operating_days[day as usize].day;
                dated.entry(journey).or_default().push(day);
            }
        }
        for days in dated.values_mut() {
            days.sort_unstable();
            days.dedup();
        }
        dated
    }

    /// Every journey: its pattern, line and calendar, each as its document
    /// resolves them, its calls with their times — its pattern's, or those it
    /// states —, and a template's headway groups. A definition repeating an
    /// earlier one of its key alike is that journey; one stating it otherwise
    /// is a journey of its own, and a finding. A journey of a flexible line,
    /// one its line version leaves no day, and one left with fewer than two
    /// timed calls are left out, each a finding.
    fn journeys(
        &mut self,
        scopes: &Scopes,
        (lines, routes): (&Lines, &[Option<u32>]),
        patterns: &mut Patterns,
        (calendar, dated): (&Calendar, &DatedDays),
    ) -> Journeys {
        let order = std::mem::take(&mut self.journey_order);
        let records = std::mem::take(&mut self.b.journeys);
        let raw = Raw {
            day_types: std::mem::take(&mut self.b.journey_day_types),
            conditions: std::mem::take(&mut self.b.journey_conditions),
            times: std::mem::take(&mut self.b.raw_times),
            calls: std::mem::take(&mut self.b.raw_calls),
            headways: std::mem::take(&mut self.b.raw_headways),
        };
        let mut journeys = Journeys {
            records: Vec::with_capacity(records.len()),
            of_position: Vec::with_capacity(records.len()),
            passing_times: Vec::with_capacity(raw.times.len() + raw.calls.len()),
            headways: Vec::new(),
            calendars: Vec::new(),
        };
        let mut calendars = Calendars::default();
        let mut variants: FxHashMap<Key, Vec<JourneyId>> = FxHashMap::default();
        let mut laid_out = LaidOut::default();
        for (position, &index) in order.iter().enumerate() {
            let record = &records[index as usize];
            let pattern = match &record.pattern {
                Some(pending) => self.resolve_in(&scopes.patterns, pending),
                None => None,
            };
            // The line it names, else the line of the route it names, else
            // its pattern's.
            let line = match (&record.line, &record.route) {
                (Some(pending), _) => self.resolve_in(&scopes.lines, pending),
                (None, Some(route)) => self.line_of_route(scopes, routes, route),
                (None, None) => pattern.and_then(|p| patterns.lines[p as usize]),
            };
            let flexible = line.is_some_and(|line| self.b.lines[line as usize].flexible);
            if record.names_flexible_line || flexible {
                let entity = (record.origin, record.key.id);
                let named = match (line, &record.line) {
                    (Some(line), _) => Some(self.b.lines[line as usize].key.id),
                    (None, Some(pending)) => Some(pending.key.id),
                    (None, None) => None,
                };
                self.found(
                    Concern::FlexibleLineJourney,
                    "ServiceJourney",
                    entity,
                    named,
                );
                journeys.of_position.push(None);
                continue;
            }
            let days = Days {
                day_types: range(&raw.day_types, record.day_types),
                conditions: range(&raw.conditions, record.conditions),
                line,
                dated: dated.get(&position_of(position)).map_or(&[], Vec::as_slice),
            };
            let calendared = self.calendar_of(
                days,
                (scopes, lines, calendar),
                &mut calendars,
                &mut journeys,
            );
            if !self.keeps_its_days(record, line, calendared) {
                journeys.of_position.push(None);
                continue;
            }
            let line = line.map(|position| lines.of_position[position as usize]);
            if record.calls.0 < record.calls.1 {
                self.lay_out_calls(record, range(&raw.calls, record.calls), &mut laid_out);
            } else {
                let times = range(&raw.times, record.raw_times);
                self.lay_out_pattern(record, pattern, patterns, times, &mut laid_out);
            }
            let Some((first, end)) = self.timed_calls(record, &laid_out.times) else {
                journeys.of_position.push(None);
                continue;
            };
            let calls = match laid_out.pattern_calls {
                Some(start) => (start + first, start + end),
                None => patterns
                    .calls
                    .of(&laid_out.calls[first as usize..end as usize]),
            };
            let times_start = position_of(journeys.passing_times.len());
            journeys
                .passing_times
                .extend_from_slice(&laid_out.times[first as usize..end as usize]);
            let headways_start = position_of(journeys.headways.len());
            for headway in range(&raw.headways, record.headways) {
                match (headway.first, headway.last, headway.interval) {
                    (Some(first_departure), Some(last_departure), Some(interval))
                        if interval > 0 =>
                    {
                        journeys.headways.push(Headway {
                            first_departure,
                            last_departure,
                            interval,
                        });
                    }
                    _ => self.found(
                        Concern::HeadwayGroupWithoutRuns,
                        "HeadwayJourneyGroup",
                        (record.origin, headway.id),
                        Some(record.key.id),
                    ),
                }
            }
            let journey = IndexedJourney {
                key: record.key,
                origin: record.origin,
                line,
                pattern,
                calls,
                calendar: calendared.index,
                times: (times_start, position_of(journeys.passing_times.len())),
                headways: (headways_start, position_of(journeys.headways.len())),
            };
            let first = first_of(&scopes.journeys.table, record.key);
            if first != position {
                let candidates = variants
                    .entry(record.key)
                    .or_insert_with(|| journeys.of_position[first].into_iter().collect());
                let same = candidates.iter().copied().find(|candidate| {
                    journeys.same(&journeys.records[candidate.0 as usize], &journey, patterns)
                });
                if let Some(same) = same {
                    self.b.redefinitions += 1;
                    journeys.passing_times.truncate(times_start as usize);
                    journeys.headways.truncate(headways_start as usize);
                    journeys.of_position.push(Some(same));
                    continue;
                }
                if let Some(&kept) = candidates.first() {
                    let about = journeys.records[kept.0 as usize].origin;
                    self.redefined("ServiceJourney", record.key.id, record.origin, about);
                }
                candidates.push(next_id(journeys.records.len(), JourneyId));
            }
            journeys
                .of_position
                .push(Some(next_id(journeys.records.len(), JourneyId)));
            journeys.records.push(journey);
        }
        journeys
    }

    /// The calendar of a journey: the days of its day types, each resolved
    /// in the journey's document first — within the days of the availability
    /// conditions it names, or those days alone where it names no day type,
    /// a condition that is not available taking its days away —, with the
    /// days dated journeys give it; of these, the days its line version
    /// governs. Journeys of the same days share one calendar.
    fn calendar_of(
        &mut self,
        days: Days<'_>,
        (scopes, lines, calendar): (&Scopes, &Lines, &Calendar),
        calendars: &mut Calendars,
        journeys: &mut Journeys,
    ) -> Calendared {
        let resolve = |work: &mut Finish, scope: &Scope, pendings: &[Pending]| {
            let mut resolved: Vec<u32> = pendings
                .iter()
                .filter_map(|pending| work.resolve_in(scope, pending))
                .collect();
            resolved.sort_unstable();
            resolved.dedup();
            resolved
        };
        let parts = (
            resolve(self, &scopes.day_types, days.day_types),
            resolve(self, &scopes.conditions, days.conditions),
            days.line,
            days.dated.to_vec(),
        );
        if let Some(known) = calendars.of_parts.get(&parts) {
            return *known;
        }
        let (day_types, conditions, line, dated) = &parts;
        let mut bits = DayBits::new(calendar.span);
        for &day_type in day_types {
            bits.union(&calendar.day_types[day_type as usize]);
        }
        let (available, unavailable): (Vec<u32>, Vec<u32>) = conditions
            .iter()
            .partition(|&&condition| self.b.conditions[condition as usize].available);
        if !available.is_empty() {
            let mut within = DayBits::new(calendar.span);
            for &condition in &available {
                within.union(&calendar.conditions[condition as usize]);
            }
            if day_types.is_empty() {
                bits = within;
            } else {
                bits.intersect(&within);
            }
        }
        for &condition in &unavailable {
            bits.subtract(&calendar.conditions[condition as usize]);
        }
        for &day in dated {
            if let Ok(offset) = usize::try_from(day - calendar.first_day) {
                bits.set(offset, true);
            }
        }
        let every = bits.count();
        if let Some(line) = *line {
            let mask = calendars
                .governed
                .entry(line)
                .or_insert_with(|| self.governed_by(line, lines, calendar));
            if let Some(mask) = mask {
                bits.intersect(mask);
            }
        }
        let kept = bits.count();
        let next = position_of(journeys.calendars.len());
        let index = *calendars.of_days.entry(bits).or_insert_with_key(|bits| {
            journeys.calendars.push(bits.clone());
            next
        });
        let calendared = Calendared { index, every, kept };
        calendars.of_parts.insert(parts, calendared);
        calendared
    }

    /// The days the line definition at `line` governs among the versions of
    /// its line ([`governed`]), as days of the calendar; none where it
    /// governs every day.
    fn governed_by(&self, line: u32, lines: &Lines, calendar: &Calendar) -> Option<DayBits> {
        let record = &self.b.lines[line as usize];
        let versions = &lines.versions[&(record.key.id, record.origin)];
        let spans: Vec<Span> = versions
            .iter()
            .map(|&version| self.b.lines[version as usize].span)
            .collect();
        let position = versions
            .iter()
            .position(|&version| version == line)
            .expect("a definition is among the versions of its line");
        match governed(&spans, position) {
            Governed::Every => None,
            days => {
                let mut mask = DayBits::new(calendar.span);
                for offset in 0..calendar.span {
                    if days.contains(calendar.first_day + offset as i64) {
                        mask.set(offset, true);
                    }
                }
                Some(mask)
            }
        }
    }

    /// Whether a journey keeps the days `calendared` gives it, with the
    /// finding of what its line version took away: a journey with no day at
    /// all is kept, one that its line version leaves no day is not.
    fn keeps_its_days(
        &mut self,
        record: &JourneyRecord,
        line: Option<u32>,
        calendared: Calendared,
    ) -> bool {
        let entity = (record.origin, record.key.id);
        let line = line.map(|line| self.b.lines[line as usize].key.id);
        if calendared.every == 0 {
            self.found(Concern::JourneyWithoutDays, "ServiceJourney", entity, None);
        } else if calendared.kept == 0 {
            self.found(
                Concern::JourneyOutsideLineVersion,
                "ServiceJourney",
                entity,
                line,
            );
            return false;
        } else if calendared.kept < calendared.every {
            self.found(
                Concern::DaysOutsideLineVersion,
                "ServiceJourney",
                entity,
                line,
            );
        }
        true
    }

    /// The calls of a journey's pattern with the journey's passing times at
    /// each, into `laid_out`: none where it states none. A passing time at a
    /// point the pattern does not have is a finding. A passing time naming no
    /// point is at the point of its position, in the order the pattern states
    /// its points; where such times are other in number than the points, that
    /// is a finding too.
    fn lay_out_pattern(
        &mut self,
        record: &JourneyRecord,
        pattern: Option<u32>,
        patterns: &Patterns,
        raw: &[RawTime],
        laid_out: &mut LaidOut,
    ) {
        laid_out.clear();
        let Some(pattern) = pattern else {
            return;
        };
        let (first_point, end_point) = patterns.points[pattern as usize];
        let keys = &self.b.pattern_point_keys[first_point as usize..end_point as usize];
        let (start, end) = patterns.indexed[pattern as usize].stop_points;
        laid_out.pattern_calls = Some(start);
        for &point in &patterns.points_of_calls[start as usize..end as usize] {
            let position = (point - first_point) as usize;
            let key = keys[position];
            // Passing times are written in pattern order; the lookup by key is
            // the fallback when they are not.
            let time = raw
                .get(position)
                .filter(|r| r.point.is_none_or(|point| point.id == key.id))
                .or_else(|| raw.iter().find(|r| r.point.is_some_and(|p| p.id == key.id)))
                .map_or(PassingTime::NONE, |r| r.time);
            laid_out.times.push(time);
        }
        let points = keys.len();
        for raw_time in raw {
            let Some(point) = raw_time.point else {
                continue;
            };
            if !keys.iter().any(|k| k.id == point.id) {
                let strings = &self.b.strings;
                self.b.unresolved.push(Unresolved {
                    origin: strings.resolve(record.origin).to_owned(),
                    referrer: format!("ServiceJourney {}", strings.resolve(record.key.id)),
                    element: "StopPointInJourneyPatternRef".to_owned(),
                    id: strings.resolve(point.id).to_owned(),
                    version: point.version.map(|v| strings.resolve(v).to_owned()),
                });
            }
        }
        if raw.iter().any(|r| r.point.is_none()) && raw.len() != points {
            let entity = (record.origin, record.key.id);
            let about = Some(patterns.indexed[pattern as usize].key.id);
            self.found(
                Concern::PassingTimesOtherThanPoints,
                "ServiceJourney",
                entity,
                about,
            );
        }
    }

    /// The calls a journey states, with their times, into `laid_out`: a call
    /// the vehicle passes through is none and a finding, a call at a stop
    /// point no document defines none either, its reference the finding.
    fn lay_out_calls(&mut self, record: &JourneyRecord, calls: &[RawCall], laid_out: &mut LaidOut) {
        laid_out.clear();
        for call in calls {
            if call.point_use.passes {
                let entity = (record.origin, call.id);
                self.found(Concern::PassedThrough, "Call", entity, Some(record.key.id));
                self.record_passed(&call.stop_point);
                continue;
            }
            if let Some(stop_point) = self.resolve_stop_point(&call.stop_point) {
                laid_out
                    .calls
                    .push((StopPointId(stop_point), call.point_use.call));
                laid_out.times.push(call.time);
            }
        }
    }

    /// The calls a journey makes of those `laid_out` gives it: from the first
    /// with a passing time to the last, the calls before and after a finding.
    /// None, and a finding, for a journey with fewer than two timed calls.
    fn timed_calls(
        &mut self,
        record: &JourneyRecord,
        laid_out: &[PassingTime],
    ) -> Option<(u32, u32)> {
        let timed = |time: &PassingTime| time.arrival.is_some() || time.departure.is_some();
        let entity = (record.origin, record.key.id);
        let first = laid_out.iter().position(timed);
        let last = laid_out.iter().rposition(timed);
        let (Some(first), Some(last)) = (first, last) else {
            self.found(Concern::TooFewTimedCalls, "ServiceJourney", entity, None);
            return None;
        };
        if first == last {
            self.found(Concern::TooFewTimedCalls, "ServiceJourney", entity, None);
            return None;
        }
        if first > 0 || last + 1 < laid_out.len() {
            self.found(Concern::UntimedEnds, "ServiceJourney", entity, None);
        }
        Some((position_of(first), position_of(last + 1)))
    }

    /// The walks, by the stop they start at, each stop's sorted and without
    /// repetitions: the offsets of each stop's walks, and the walks.
    fn transfers(&mut self) -> (Vec<u32>, Vec<Transfer>) {
        let transfers = std::mem::take(&mut self.b.transfers);
        let mut by_stop: Vec<Vec<Transfer>> = vec![Vec::new(); self.b.stops.len()];
        for transfer in &transfers {
            let from = self.resolve_stop(&transfer.from[0]);
            let to = self.resolve_stop(&transfer.to[0]);
            let (Some(from), Some(to)) = (from, to) else {
                continue;
            };
            if from == to {
                continue;
            }
            by_stop[from as usize].push(Transfer {
                from: StopId(from),
                to: StopId(to),
                seconds: transfer.seconds,
                both_ways: transfer.both_ways,
            });
            if transfer.both_ways {
                by_stop[to as usize].push(Transfer {
                    from: StopId(to),
                    to: StopId(from),
                    seconds: transfer.seconds,
                    both_ways: true,
                });
            }
        }
        let mut offsets = Vec::with_capacity(self.b.stops.len() + 1);
        let mut all = Vec::new();
        offsets.push(0u32);
        for mut list in by_stop {
            list.sort_by_key(|t| (t.to, t.seconds));
            list.dedup();
            all.extend(list);
            offsets.push(u32::try_from(all.len()).expect("fewer than 2^32 transfers"));
        }
        (offsets, all)
    }

    /// The interchanges, by the stop point the feeder arrives at: the offsets
    /// of each stop point's interchanges, and the interchanges. A journey is
    /// resolved in the interchange's own document first, as every reference;
    /// an interchange with a journey the index leaves out is left out with it.
    fn interchanges(
        &mut self,
        scopes: &Scopes,
        journeys: &Journeys,
    ) -> (Vec<u32>, Vec<Interchange>) {
        let interchanges = std::mem::take(&mut self.b.interchanges);
        let mut by_point: Vec<Vec<Interchange>> = vec![Vec::new(); self.b.stop_points.len()];
        let journey = |work: &mut Finish, pending: &Pending| {
            work.resolve_in(&scopes.journeys, pending)
                .and_then(|position| journeys.of_position[position as usize])
        };
        for record in &interchanges {
            let from_journey = journey(self, &record.from_journey);
            let to_journey = journey(self, &record.to_journey);
            let from_point = self.resolve_stop_point(&record.from_stop_point);
            let to_point = self.resolve_stop_point(&record.to_stop_point);
            let (Some(from_journey), Some(to_journey), Some(from_point), Some(to_point)) =
                (from_journey, to_journey, from_point, to_point)
            else {
                continue;
            };
            by_point[from_point as usize].push(Interchange {
                from_journey,
                to_journey,
                from_stop_point: StopPointId(from_point),
                to_stop_point: StopPointId(to_point),
                stay_seated: record.stay_seated,
                guaranteed: record.guaranteed,
                maximum_wait_seconds: record.maximum_wait_seconds,
            });
        }
        let mut offsets = Vec::with_capacity(self.b.stop_points.len() + 1);
        let mut all = Vec::new();
        offsets.push(0u32);
        for list in by_point {
            all.extend(list);
            offsets.push(u32::try_from(all.len()).expect("fewer than 2^32 interchanges"));
        }
        (offsets, all)
    }

    /// The stops by DHID: of several stops carrying one, the one whose
    /// identifier sorts first, at its highest version.
    fn stops_by_dhid(&self) -> FxHashMap<Sym, StopId> {
        let mut stop_by_dhid: FxHashMap<Sym, StopId> = FxHashMap::default();
        for (index, stop) in self.b.stops.iter().enumerate() {
            let Some(dhid) = stop.dhid else { continue };
            let found = StopId(position_of(index));
            match stop_by_dhid.entry(dhid) {
                Entry::Vacant(slot) => {
                    slot.insert(found);
                }
                Entry::Occupied(mut slot) => {
                    let held = self.b.stops[slot.get().0 as usize].key;
                    let strings = &self.b.strings;
                    let (id, held_id) = (strings.resolve(stop.key.id), strings.resolve(held.id));
                    if id < held_id
                        || (id == held_id
                            && version_is_higher(strings, stop.key.version, held.version))
                    {
                        slot.insert(found);
                    }
                }
            }
        }
        stop_by_dhid
    }
}

impl Journeys {
    /// Whether two journeys of one key say the same: line, pattern, calls,
    /// calendar, passing times and headway groups.
    fn same(&self, a: &IndexedJourney, b: &IndexedJourney, patterns: &Patterns) -> bool {
        let pattern =
            |journey: &IndexedJourney| journey.pattern.map(|p| patterns.indexed[p as usize].key);
        let calls = |journey: &IndexedJourney| {
            (
                range(&patterns.calls.stop_points, journey.calls),
                range(&patterns.calls.uses, journey.calls),
            )
        };
        let times = |journey: &IndexedJourney| range(&self.passing_times, journey.times);
        let headways = |journey: &IndexedJourney| range(&self.headways, journey.headways);
        (a.line, pattern(a), a.calendar) == (b.line, pattern(b), b.calendar)
            && calls(a) == calls(b)
            && times(a) == times(b)
            && headways(a) == headways(b)
    }
}

/// The order levels are numbered in where references are recorded: by
/// identifier, then version, each by its text.
fn level_order(strings: &Strings, a: &Key, b: &Key) -> std::cmp::Ordering {
    let text = |key: &Key| {
        (
            strings.resolve(key.id),
            key.version.map(|version| strings.resolve(version)),
        )
    };
    text(a).cmp(&text(b))
}

/// The journeys of each line: the offsets of each line's journeys, and the journeys.
fn by_line(journeys: &[IndexedJourney], lines: usize) -> (Vec<u32>, Vec<JourneyId>) {
    let mut by_line: Vec<Vec<JourneyId>> = vec![Vec::new(); lines];
    for (index, journey) in journeys.iter().enumerate() {
        if let Some(line) = journey.line {
            by_line[line.0 as usize].push(JourneyId(position_of(index)));
        }
    }
    let mut offsets = Vec::with_capacity(lines + 1);
    let mut all = Vec::with_capacity(journeys.len());
    offsets.push(0u32);
    for list in by_line {
        all.extend(list);
        offsets.push(position_of(all.len()));
    }
    (offsets, all)
}

/// The entries `range` names in `list`.
fn range<T>(list: &[T], (start, end): (u32, u32)) -> &[T] {
    &list[start as usize..end as usize]
}

/// The position of the definition the delivery answers with for `key`: the
/// first of the key, in the order of the documents' names.
fn first_of(scope: &Scoped, key: Key) -> usize {
    scope
        .of_delivery(key)
        .expect("every definition is in the scope built over the definitions") as usize
}

/// A position as the index stores it.
fn position_of(position: usize) -> u32 {
    u32::try_from(position).expect("fewer than 2^32 entities of a kind")
}

/// The identifier of the next entity of a list `len` long.
fn next_id<T>(len: usize, id: impl Fn(u32) -> T) -> T {
    id(position_of(len))
}

/// The rank of each document by its name: the order the index takes what the
/// documents define of their own in.
struct Ranks(FxHashMap<Sym, u32>);

impl Ranks {
    fn of(strings: &Strings, documents: &[Sym]) -> Ranks {
        let mut names: Vec<Sym> = documents.to_vec();
        names.sort_unstable_by(|a, b| strings.resolve(*a).cmp(strings.resolve(*b)));
        names.dedup();
        Ranks(
            names
                .into_iter()
                .enumerate()
                .map(|(rank, name)| (name, position_of(rank)))
                .collect(),
        )
    }

    /// The rank of `document`; a name no document was added under sorts last.
    fn of_document(&self, document: Sym) -> u32 {
        self.0.get(&document).copied().unwrap_or(u32::MAX)
    }

    /// Sorts records by the rank of their document, keeping the order within one.
    fn sort<T>(&self, records: &mut [T], document: impl Fn(&T) -> Sym) {
        records.sort_by_key(|record| self.of_document(document(record)));
    }
}

/// The days of each period or condition as days of the calendar starting on
/// `first_day` and spanning `span` days, with a finding for what was off
/// about one.
fn bits_of_periods(
    records: &[PeriodRecord],
    (first_day, span): (i64, usize),
    findings: &mut Vec<FindingRecord>,
) -> Vec<DayBits> {
    records
        .iter()
        .map(|record| {
            let PeriodDays { days, issue } = record.days();
            let concern = issue.map(|issue| match issue {
                PeriodIssue::NoFirstDay => Concern::PeriodWithoutFirstDay,
                PeriodIssue::BitsOtherThanDays { .. } => Concern::DayBitsLengthDiffers,
            });
            if let Some(concern) = concern {
                findings.push(FindingRecord {
                    concern,
                    element: record.element,
                    document: record.origin,
                    entity: record.key.id,
                    about: None,
                });
            }
            let mut bits = DayBits::new(span);
            for offset in days
                .into_iter()
                .filter_map(|day| usize::try_from(day - first_day).ok())
                .filter(|&offset| offset < span)
            {
                bits.set(offset, true);
            }
            bits
        })
        .collect()
}

/// The findings of periods or conditions a document states otherwise than
/// the one the delivery answers with.
fn redefined_periods(records: &[PeriodRecord], scope: &Scoped, findings: &mut Vec<FindingRecord>) {
    for (position, record) in records.iter().enumerate() {
        let first = first_of(scope, record.key);
        if first != position && !record.says_the_same_as(&records[first]) {
            findings.push(FindingRecord {
                concern: Concern::RedefinedOtherwise,
                element: record.element,
                document: record.origin,
                entity: record.key.id,
                about: Some(records[first].origin),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::index::IndexBuilder;
    use crate::model::Date;

    /// A delivery whose composite frame states a condition from 1900 to 9999
    /// that no journey names, and a journey running on the weekdays of an
    /// October period.
    const FAR_REACHING_FRAME: &str = r#"<PublicationDelivery xmlns="http://www.netex.org.uk/netex" version="1.15:NO-NeTEx-networktimetable:1.5">
  <PublicationTimestamp>2026-10-01T00:00:00</PublicationTimestamp>
  <ParticipantRef>EX</ParticipantRef>
  <dataObjects>
    <CompositeFrame version="1" id="EX:CompositeFrame:1">
      <validityConditions>
        <AvailabilityCondition version="1" id="EX:AvailabilityCondition:always">
          <FromDate>1900-01-01T00:00:00</FromDate>
          <ToDate>9999-12-31T00:00:00</ToDate>
        </AvailabilityCondition>
      </validityConditions>
      <frames>
        <ServiceCalendarFrame version="1" id="EX:ServiceCalendarFrame:1">
          <dayTypes>
            <DayType version="1" id="EX:DayType:weekdays">
              <properties><PropertyOfDay><DaysOfWeek>Monday Tuesday Wednesday Thursday Friday</DaysOfWeek></PropertyOfDay></properties>
            </DayType>
          </dayTypes>
          <operatingPeriods>
            <OperatingPeriod version="1" id="EX:OperatingPeriod:october">
              <FromDate>2026-10-01T00:00:00</FromDate>
              <ToDate>2026-10-31T00:00:00</ToDate>
            </OperatingPeriod>
          </operatingPeriods>
          <dayTypeAssignments>
            <DayTypeAssignment order="1" version="1" id="EX:DayTypeAssignment:1">
              <OperatingPeriodRef ref="EX:OperatingPeriod:october"/>
              <DayTypeRef ref="EX:DayType:weekdays"/>
            </DayTypeAssignment>
          </dayTypeAssignments>
        </ServiceCalendarFrame>
        <TimetableFrame version="1" id="EX:TimetableFrame:1">
          <vehicleJourneys>
            <ServiceJourney version="1" id="EX:ServiceJourney:1">
              <dayTypes><DayTypeRef ref="EX:DayType:weekdays"/></dayTypes>
            </ServiceJourney>
          </vehicleJourneys>
        </TimetableFrame>
      </frames>
    </CompositeFrame>
  </dataObjects>
</PublicationDelivery>"#;

    /// The calendar spans the days a journey can run on — the October period
    /// assigned —, not the condition no journey names: its days could give
    /// no journey a day, and would make every calendar of the delivery
    /// three million days long.
    #[test]
    fn a_condition_no_journey_names_does_not_widen_the_calendar() {
        let document = crate::read(FAR_REACHING_FRAME).expect("the document reads");
        let mut builder = IndexBuilder::new();
        builder.add(&document, "far.xml");
        let index = builder.finish();
        let first = Date::from("2026-10-01").days_since_epoch().expect("a date");
        assert_eq!(index.first_day, first);
        assert!(
            index
                .calendars
                .iter()
                .all(|calendar| calendar.days().all(|offset| offset < 31)),
            "every day within October"
        );
    }
}
