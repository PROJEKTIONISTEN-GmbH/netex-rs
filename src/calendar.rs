//! The days a NeTEx calendar states, by the rules this crate reads them with.
//!
//! The [index](crate::index) applies these rules; a reader that gathers a
//! delivery its own way applies the same functions, so that every reader of
//! the crate gives a journey the same days. A day is a number of days since
//! 1970-01-01 ([`Date::days_since_epoch`](crate::model::Date::days_since_epoch)).
//!
//! - A calendar holds the days of the years 1 to 9999 ([`FIRST_DAY`],
//!   [`LAST_DAY`]); a delivery naming days before or after names none of
//!   them.
//! - An operating period gives the days its bits name, counted from its first
//!   day — a day without a bit available, as the schema says —, or every day
//!   from its first to its last where it states no bits ([`period_days`]), as
//!   far as they are asked for: a period reaching the year 9999 gives the
//!   days of the span a reader holds, not millions of others. Its first and
//!   last day are the dates of its bounds; a bound written as a whole hour in
//!   UTC from noon on — the midnight beginning a local day, as the Belgian
//!   deliveries write it — names the next day ([`bound_day`]). A validity
//!   (`ValidBetween`) is read so too, every one an entity states
//!   ([`Span::all_of`]).
//! - Day type assignments apply in the order of their `order`, those stating
//!   none last, the document's order between equals; a later assignment
//!   overrides an earlier one, adding its days or, where it is not available,
//!   taking them away ([`assignment_rank`]).
//! - A line written in versions — one identifier, a `version` and a
//!   `ValidBetween` each, as the Austrian deliveries write it — limits the
//!   days of a journey naming one: a version governs the days it is valid on
//!   but those a version beginning later is valid on ([`governed`]).
//! - A day type's properties are read where they name days of the week and
//!   nothing else ([`properties`]): an assignment of the day type to an
//!   operating period without day bits gives the period's days on those days
//!   of the week; and a day type stating a validity of a first and a last day,
//!   none of whose assignments gives it a day of its own — a period, or a date
//!   within its validity on one of its days of the week —, runs on the days of
//!   its validity on its days of the week, its assignments adding days to them
//!   and taking days away in their order. That is the French profile's day
//!   type: a validity and days of the week, the assignments its exceptions. A
//!   day type whose assignments do give it such a day — as where a delivery
//!   lists each day it runs on — runs on its assignments' days alone, its
//!   properties describing them.

use std::ops::RangeInclusive;

use crate::model::{DateTime, Order, PropertyOfDay, Time, ValidBetween};

/// The first day a calendar holds, in days since 1970-01-01: 1 January of the
/// year 1.
pub const FIRST_DAY: i64 = -719_162;

/// The last day a calendar holds, in days since 1970-01-01: 31 December 9999,
/// the last day of a four-digit year, which deliveries write for "until
/// further notice".
pub const LAST_DAY: i64 = 2_932_896;

/// The days a version of an entity is valid on, either end open where it
/// states none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    /// The first day, if stated.
    pub from: Option<i64>,
    /// The last day, if stated.
    pub to: Option<i64>,
}

impl Span {
    /// Valid on every day.
    pub const OPEN: Span = Span {
        from: None,
        to: None,
    };

    /// The days of each `ValidBetween` an entity states — the schema allows
    /// any number —, each end read as [`bound_day`] reads a bound and open
    /// where it states none; the open span alone where it states none.
    pub fn all_of(valid_between: &[ValidBetween]) -> Vec<Span> {
        if valid_between.is_empty() {
            return vec![Span::OPEN];
        }
        let day = |bound: &Option<DateTime>| bound.as_ref().and_then(bound_day);
        valid_between
            .iter()
            .map(|valid| Span {
                from: day(&valid.from_date),
                to: day(&valid.to_date),
            })
            .collect()
    }
}

/// The days a version of a line governs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Governed {
    /// Every day: a version without a validity that no later one takes days from.
    Every,
    /// The days of these spans, first and last day each, ascending and apart.
    Spans(Vec<(i64, i64)>),
}

impl Governed {
    /// Whether `day` is governed.
    pub fn contains(&self, day: i64) -> bool {
        match self {
            Governed::Every => true,
            Governed::Spans(spans) => spans
                .iter()
                .any(|(first, last)| (*first..=*last).contains(&day)),
        }
    }
}

/// The days the version at `position` among the versions of one line
/// governs, each version given as the spans of its validities
/// ([`Span::all_of`]): the days each of its validities is valid on, but those
/// of every validity of another version beginning later, whatever end the
/// earlier one states. A validity without a beginning takes no day from
/// another.
///
/// The rule is read off the Austrian deliveries, whose journeys carry the day
/// bits of the whole timetable year and whose lines change in versions.
///
/// # Panics
///
/// If `position` is not a position of `versions`.
pub fn governed(versions: &[Vec<Span>], position: usize) -> Governed {
    let own = &versions[position];
    let others: Vec<Span> = versions
        .iter()
        .enumerate()
        .filter(|&(version, _)| version != position)
        .flat_map(|(_, spans)| spans.iter().copied())
        .collect();
    if own.iter().all(|span| *span == Span::OPEN) && others.iter().all(|span| span.from.is_none()) {
        return Governed::Every;
    }
    let mut governed = Vec::new();
    for span in own {
        let mut pieces = vec![(span.from.unwrap_or(i64::MIN), span.to.unwrap_or(i64::MAX))];
        let later = others.iter().filter_map(|other| {
            let from = other
                .from
                .filter(|from| span.from.is_none_or(|own_from| *from > own_from))?;
            Some((from, other.to.unwrap_or(i64::MAX)))
        });
        for (from, to) in later {
            pieces = pieces
                .into_iter()
                .flat_map(|(first, last)| {
                    let before = (first < from).then(|| (first, last.min(from - 1)));
                    let after = (last > to).then(|| (first.max(to + 1), last));
                    before.into_iter().chain(after)
                })
                .filter(|(first, last)| first <= last)
                .collect();
        }
        governed.extend(pieces);
    }
    Governed::Spans(union_of_spans(governed))
}

/// Spans of days, first and last day each, made ascending and apart: those
/// that overlap or meet joined.
pub(crate) fn union_of_spans(mut spans: Vec<(i64, i64)>) -> Vec<(i64, i64)> {
    spans.sort_unstable();
    let mut joined: Vec<(i64, i64)> = Vec::with_capacity(spans.len());
    for (first, last) in spans {
        match joined.last_mut() {
            Some((_, held)) if first <= held.saturating_add(1) => *held = (*held).max(last),
            _ => joined.push((first, last)),
        }
    }
    joined
}

/// The days two lists of spans, each ascending and apart, have in common.
pub(crate) fn intersection_of_spans(a: &[(i64, i64)], b: &[(i64, i64)]) -> Vec<(i64, i64)> {
    let (mut i, mut j) = (0, 0);
    let mut common = Vec::new();
    while let (Some(&(a_first, a_last)), Some(&(b_first, b_last))) = (a.get(i), b.get(j)) {
        let (first, last) = (a_first.max(b_first), a_last.min(b_last));
        if first <= last {
            common.push((first, last));
        }
        if a_last < b_last {
            i += 1;
        } else {
            j += 1;
        }
    }
    common
}

/// The days an operating period gives, and what was off about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeriodDays {
    /// The days, ascending.
    pub days: Vec<i64>,
    /// What the period states that the days were read past, if anything: a
    /// day bit other than `0` or `1` before day bits other in number than its
    /// days, where it states both.
    pub issue: Option<PeriodIssue>,
}

/// What a period states that its days are read past.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PeriodIssue {
    /// No first day: the period gives no days.
    NoFirstDay,
    /// Day bits other in number than the days from the first to the last
    /// day: the bits are read from the first day, a day without one is
    /// available, as the schema says, and a bit past the last day gives none.
    BitsOtherThanDays {
        /// How many bits the period states, white space aside.
        bits: usize,
        /// How many days its first to its last day span.
        days: i64,
    },
    /// A day bit other than `0` or `1`, white space aside: no day there.
    BitOtherThanZeroOrOne {
        /// The first such bit, counted from the period's first day.
        at: usize,
    },
}

/// The days of an operating period from `from` to `to` among the days
/// `within`, with the bits of a UIC operating period where it states them:
/// the days its bits name, counted from its first day — a day to its last
/// without a bit available ("If bit is missing, assume available"), a bit
/// past its last day none —, else every day from its first to its last — the
/// first alone where it states no last. The bits are a normalized string:
/// white space among them — a line break, an indent — is no bit.
///
/// Only the days `within` are given, so that what a period costs is the days
/// asked for: one written to the year 9999 gives a reader holding a year of
/// days that year's, not the 2.9 million to its end.
pub fn period_days(
    from: Option<i64>,
    to: Option<i64>,
    bits: Option<&str>,
    within: RangeInclusive<i64>,
) -> PeriodDays {
    let mut days = Vec::new();
    let issue = period_runs(from, to, bits, within, |first, last| {
        days.extend(first..=last);
    });
    PeriodDays { days, issue }
}

/// The days of [`period_days`], as runs: `run` is called with the first and
/// last day of each run of days one after the other, ascending; and what was
/// off about the period.
pub(crate) fn period_runs(
    from: Option<i64>,
    to: Option<i64>,
    bits: Option<&str>,
    within: RangeInclusive<i64>,
    mut run: impl FnMut(i64, i64),
) -> Option<PeriodIssue> {
    let Some(from) = from else {
        return Some(PeriodIssue::NoFirstDay);
    };
    let (lowest, highest) = within.into_inner();
    match bits {
        Some(bits) => {
            let last_day = to.unwrap_or(i64::MAX);
            let mut count = 0usize;
            let mut other = None;
            let mut runs = Runs::within(lowest, highest.min(last_day));
            for bit in bits
                .chars()
                .filter(|c| !matches!(c, ' ' | '\t' | '\n' | '\r'))
            {
                let day = from.saturating_add(count as i64);
                count += 1;
                match bit {
                    '1' => runs.add(day, day, &mut run),
                    '0' => {}
                    _ => {
                        other.get_or_insert(count - 1);
                    }
                }
            }
            let days = to.map(|to| to - from + 1);
            // A day of the period without a bit is available: "If bit is
            // missing, assume available" (the schema's `ValidDayBits`).
            if let Some(days) = days.filter(|&days| (count as i64) < days) {
                runs.add(from + count as i64, from + days - 1, &mut run);
            }
            runs.finish(&mut run);
            let length = days.and_then(|days| {
                (count as i64 != days)
                    .then_some(PeriodIssue::BitsOtherThanDays { bits: count, days })
            });
            other
                .map(|at| PeriodIssue::BitOtherThanZeroOrOne { at })
                .or(length)
        }
        None => {
            let last = to.unwrap_or(from).max(from).min(highest);
            let first = from.max(lowest);
            if first <= last {
                run(first, last);
            }
            None
        }
    }
}

/// Days gathered into runs of days one after the other, within a span, each
/// run handed on once it ends.
struct Runs {
    lowest: i64,
    highest: i64,
    current: Option<(i64, i64)>,
}

impl Runs {
    fn within(lowest: i64, highest: i64) -> Runs {
        Runs {
            lowest,
            highest,
            current: None,
        }
    }

    /// Adds the days from `first` to `last`, ascending after those added
    /// before, as far as they lie within the span.
    fn add(&mut self, first: i64, last: i64, run: &mut impl FnMut(i64, i64)) {
        let (first, last) = (first.max(self.lowest), last.min(self.highest));
        if first > last {
            return;
        }
        self.current = match self.current {
            Some((held, end)) if end + 1 >= first => Some((held, end.max(last))),
            Some((held, end)) => {
                run(held, end);
                Some((first, last))
            }
            None => Some((first, last)),
        };
    }

    /// Hands on the run still open.
    fn finish(self, run: &mut impl FnMut(i64, i64)) {
        if let Some((first, last)) = self.current {
            run(first, last);
        }
    }
}

/// Seconds since midnight from which a bound written as an instant in UTC
/// may be the midnight beginning the next local day.
const NOON: u32 = 12 * 3600;

/// The day a bound names — the `FromDate` or `ToDate` of an operating period,
/// an availability condition or a validity, an `xsd:dateTime` —: the day of
/// its date; but where it is written as a whole hour in UTC (`Z`, `+00:00`)
/// at noon or later, the next day.
///
/// The bounds of a period are days. A producer writing them as instants in
/// UTC writes the midnight beginning each local day: `2026-10-21T22:00:00Z`
/// is the midnight beginning 22 October in Brussels in summer time,
/// `2026-10-29T23:00:00Z` the one beginning 30 October in winter time — the
/// Belgian deliveries' bounds, and the day bits they state count from those
/// days. No time zone is needed to read them so: a local midnight within
/// twelve hours of UTC is nearest the UTC midnight of its own day. Any other
/// bound is the day of its date: one without a zone, one before noon, the
/// end of a day (`T23:59:59Z`), and a time written in a zone of its own
/// (`T23:00:00+01:00`), which is that zone's time of that day.
pub fn bound_day(bound: &DateTime) -> Option<i64> {
    let day = bound.date()?.days_since_epoch()?;
    Some(day + i64::from(may_begin_the_next_local_day(bound)))
}

/// Whether an instant is a whole hour in UTC from noon on: the midnight
/// beginning a local day east of Greenwich, written in UTC.
fn may_begin_the_next_local_day(bound: &DateTime) -> bool {
    let Some((_, time_and_zone)) = bound.as_str().split_once('T') else {
        return false;
    };
    let Some(zone_at) = time_and_zone.find(['Z', '+', '-']) else {
        return false;
    };
    let (time, zone) = time_and_zone.split_at(zone_at);
    let (time, fraction) = time.split_once('.').unwrap_or((time, ""));
    matches!(zone, "Z" | "+00:00" | "-00:00")
        && fraction.bytes().all(|digit| digit == b'0')
        && Time::seconds_of(time).is_some_and(|seconds| seconds >= NOON && seconds % 3600 == 0)
}

/// Days of the week, as a `DaysOfWeek` list of a property of day names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct DaysOfWeek(u8);

impl DaysOfWeek {
    /// No day.
    pub const NONE: DaysOfWeek = DaysOfWeek(0);
    /// Every day of the week.
    pub const EVERY: DaysOfWeek = DaysOfWeek(0b111_1111);

    /// The days a list of the schema's `DayOfWeekEnumeration` tokens names —
    /// `Monday` to `Sunday`, `Weekdays`, `Weekend`, `Everyday`, `none` —, or
    /// none where a token is not one of them.
    pub fn parse(list: &str) -> Option<DaysOfWeek> {
        list.split_whitespace()
            .try_fold(DaysOfWeek::NONE, |days, token| {
                let named = match token {
                    "Monday" => 1,
                    "Tuesday" => 1 << 1,
                    "Wednesday" => 1 << 2,
                    "Thursday" => 1 << 3,
                    "Friday" => 1 << 4,
                    "Saturday" => 1 << 5,
                    "Sunday" => 1 << 6,
                    "Weekdays" => 0b001_1111,
                    "Weekend" => 0b110_0000,
                    "Everyday" => DaysOfWeek::EVERY.0,
                    "none" => 0,
                    _ => return None,
                };
                Some(DaysOfWeek(days.0 | named))
            })
    }

    /// Whether `day`, in days since 1970-01-01, falls on one of these days.
    pub fn contains(self, day: i64) -> bool {
        // 1970-01-01 was a Thursday, the fourth day from Monday.
        let from_monday = (day + 3).rem_euclid(7);
        self.0 & (1 << from_monday) != 0
    }
}

/// What a day type's properties say of the days it runs on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Properties {
    /// It states none.
    Absent,
    /// Days of the week and nothing else.
    DaysOfWeek(DaysOfWeek),
    /// More than days of the week — holidays, seasons, weeks of the month, a
    /// token the schema does not list —, which the crate does not read.
    Unread,
}

/// What the properties of a day type say of its days: each property of day
/// one alternative, the days of the week it names — every day where it names
/// none, as the French profile says, "Tous les jours par défaut" —, and
/// [`Properties::Unread`] where any names more than days of the week.
pub fn properties(properties: &[PropertyOfDay]) -> Properties {
    if properties.is_empty() {
        return Properties::Absent;
    }
    let mut days = DaysOfWeek::NONE;
    for property in properties {
        let named = match &property.days_of_week {
            Some(list) => DaysOfWeek::parse(list),
            None => Some(DaysOfWeek::EVERY),
        };
        match named {
            Some(named) if !names_more_than_days_of_week(property) => days.0 |= named.0,
            _ => return Properties::Unread,
        }
    }
    Properties::DaysOfWeek(days)
}

/// Whether a property of day names more of its days than days of the week.
fn names_more_than_days_of_week(property: &PropertyOfDay) -> bool {
    property.weeks_of_month.is_some()
        || property.month_of_year.is_some()
        || property.day_of_month.is_some()
        || property.day_of_year.is_some()
        || property.country_ref.is_some()
        || property.holiday_types.is_some()
        || property.seasons.is_some()
        || !property.unprocessed.is_empty()
}

/// Whether a property of day names nothing of its days — no days of the
/// week, nothing more —, which [`properties`] reads as every day.
pub(crate) fn names_no_day(property: &PropertyOfDay) -> bool {
    property.days_of_week.is_none() && !names_more_than_days_of_week(property)
}

/// Where an assignment of a day type with `order` stands among the
/// assignments of its document: by its order, one stating none after every
/// one that does.
pub fn assignment_rank(order: Option<&Order>) -> u32 {
    order.and_then(Order::value).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Date;

    fn valid(from: i64, to: Option<i64>) -> Span {
        Span {
            from: Some(from),
            to,
        }
    }

    #[test]
    fn a_later_beginning_version_takes_its_days_over_whatever_end_the_earlier_states() {
        // Version 1 from day 10 on, open; version 2 days 20 to 25; version 3 from day 30 on.
        let versions = [
            vec![valid(10, None)],
            vec![valid(20, Some(25))],
            vec![valid(30, None)],
        ];
        assert_eq!(
            governed(&versions, 0),
            Governed::Spans(vec![(10, 19), (26, 29)])
        );
        assert_eq!(
            governed(&versions, 1),
            Governed::Spans(vec![(20, 25)]),
            "version 3 begins after version 2 ends"
        );
        assert_eq!(
            governed(&versions, 2),
            Governed::Spans(vec![(30, i64::MAX)])
        );
        assert!(!governed(&versions, 0).contains(22) && governed(&versions, 0).contains(27));
    }

    #[test]
    fn a_period_gives_the_days_of_its_bits_or_of_its_span() {
        let all = FIRST_DAY..=LAST_DAY;
        assert_eq!(
            period_days(Some(10), Some(14), Some("10100"), all.clone()).days,
            [10, 12]
        );
        assert_eq!(
            period_days(Some(10), Some(12), None, all.clone()).days,
            [10, 11, 12]
        );
        assert_eq!(
            period_days(Some(10), None, None, all.clone()).days,
            [10],
            "the first day alone without a last"
        );
        let short = period_days(Some(10), Some(14), Some("11"), all.clone());
        assert_eq!(short.days, [10, 11, 12, 13, 14]);
        assert_eq!(
            short.issue,
            Some(PeriodIssue::BitsOtherThanDays { bits: 2, days: 5 })
        );
        assert_eq!(
            period_days(None, Some(14), Some("1"), all),
            PeriodDays {
                days: Vec::new(),
                issue: Some(PeriodIssue::NoFirstDay)
            }
        );
    }

    #[test]
    fn a_period_gives_its_days_within_the_span_asked_for_in_runs() {
        assert_eq!(
            period_days(Some(10), Some(i64::MAX / 2), None, 12..=13).days,
            [12, 13]
        );
        assert_eq!(
            period_days(Some(10), Some(19), Some("1111011011"), 11..=17).days,
            [11, 12, 13, 15, 16]
        );
        assert!(period_days(Some(10), Some(14), None, 20..=30)
            .days
            .is_empty());
        let mut runs = Vec::new();
        period_runs(Some(10), None, Some("0110111"), 0..=100, |first, last| {
            runs.push((first, last));
        });
        assert_eq!(runs, [(11, 12), (14, 16)]);
    }

    #[test]
    fn a_calendar_holds_the_years_1_to_9999() {
        assert_eq!(Date::from("0001-01-01").days_since_epoch(), Some(FIRST_DAY));
        assert_eq!(Date::from("9999-12-31").days_since_epoch(), Some(LAST_DAY));
    }

    fn bound(text: &str) -> Option<String> {
        bound_day(&DateTime::from(text)).map(|day| Date::from_days_since_epoch(day).to_string())
    }

    #[test]
    fn a_bound_written_as_a_local_midnight_in_utc_names_the_day_it_begins() {
        let next = |text| bound(text).expect("a day");
        assert_eq!(next("2026-10-21T22:00:00Z"), "2026-10-22", "summer time");
        assert_eq!(next("2026-10-29T23:00:00Z"), "2026-10-30", "winter time");
        assert_eq!(next("2026-10-29T23:00:00+00:00"), "2026-10-30");
        assert_eq!(
            bound("2026-10-29T23:00:00-01:00").as_deref(),
            Some("2026-10-29"),
            "a time written in a zone of its own is that zone's, on its date"
        );
        assert_eq!(
            bound("2026-10-21T00:00:00+01:00").as_deref(),
            Some("2026-10-21")
        );
        assert_eq!(
            bound("2030-01-10T11:45:00.000Z").as_deref(),
            Some("2030-01-10")
        );
        assert_eq!(
            bound("2026-12-12T23:59:59").as_deref(),
            Some("2026-12-12"),
            "a local time is the day of its date"
        );
        assert_eq!(
            bound("2026-10-21").as_deref(),
            None,
            "no time, no date-time"
        );
    }

    #[test]
    fn days_of_the_week_are_read_from_monday_on() {
        let weekend = DaysOfWeek::parse("Saturday Sunday").expect("days of the week");
        assert_eq!(Some(weekend), DaysOfWeek::parse("Weekend"));
        assert_eq!(
            DaysOfWeek::parse("Weekdays Weekend"),
            Some(DaysOfWeek::EVERY)
        );
        assert_eq!(DaysOfWeek::parse("none"), Some(DaysOfWeek::NONE));
        assert_eq!(DaysOfWeek::parse("Friday Holiday"), None);
        // 1970-01-01 was a Thursday, 2026-10-17 a Saturday.
        let saturday = crate::model::Date::from("2026-10-17")
            .days_since_epoch()
            .expect("a date");
        assert!(weekend.contains(saturday) && weekend.contains(saturday + 1));
        assert!(!weekend.contains(saturday + 2) && !weekend.contains(saturday - 1));
        let thursday = DaysOfWeek::parse("Thursday").expect("a day");
        assert!(thursday.contains(0) && !thursday.contains(1) && thursday.contains(-7));
    }

    #[test]
    fn properties_are_alternatives_of_days_of_the_week_or_unread() {
        let property = |days: Option<&str>| PropertyOfDay {
            days_of_week: days.map(Box::from),
            ..PropertyOfDay::default()
        };
        assert_eq!(properties(&[]), Properties::Absent);
        assert_eq!(
            properties(&[property(Some("Monday")), property(Some("Friday"))]),
            Properties::DaysOfWeek(DaysOfWeek::parse("Monday Friday").expect("days"))
        );
        assert_eq!(
            properties(&[property(None)]),
            Properties::DaysOfWeek(DaysOfWeek::EVERY),
            "a property naming no day of the week is every day"
        );
        let summer_saturdays = PropertyOfDay {
            seasons: Some("Summer".into()),
            ..property(Some("Saturday"))
        };
        assert_eq!(properties(&[summer_saturdays]), Properties::Unread);
        assert_eq!(
            properties(&[property(Some("Feiertag"))]),
            Properties::Unread
        );
    }

    #[test]
    fn a_property_names_no_day_where_it_states_neither_days_of_the_week_nor_more() {
        let named = |name: &str| PropertyOfDay {
            name: Some(crate::model::MultilingualString::from(name)),
            ..PropertyOfDay::default()
        };
        assert!(names_no_day(&PropertyOfDay::default()));
        assert!(names_no_day(&named("Mo-Fr")), "a name names no day");
        let everyday = PropertyOfDay {
            days_of_week: Some("Everyday".into()),
            ..PropertyOfDay::default()
        };
        assert!(!names_no_day(&everyday));
        let summer = PropertyOfDay {
            seasons: Some("Summer".into()),
            ..PropertyOfDay::default()
        };
        assert!(!names_no_day(&summer));
    }

    #[test]
    fn an_assignment_without_an_order_comes_after_every_one_with() {
        assert_eq!(assignment_rank(Some(&Order::from(3))), 3);
        assert_eq!(assignment_rank(None), u32::MAX);
        assert_eq!(assignment_rank(Some(&Order("x".into()))), u32::MAX);
    }
}
