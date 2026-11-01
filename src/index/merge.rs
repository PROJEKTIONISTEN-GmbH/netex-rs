//! Merging builders: what several workers gathered, brought into one builder
//! as if it had seen every document itself.
//!
//! Each worker interns its own strings, so the other builder's symbols are
//! renumbered into the receiving builder's interner first; every record is
//! then appended with its symbols and its ranges into the flat arrays
//! translated. An entity whose key — and, for a stop or a stop point, name —
//! the receiving builder already holds is a redefinition, exactly as it would
//! be had one builder read both documents: the quays a redefined stop place
//! brings are quays of the stop place that was kept, and of two contradicting
//! definitions of a stop, a stop point or a level the one from the document
//! named first is kept. Which document defines which stop is taken in as well,
//! for the stops some documents name otherwise.

use std::collections::hash_map::Entry;

use rayon::prelude::*;

use super::build::{
    level_fingerprint, precedes, AssignedDaysRecord, DayTypeAssignmentRecord, IndexBuilder,
    InterchangeRecord, Key, LevelRecord, Pending, PeriodRecord, Referrer, TransferRecord,
};
use super::strings::{Strings, Sym};
use super::StopId;

/// Where entries appended to `list` start.
fn offset_of<T>(list: &[T]) -> u32 {
    u32::try_from(list.len()).expect("fewer than 2^32 entries of a kind")
}

/// The other builder's symbols, renumbered into the receiving builder's.
struct Renumbered {
    syms: Vec<Sym>,
}

impl Renumbered {
    /// Renumbers every string of `other` into `strings`.
    ///
    /// Most of `other`'s strings are in `strings` already — the stops a line
    /// file repeats are in every worker's builder — so the lookups run on
    /// every thread of the pool first, and only the strings none found are
    /// interned, one after the other; those are distinct within `other`, so
    /// each is interned once.
    fn of(strings: &mut Strings, other: &Strings) -> Renumbered {
        let found: Vec<Option<Sym>> = other
            .as_slice()
            .par_iter()
            .map(|text| strings.get(text))
            .collect();
        let syms = found
            .into_iter()
            .zip(other.as_slice())
            .map(|(sym, text)| sym.unwrap_or_else(|| strings.intern(text)))
            .collect();
        Renumbered { syms }
    }

    fn sym(&self, sym: Sym) -> Sym {
        self.syms[sym as usize]
    }

    fn key(&self, key: Key) -> Key {
        Key {
            id: self.sym(key.id),
            version: key.version.map(|v| self.sym(v)),
        }
    }

    /// Contents noted for an entity, their documents renumbered.
    fn contents(&self, contents: Vec<(u64, Sym)>) -> Vec<(u64, Sym)> {
        contents
            .into_iter()
            .map(|(content, document)| (content, self.sym(document)))
            .collect()
    }

    fn pending(&self, pending: Pending) -> Pending {
        Pending {
            key: self.key(pending.key),
            version_ref: pending.version_ref.map(|v| self.sym(v)),
            element: pending.element,
            referrer: Referrer {
                element: pending.referrer.element,
                id: self.sym(pending.referrer.id),
            },
            origin: self.sym(pending.origin),
        }
    }
}

impl IndexBuilder {
    /// Takes in everything `other` gathered.
    ///
    /// The result is what this builder would hold had it been given `other`'s
    /// documents after its own, in `other`'s order. The instant set with
    /// [`at`](Self::at) stays this builder's; the references are kept
    /// ([`with_referrers`](Self::with_referrers)) if either builder keeps them.
    pub fn merge(&mut self, mut other: IndexBuilder) {
        self.referrers |= other.referrers;
        let renumbered = Renumbered::of(&mut self.strings, &other.strings);
        self.document_names.extend(
            other
                .document_names
                .iter()
                .map(|&name| renumbered.sym(name)),
        );
        self.redefinitions += other.redefinitions;
        self.merge_stops(&mut other, &renumbered);
        self.merge_levels(&mut other, &renumbered);
        self.merge_stop_points(&mut other, &renumbered);
        for (point, quay, place) in other.assignments {
            self.assignments.push((
                renumbered.pending(point),
                quay.map(|p| renumbered.pending(p)),
                place.map(|p| renumbered.pending(p)),
            ));
        }
        // What a document defines of its own is taken in as it is: which of
        // its definitions a reference resolves to is decided once every
        // document is in.
        for mut record in other.routes {
            record.key = renumbered.key(record.key);
            record.origin = renumbered.sym(record.origin);
            record.line = record.line.map(|p| renumbered.pending(p));
            self.routes.push(record);
        }
        for mut record in other.lines {
            record.key = renumbered.key(record.key);
            record.origin = renumbered.sym(record.origin);
            record.name = renumbered.sym(record.name);
            record.public_code = record.public_code.map(|c| renumbered.sym(c));
            record.mode = record.mode.map(|m| renumbered.sym(m));
            self.lines.push(record);
        }
        let point_offset =
            u32::try_from(self.pattern_point_refs.len()).expect("fewer than 2^32 pattern points");
        self.pattern_point_refs.extend(
            other
                .pattern_point_refs
                .into_iter()
                .map(|p| renumbered.pending(p)),
        );
        self.pattern_point_keys.extend(
            other
                .pattern_point_keys
                .into_iter()
                .map(|key| renumbered.key(key)),
        );
        self.pattern_point_uses.extend(other.pattern_point_uses);
        for mut record in other.patterns {
            record.key = renumbered.key(record.key);
            record.origin = renumbered.sym(record.origin);
            record.line = record.line.map(|p| renumbered.pending(p));
            record.route = record.route.map(|p| renumbered.pending(p));
            record.points = (
                record.points.0 + point_offset,
                record.points.1 + point_offset,
            );
            self.patterns.push(record);
        }
        let day_type_offset =
            u32::try_from(self.journey_day_types.len()).expect("fewer than 2^32 day type refs");
        self.journey_day_types.extend(
            other
                .journey_day_types
                .into_iter()
                .map(|p| renumbered.pending(p)),
        );
        let condition_offset = offset_of(&self.journey_conditions);
        self.journey_conditions.extend(
            other
                .journey_conditions
                .into_iter()
                .map(|p| renumbered.pending(p)),
        );
        let time_offset = offset_of(&self.raw_times);
        self.raw_times
            .extend(other.raw_times.into_iter().map(|mut raw| {
                raw.point = raw.point.map(|point| renumbered.key(point));
                raw
            }));
        let call_offset = offset_of(&self.raw_calls);
        self.raw_calls
            .extend(other.raw_calls.into_iter().map(|mut call| {
                call.id = renumbered.sym(call.id);
                call.stop_point = renumbered.pending(call.stop_point);
                call
            }));
        let headway_offset = offset_of(&self.raw_headways);
        self.raw_headways
            .extend(other.raw_headways.into_iter().map(|mut headway| {
                headway.id = renumbered.sym(headway.id);
                headway
            }));
        let moved = |(start, end): (u32, u32), by: u32| (start + by, end + by);
        for mut record in other.journeys {
            record.key = renumbered.key(record.key);
            record.origin = renumbered.sym(record.origin);
            record.line = record.line.map(|p| renumbered.pending(p));
            record.route = record.route.map(|p| renumbered.pending(p));
            record.pattern = record.pattern.map(|p| renumbered.pending(p));
            record.day_types = moved(record.day_types, day_type_offset);
            record.conditions = moved(record.conditions, condition_offset);
            record.calls = moved(record.calls, call_offset);
            record.headways = moved(record.headways, headway_offset);
            record.raw_times = moved(record.raw_times, time_offset);
            self.journeys.push(record);
        }
        for mut record in other.datings {
            record.journey = renumbered.pending(record.journey);
            record.day = renumbered.pending(record.day);
            self.datings.push(record);
        }
        for mut record in other.day_types {
            record.key = renumbered.key(record.key);
            record.origin = renumbered.sym(record.origin);
            self.day_types.push(record);
        }
        for record in other.day_type_assignments {
            self.day_type_assignments.push(DayTypeAssignmentRecord {
                day_type: renumbered.pending(record.day_type),
                days: match record.days {
                    AssignedDaysRecord::Period(p) => {
                        AssignedDaysRecord::Period(renumbered.pending(p))
                    }
                    AssignedDaysRecord::OperatingDay(p) => {
                        AssignedDaysRecord::OperatingDay(renumbered.pending(p))
                    }
                    AssignedDaysRecord::Date(day) => AssignedDaysRecord::Date(day),
                },
                available: record.available,
                rank: record.rank,
            });
        }
        let period = |mut record: PeriodRecord| {
            record.key = renumbered.key(record.key);
            record.origin = renumbered.sym(record.origin);
            record.bounds = record
                .bounds
                .map(|bound| bound.map(|p| renumbered.pending(p)));
            record
        };
        self.periods.extend(other.periods.into_iter().map(period));
        self.conditions
            .extend(other.conditions.into_iter().map(period));
        for mut record in other.operating_days {
            record.key = renumbered.key(record.key);
            record.origin = renumbered.sym(record.origin);
            self.operating_days.push(record);
        }
        for mut link in other.links {
            link.id = renumbered.sym(link.id);
            link.origin = renumbered.sym(link.origin);
            link.from = link.from.map(|p| renumbered.pending(p));
            self.links.push(link);
        }
        for record in other.transfers {
            self.transfers.push(TransferRecord {
                from: record
                    .from
                    .into_iter()
                    .map(|p| renumbered.pending(p))
                    .collect(),
                to: record
                    .to
                    .into_iter()
                    .map(|p| renumbered.pending(p))
                    .collect(),
                seconds: record.seconds,
                both_ways: record.both_ways,
            });
        }
        for record in other.interchanges {
            self.interchanges.push(InterchangeRecord {
                from_journey: renumbered.pending(record.from_journey),
                to_journey: renumbered.pending(record.to_journey),
                from_stop_point: renumbered.pending(record.from_stop_point),
                to_stop_point: renumbered.pending(record.to_stop_point),
                stay_seated: record.stay_seated,
                guaranteed: record.guaranteed,
                maximum_wait_seconds: record.maximum_wait_seconds,
            });
        }
        self.unresolved.extend(other.unresolved);
        for mut finding in other.gathered {
            finding.document = renumbered.sym(finding.document);
            finding.entity = renumbered.sym(finding.entity);
            finding.about = finding.about.map(|about| renumbered.sym(about));
            self.gathered.push(finding);
        }
    }

    /// Stop places and their quays. A quay refers to its stop place by
    /// index — not always an earlier one, since a later document can replace
    /// a quay's record and move it to a stop place numbered after it — so
    /// every stop of `other` is first mapped to its index here: a new stop to
    /// the index it is appended at, one this builder holds already to the
    /// stop it kept, whose record `other`'s replaces if it comes from a
    /// document named first. The quays taken in are then pointed at their
    /// places' indexes here, and the contents `other` noted for its stops
    /// are noted for the stops they are here; so are the stop places naming
    /// quays defined beside them, and the quays naming their stop place.
    fn merge_stops(&mut self, other: &mut IndexBuilder, renumbered: &Renumbered) {
        let mut moved: Vec<StopId> = Vec::with_capacity(other.stops.len());
        let mut taken_in: Vec<usize> = Vec::with_capacity(other.stops.len());
        for record in &other.stops {
            let mut record = record.clone();
            record.key = renumbered.key(record.key);
            record.origin = renumbered.sym(record.origin);
            record.dhid = record.dhid.map(|d| renumbered.sym(d));
            record.name = renumbered.sym(record.name);
            record.level_ref = record.level_ref.map(|p| renumbered.pending(p));
            record.level = record.level.map(|l| renumbered.sym(l));
            let index = u32::try_from(self.stops.len()).expect("fewer than 2^32 stops");
            match self
                .stop_table
                .insert_or_kept(record.key, record.name, index)
            {
                Ok(()) => {
                    self.stops.push(record);
                    taken_in.push(index as usize);
                    moved.push(StopId(index));
                }
                Err(kept) => {
                    self.redefinitions += 1;
                    let held = &self.stops[kept as usize];
                    self.stop_variants.note(
                        &self.strings,
                        kept,
                        (held.content, held.origin),
                        (record.content, record.origin),
                    );
                    if precedes(&self.strings, record.origin, held.origin) {
                        self.stops[kept as usize] = record;
                        taken_in.push(kept as usize);
                    }
                    moved.push(StopId(kept));
                }
            }
        }
        for index in taken_in {
            let stop = &mut self.stops[index];
            stop.parent = stop.parent.map(|parent| moved[parent.0 as usize]);
        }
        self.stop_table.absorb_definitions(
            &mut other.stop_table,
            |stop| moved[stop as usize].0,
            |document| renumbered.sym(document),
        );
        for (stop, contents) in std::mem::take(&mut other.stop_variants.of) {
            let contents = renumbered.contents(contents);
            self.stop_variants
                .absorb(&self.strings, moved[stop as usize].0, contents);
        }
        for (place, pending) in std::mem::take(&mut other.quay_listings) {
            let place = moved[place.0 as usize];
            self.quay_listings
                .push((place, renumbered.pending(pending)));
        }
        for (quay, pending) in std::mem::take(&mut other.quay_sites) {
            let quay = moved[quay.0 as usize];
            self.quay_sites.push((quay, renumbered.pending(pending)));
        }
    }

    /// Levels: a level this builder holds already is replaced by `other`'s
    /// where that comes from a document named first.
    fn merge_levels(&mut self, other: &mut IndexBuilder, renumbered: &Renumbered) {
        for (key, level) in std::mem::take(&mut other.levels) {
            let key = renumbered.key(key);
            let level = LevelRecord {
                name: renumbered.sym(level.name),
                origin: renumbered.sym(level.origin),
            };
            match self.levels.entry(key) {
                Entry::Vacant(slot) => {
                    slot.insert(level);
                }
                Entry::Occupied(mut kept) => {
                    let held = *kept.get();
                    let content =
                        |level: LevelRecord| level_fingerprint(self.strings.resolve(level.name));
                    self.level_variants.note(
                        &self.strings,
                        key,
                        (content(held), held.origin),
                        (content(level), level.origin),
                    );
                    if precedes(&self.strings, level.origin, held.origin) {
                        kept.insert(level);
                    }
                }
            }
        }
        for (key, contents) in std::mem::take(&mut other.level_variants.of) {
            let contents = renumbered.contents(contents);
            self.level_variants
                .absorb(&self.strings, renumbered.key(key), contents);
        }
    }

    /// Stop points: one this builder holds already is replaced by `other`'s
    /// where that comes from a document named first.
    fn merge_stop_points(&mut self, other: &mut IndexBuilder, renumbered: &Renumbered) {
        let mut moved: Vec<u32> = Vec::with_capacity(other.stop_points.len());
        for mut record in std::mem::take(&mut other.stop_points) {
            record.key = renumbered.key(record.key);
            record.origin = renumbered.sym(record.origin);
            record.name = renumbered.sym(record.name);
            let index = u32::try_from(self.stop_points.len()).expect("fewer than 2^32 stop points");
            match self
                .stop_point_table
                .insert_or_kept(record.key, record.name, index)
            {
                Ok(()) => {
                    self.stop_points.push(record);
                    moved.push(index);
                }
                Err(kept) => {
                    self.redefinitions += 1;
                    let held = &self.stop_points[kept as usize];
                    self.stop_point_variants.note(
                        &self.strings,
                        kept,
                        (held.content, held.origin),
                        (record.content, record.origin),
                    );
                    if precedes(&self.strings, record.origin, held.origin) {
                        self.stop_points[kept as usize] = record;
                    }
                    moved.push(kept);
                }
            }
        }
        self.stop_point_table.absorb_definitions(
            &mut other.stop_point_table,
            |point| moved[point as usize],
            |document| renumbered.sym(document),
        );
        for (point, contents) in std::mem::take(&mut other.stop_point_variants.of) {
            let contents = renumbered.contents(contents);
            self.stop_point_variants
                .absorb(&self.strings, moved[point as usize], contents);
        }
    }

    /// Merges builders into one, the largest first.
    ///
    /// The merge is sequential: a parallel pairwise merge would hold two
    /// halves and their union at once, and over the German delivery that
    /// peaked at 24 GB. Merging into the largest builder and consuming the
    /// others one by one holds the builders and, at most, one copy of the
    /// smaller one. Which definition of an entity is kept does
    /// not depend on the merge order (see [`IndexBuilder`]), so neither does
    /// the index; only the numbering of its entities does.
    pub fn merge_all(mut builders: Vec<IndexBuilder>) -> IndexBuilder {
        builders.sort_by_key(|builder| std::cmp::Reverse(builder.raw_times.len()));
        let mut builders = builders.into_iter();
        let Some(mut merged) = builders.next() else {
            return IndexBuilder::new();
        };
        // Growing the interner's table rehashes every string in it; room for
        // every string the others hold, made once, keeps that to one growth
        // rather than one per doubling — more than the merged builder will
        // need, since most of those strings it holds already.
        let others: Vec<IndexBuilder> = builders.collect();
        merged
            .strings
            .reserve(others.iter().map(|b| b.strings.len()).sum());
        for builder in others {
            merged.merge(builder);
        }
        merged
    }
}
