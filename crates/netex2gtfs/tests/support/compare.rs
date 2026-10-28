//! Two GTFS feeds of the same timetable compared day by day: the trips each
//! runs, the stations they serve, their times, and on which days — every
//! difference counted with its cause.
//!
//! A trip is compared by what it does, not by its identifier: the stations it
//! calls at, in order, with its times. The German export merges journeys of one
//! content into one trip under one of their numbers, so trips are counted per
//! day by content. A station is a stop's DHID cut to its first three parts
//! (`de:06412:1502`, `at:46:3040`), else its parent station's, else its Swiss
//! SLOID cut to its first four parts (`ch:1:sloid:3000`), else its parent
//! station's, else the first
//! run of seven or more digits in its identifier without leading zeros — the
//! Swiss and UIC station numbers —, else its parent station's, else its parent
//! station, else the stop itself. Where two feeds name one place by other
//! identifiers — a DHID in one, an internal number in the other —, a trip is
//! set against the other feed's trip of the same times and its stations are
//! compared by position. Both feeds are read from directories.

use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use rustc_hash::{FxHashMap, FxHashSet, FxHasher};
use serde::Serialize;

/// A call as compared: the station, arrival and departure in seconds since
/// the start of the service day.
pub type Call = (u32, u32, u32);

/// Two stations at most this far apart are one place named otherwise, in metres.
const ONE_PLACE_METRES: f64 = 300.0;

/// A feed read for comparison.
pub struct Feed {
    /// The stations, by number.
    stations: Vec<String>,
    /// Where each station is: the position of the first of its stops stating one.
    positions: Vec<Option<(f64, f64)>>,
    /// Per day compared, every trip running: its identifier — with `@` and
    /// the departure for a run of a frequency — and its calls.
    days: BTreeMap<String, Vec<(String, Vec<Call>)>>,
    /// The fingerprint of every trip's calls, whatever day it runs on.
    fingerprints: FxHashSet<u64>,
}

/// Where a trip starts, when it leaves there, and where it ends.
type Ends = (u32, u32, u32);

/// A trip of a day: its identifier and its calls.
type Trip<'a> = (&'a str, &'a [Call]);

/// Why a trip of one feed has no counterpart in the other on a day.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Cause {
    /// The other feed has a trip from the same station at the same time to
    /// the same last station through the same stations, at times at most a
    /// minute off at each call.
    TimesWithinAMinute,
    /// The same start, end and stations, at times more than a minute off.
    TimesDiffer,
    /// The other feed has a trip at the same times, or within a minute,
    /// calling at the same places, some named by other identifiers: within
    /// 300 m of each other.
    StationsNamedOtherwise,
    /// The same start and end, one calling at stations the other does not,
    /// the rest in the same order.
    CallsLeftOut,
    /// The same start and end, or the same times, the stations otherwise
    /// different.
    CallsDiffer,
    /// The same trip runs in the other feed, on other days.
    OtherDays,
    /// The other feed has no trip like it on any day.
    Missing,
}

/// The comparison of one day.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Day {
    /// Trips the feed compared runs.
    pub ours: u64,
    /// Trips the reference runs.
    pub reference: u64,
    /// Trips alike in both, as often as both run them.
    pub matched: u64,
    /// Trips of ours without a counterpart, by cause.
    pub only_ours: BTreeMap<Cause, u64>,
    /// Trips of the reference without a counterpart, by cause.
    pub only_reference: BTreeMap<Cause, u64>,
    /// An example of each kind of difference: the trip, and the counterpart
    /// it was set against, if any.
    pub examples: BTreeMap<String, (String, Option<String>)>,
    /// Stations served by trips of ours, of the reference, of both.
    pub stations: (u64, u64, u64),
}

impl Feed {
    /// Reads the feed in the directory `path` for the days `days` (`YYYYMMDD`).
    pub fn read(path: &Path, days: &[&str]) -> Feed {
        let mut parents: FxHashMap<String, String> = FxHashMap::default();
        let mut stop_positions: FxHashMap<String, (f64, f64)> = FxHashMap::default();
        for [stop, parent, latitude, longitude] in rows(
            path,
            "stops.txt",
            &["stop_id", "parent_station", "stop_lat", "stop_lon"],
        ) {
            if let (Ok(latitude), Ok(longitude)) = (latitude.parse(), longitude.parse()) {
                stop_positions.insert(stop.clone(), (latitude, longitude));
            }
            if !parent.is_empty() {
                parents.insert(stop, parent);
            }
        }
        let services = active_services(path, days);
        let mut trip_days: FxHashMap<String, Vec<usize>> = FxHashMap::default();
        for [trip, service] in rows(path, "trips.txt", &["trip_id", "service_id"]) {
            if let Some(active) = services.get(&service).filter(|days| !days.is_empty()) {
                trip_days.insert(trip, active.clone());
            }
        }
        // A trip run at a frequency with exact times runs once per departure;
        // one without exact times states no runs, and counts once per group,
        // at its start.
        let mut runs: FxHashMap<String, Vec<u32>> = FxHashMap::default();
        if path.join("frequencies.txt").exists() {
            for [trip, start, end, step, exact] in rows(
                path,
                "frequencies.txt",
                &[
                    "trip_id",
                    "start_time",
                    "end_time",
                    "headway_secs",
                    "exact_times",
                ],
            ) {
                let (start, end) = (
                    seconds(&start).expect("a start"),
                    seconds(&end).expect("an end"),
                );
                let step: u32 = step.parse().expect("a headway");
                let departures = runs.entry(trip).or_default();
                if exact == "1" {
                    departures.extend((start..end).step_by(step as usize));
                } else {
                    departures.push(start);
                }
            }
        }
        let mut stations = Stations::default();
        let mut feed = Feed {
            stations: Vec::new(),
            positions: Vec::new(),
            days: days
                .iter()
                .map(|day| ((*day).to_owned(), Vec::new()))
                .collect(),
            fingerprints: FxHashSet::default(),
        };
        let mut trip: Option<(String, Vec<(u32, Call)>)> = None;
        for [id, sequence, stop, arrival, departure] in rows(
            path,
            "stop_times.txt",
            &[
                "trip_id",
                "stop_sequence",
                "stop_id",
                "arrival_time",
                "departure_time",
            ],
        ) {
            let station = stations.of(&stop, &parents, &stop_positions);
            let arrival = seconds(&arrival)
                .or(seconds(&departure))
                .unwrap_or(u32::MAX);
            let departure = seconds(&departure).unwrap_or(arrival);
            let call = (
                sequence.parse().expect("a stop sequence"),
                (station, arrival, departure),
            );
            match &mut trip {
                Some((current, calls)) if *current == id => calls.push(call),
                _ => {
                    if let Some((done, calls)) = trip.take() {
                        feed.add(done, calls, &trip_days, &runs, days);
                    }
                    trip = Some((id, vec![call]));
                }
            }
        }
        if let Some((done, calls)) = trip.take() {
            feed.add(done, calls, &trip_days, &runs, days);
        }
        for trips in feed.days.values_mut() {
            trips.sort_unstable();
        }
        feed.stations = stations.names;
        feed.positions = stations.positions;
        feed
    }

    /// Takes in a trip's calls: its fingerprint, and the trip on each day
    /// compared it runs on — each run, for a trip run at a frequency.
    fn add(
        &mut self,
        trip: String,
        mut calls: Vec<(u32, Call)>,
        trip_days: &FxHashMap<String, Vec<usize>>,
        runs: &FxHashMap<String, Vec<u32>>,
        days: &[&str],
    ) {
        calls.sort_unstable_by_key(|(order, _)| *order);
        let calls: Vec<Call> = calls.into_iter().map(|(_, call)| call).collect();
        let runs_of: Vec<(String, Vec<Call>)> = match runs.get(&trip) {
            None => vec![(trip.clone(), calls)],
            Some(departures) => {
                let first = calls.first().map_or(0, |call| call.2);
                departures
                    .iter()
                    .map(|&departure| {
                        let shift = |time: u32| {
                            if time == u32::MAX {
                                time
                            } else {
                                let shifted =
                                    i64::from(time) + i64::from(departure) - i64::from(first);
                                u32::try_from(shifted).unwrap_or(0)
                            }
                        };
                        let shifted = calls
                            .iter()
                            .map(|&(s, a, d)| (s, shift(a), shift(d)))
                            .collect();
                        (format!("{trip}@{departure}"), shifted)
                    })
                    .collect()
            }
        };
        for (id, calls) in runs_of {
            self.fingerprints.insert(fingerprint(&calls));
            for &day in trip_days.get(&trip).into_iter().flatten() {
                self.days
                    .get_mut(days[day])
                    .expect("a day compared")
                    .push((id.clone(), calls.clone()));
            }
        }
    }
}

/// The two feeds' stations in one numbering — ours first, then those only the
/// reference has — with the position of each.
struct Unified {
    to_ours: Vec<u32>,
    to_reference: Vec<Option<u32>>,
    positions: Vec<Option<(f64, f64)>>,
    ours: usize,
}

impl Unified {
    fn of(ours: &Feed, reference: &Feed) -> Unified {
        let mut numbers: FxHashMap<&str, u32> = ours
            .stations
            .iter()
            .enumerate()
            .map(|(number, name)| (name.as_str(), number as u32))
            .collect();
        let mut positions = ours.positions.clone();
        let to_ours: Vec<u32> = reference
            .stations
            .iter()
            .zip(&reference.positions)
            .map(|(name, &position)| {
                let next = numbers.len() as u32;
                *numbers.entry(name.as_str()).or_insert_with(|| {
                    positions.push(position);
                    next
                })
            })
            .collect();
        let in_reference: FxHashMap<&str, u32> = reference
            .stations
            .iter()
            .enumerate()
            .map(|(number, name)| (name.as_str(), number as u32))
            .collect();
        let to_reference = ours
            .stations
            .iter()
            .map(|name| in_reference.get(name.as_str()).copied())
            .collect();
        Unified {
            to_ours,
            to_reference,
            positions,
            ours: ours.stations.len(),
        }
    }

    /// Whether two stations are one place: one station, or within 300 m.
    fn one_place(&self, a: u32, b: u32) -> bool {
        a == b
            || matches!(
                (self.positions[a as usize], self.positions[b as usize]),
                (Some(x), Some(y)) if netex_rs::geometry::metres_between(x, y) <= ONE_PLACE_METRES
            )
    }
}

/// Compares `ours` with `reference`, day by day.
pub fn compare(ours: &Feed, reference: &Feed) -> BTreeMap<String, Day> {
    let unified = Unified::of(ours, reference);
    ours.days
        .keys()
        .map(|day| {
            let mine = &ours.days[day];
            let theirs: Vec<(String, Vec<Call>)> = reference.days[day]
                .iter()
                .map(|(id, calls)| {
                    let calls = calls
                        .iter()
                        .map(|&(s, a, d)| (unified.to_ours[s as usize], a, d))
                        .collect();
                    (id.clone(), calls)
                })
                .collect();
            let elsewhere_in_reference = |calls: &[Call]| {
                let mapped: Option<Vec<Call>> = calls
                    .iter()
                    .map(|&(s, a, d)| Some((unified.to_reference[s as usize]?, a, d)))
                    .collect();
                mapped.is_some_and(|calls| reference.fingerprints.contains(&fingerprint(&calls)))
            };
            let elsewhere_in_ours = |calls: &[Call]| {
                calls.iter().all(|&(s, _, _)| (s as usize) < unified.ours)
                    && ours.fingerprints.contains(&fingerprint(calls))
            };
            let day_result = compare_day(
                mine,
                &theirs,
                &unified,
                &elsewhere_in_reference,
                &elsewhere_in_ours,
            );
            (day.clone(), day_result)
        })
        .collect()
}

fn compare_day(
    mine: &[(String, Vec<Call>)],
    theirs: &[(String, Vec<Call>)],
    unified: &Unified,
    elsewhere_in_reference: &dyn Fn(&[Call]) -> bool,
    elsewhere_in_ours: &dyn Fn(&[Call]) -> bool,
) -> Day {
    let mut result = Day {
        ours: mine.len() as u64,
        reference: theirs.len() as u64,
        ..Day::default()
    };
    let mut pool: HashMap<&[Call], Vec<&str>> = HashMap::new();
    for (id, calls) in theirs {
        pool.entry(calls.as_slice()).or_default().push(id);
    }
    let mut unmatched_ours: Vec<(&str, &[Call])> = Vec::new();
    for (id, calls) in mine {
        match pool.get_mut(calls.as_slice()).and_then(|ids| ids.pop()) {
            Some(_) => result.matched += 1,
            None => unmatched_ours.push((id, calls)),
        }
    }
    let mut unmatched_reference: Vec<(&str, &[Call])> = pool
        .into_iter()
        .flat_map(|(calls, ids)| ids.into_iter().map(move |id| (id, calls)))
        .collect();
    unmatched_reference.sort_unstable();
    result.only_ours = classify(
        &unmatched_ours,
        &unmatched_reference,
        unified,
        elsewhere_in_reference,
        &mut result.examples,
        "ours",
    );
    result.only_reference = classify(
        &unmatched_reference,
        &unmatched_ours,
        unified,
        elsewhere_in_ours,
        &mut result.examples,
        "reference",
    );
    let served = |trips: &[(String, Vec<Call>)]| -> FxHashSet<u32> {
        trips
            .iter()
            .flat_map(|(_, calls)| calls.iter().map(|call| call.0))
            .collect()
    };
    let (a, b) = (served(mine), served(theirs));
    result.stations = (
        a.len() as u64,
        b.len() as u64,
        a.intersection(&b).count() as u64,
    );
    result
}

/// The causes of the trips of one feed without a counterpart in the other,
/// `other` being the other feed's unmatched trips of the day: set against a
/// trip of the same start, departure and end, else one of the same times,
/// else looked for on other days.
fn classify(
    unmatched: &[(&str, &[Call])],
    other: &[(&str, &[Call])],
    unified: &Unified,
    elsewhere: &dyn Fn(&[Call]) -> bool,
    examples: &mut BTreeMap<String, (String, Option<String>)>,
    side: &str,
) -> BTreeMap<Cause, u64> {
    let mut by_ends: HashMap<Ends, Vec<Trip>> = HashMap::new();
    let mut by_times: HashMap<Vec<(u32, u32)>, Vec<Trip>> = HashMap::new();
    for &(id, calls) in other {
        if let (Some(first), Some(last)) = (calls.first(), calls.last()) {
            by_ends
                .entry((first.0, first.2, last.0))
                .or_default()
                .push((id, calls));
        }
        by_times.entry(times(calls)).or_default().push((id, calls));
    }
    let mut counts = BTreeMap::new();
    for &(id, calls) in unmatched {
        let (first, last) = (calls[0], calls[calls.len() - 1]);
        let counterpart = by_ends
            .get(&(first.0, first.2, last.0))
            .or_else(|| by_times.get(&times(calls)))
            .and_then(|candidates| candidates.first());
        let (cause, counterpart) = match counterpart {
            Some(&(other_id, other_calls)) => {
                (cause_between(calls, other_calls, unified), Some(other_id))
            }
            None if elsewhere(calls) => (Cause::OtherDays, None),
            None => (Cause::Missing, None),
        };
        *counts.entry(cause).or_insert(0) += 1;
        examples
            .entry(format!("{side} {cause:?}"))
            .or_insert_with(|| (id.to_owned(), counterpart.map(str::to_owned)));
    }
    counts
}

/// The times of a trip's calls, for setting it against another of the same.
fn times(calls: &[Call]) -> Vec<(u32, u32)> {
    calls
        .iter()
        .map(|&(_, arrival, departure)| (arrival, departure))
        .collect()
}

/// Why two trips set against each other differ.
fn cause_between(a: &[Call], b: &[Call], unified: &Unified) -> Cause {
    let within_a_minute = |x: &Call, y: &Call| x.1.abs_diff(y.1) <= 60 && x.2.abs_diff(y.2) <= 60;
    if a.len() == b.len() {
        let same = a.iter().zip(b).all(|(x, y)| x.0 == y.0);
        let one_place = a.iter().zip(b).all(|(x, y)| unified.one_place(x.0, y.0));
        let close = a.iter().zip(b).all(|(x, y)| within_a_minute(x, y));
        match (same, one_place, close) {
            (true, _, true) => return Cause::TimesWithinAMinute,
            (true, _, false) => return Cause::TimesDiffer,
            (false, true, true) => return Cause::StationsNamedOtherwise,
            _ => {}
        }
    }
    let stations = |calls: &[Call]| -> Vec<u32> { calls.iter().map(|call| call.0).collect() };
    let (sa, sb) = (stations(a), stations(b));
    let (short, long) = if sa.len() < sb.len() {
        (&sa, &sb)
    } else {
        (&sb, &sa)
    };
    let mut rest = long.iter();
    if short.len() < long.len()
        && short
            .iter()
            .all(|station| rest.any(|other| other == station))
    {
        Cause::CallsLeftOut
    } else {
        Cause::CallsDiffer
    }
}

fn fingerprint(calls: &[Call]) -> u64 {
    let mut hasher = FxHasher::default();
    calls.hash(&mut hasher);
    hasher.finish()
}

/// `H:MM:SS` in seconds.
fn seconds(text: &str) -> Option<u32> {
    let mut parts = text.trim().split(':');
    let hours: u32 = parts.next()?.parse().ok()?;
    let minutes: u32 = parts.next()?.parse().ok()?;
    let seconds: u32 = parts.next()?.parse().ok()?;
    Some(hours * 3600 + minutes * 60 + seconds)
}

/// The stations of a feed, numbered by key, with the position of each.
#[derive(Default)]
struct Stations {
    names: Vec<String>,
    positions: Vec<Option<(f64, f64)>>,
    numbers: FxHashMap<String, u32>,
    of_stop: FxHashMap<String, u32>,
}

impl Stations {
    /// The number of the station of a stop.
    fn of(
        &mut self,
        stop: &str,
        parents: &FxHashMap<String, String>,
        positions: &FxHashMap<String, (f64, f64)>,
    ) -> u32 {
        if let Some(&known) = self.of_stop.get(stop) {
            return known;
        }
        let parent = parents.get(stop).map(String::as_str);
        let key = dhid_station(stop)
            .or_else(|| parent.and_then(dhid_station))
            .or_else(|| sloid_station(stop))
            .or_else(|| parent.and_then(sloid_station))
            .or_else(|| station_number(stop))
            .or_else(|| parent.and_then(station_number))
            .unwrap_or_else(|| parent.unwrap_or(stop).to_owned());
        let number = match self.numbers.get(&key) {
            Some(&number) => number,
            None => {
                let next = self.names.len() as u32;
                self.numbers.insert(key.clone(), next);
                self.names.push(key);
                self.positions.push(None);
                next
            }
        };
        let slot = &mut self.positions[number as usize];
        if slot.is_none() {
            *slot = positions.get(stop).copied();
        }
        self.of_stop.insert(stop.to_owned(), number);
        number
    }
}

/// The station of a DHID — a stop's or a quay's —: its first three parts
/// (`de:06412:1502`, `at:46:3040`).
pub fn dhid_station(id: &str) -> Option<String> {
    // A feed numbers a DHID several of its stops carry: `de:06412:1502#2`.
    let id = id.split('#').next().unwrap_or(id);
    let parts: Vec<&str> = id.split(':').collect();
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
    (parts.len() >= 3
        && parts[0].len() == 2
        && parts[0].bytes().all(|b| b.is_ascii_lowercase())
        && digits(parts[1])
        && digits(parts[2]))
    .then(|| parts[..3].join(":"))
}

/// The station of a Swiss SLOID — a stop's or a quay's, or its parent's as
/// the Swiss GTFS writes it, `Parentch:1:sloid:3000` —: its first four parts.
pub fn sloid_station(id: &str) -> Option<String> {
    let at = id.find("ch:1:sloid:")?;
    let parts: Vec<&str> = id[at..].splitn(5, ':').collect();
    let number = parts
        .get(3)
        .filter(|part| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit()))?;
    Some(format!("ch:1:sloid:{number}"))
}

/// The station number an identifier carries: its first run of at least seven
/// digits, without leading zeros — a Swiss or UIC station number, written
/// `8503000`, `000005603743` or within `DE::ScheduledStopPoint:5603743_DBDB_::`.
pub fn station_number(id: &str) -> Option<String> {
    let bytes = id.as_bytes();
    let mut start = None;
    for at in 0..=bytes.len() {
        let digit = bytes.get(at).is_some_and(u8::is_ascii_digit);
        match (digit, start) {
            (true, None) => start = Some(at),
            (false, Some(from)) => {
                if at - from >= 7 {
                    return Some(id[from..at].trim_start_matches('0').to_owned());
                }
                start = None;
            }
            _ => {}
        }
    }
    None
}

/// The days compared each service of a feed runs on, by position in `days`.
fn active_services(path: &Path, days: &[&str]) -> FxHashMap<String, Vec<usize>> {
    const WEEKDAYS: [&str; 7] = [
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
    ];
    let weekdays: Vec<usize> = days
        .iter()
        .map(|day| {
            let date = netex_rs::model::Date::from(
                format!("{}-{}-{}", &day[..4], &day[4..6], &day[6..]).as_str(),
            );
            (date.days_since_epoch().expect("a date") + 3).rem_euclid(7) as usize
        })
        .collect();
    let mut active: FxHashMap<String, Vec<usize>> = FxHashMap::default();
    if path.join("calendar.txt").exists() {
        let mut columns = vec!["service_id", "start_date", "end_date"];
        columns.extend(WEEKDAYS);
        for row in rows_of(path, "calendar.txt", &columns) {
            let runs = days
                .iter()
                .enumerate()
                .filter(|&(at, day)| {
                    row[3 + weekdays[at]] == "1"
                        && row[1].as_str() <= *day
                        && *day <= row[2].as_str()
                })
                .map(|(at, _)| at)
                .collect();
            active.insert(row[0].clone(), runs);
        }
    }
    if path.join("calendar_dates.txt").exists() {
        for [service, date, kind] in rows(
            path,
            "calendar_dates.txt",
            &["service_id", "date", "exception_type"],
        ) {
            let Some(at) = days.iter().position(|day| *day == date) else {
                continue;
            };
            let list = active.entry(service).or_default();
            if kind == "1" {
                if !list.contains(&at) {
                    list.push(at);
                }
            } else {
                list.retain(|&d| d != at);
            }
        }
    }
    active
}

/// The columns `wanted` of every row of a file of the feed, in order.
fn rows<const N: usize>(
    path: &Path,
    name: &str,
    wanted: &[&str; N],
) -> impl Iterator<Item = [String; N]> {
    rows_of(path, name, wanted)
        .map(|row| <[String; N]>::try_from(row).expect("as many columns as asked for"))
}

fn rows_of(path: &Path, name: &str, wanted: &[&str]) -> impl Iterator<Item = Vec<String>> {
    let file: PathBuf = path.join(name);
    let mut reader =
        csv::Reader::from_path(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
    let header: Vec<String> = reader
        .headers()
        .expect("a header")
        .iter()
        .map(|h| h.trim_start_matches('\u{feff}').to_owned())
        .collect();
    let columns: Vec<Option<usize>> = wanted
        .iter()
        .map(|column| header.iter().position(|h| h == column))
        .collect();
    reader.into_records().map(move |record| {
        let record = record.expect("a row");
        columns
            .iter()
            .map(|column| column.and_then(|c| record.get(c)).unwrap_or("").to_owned())
            .collect()
    })
}
