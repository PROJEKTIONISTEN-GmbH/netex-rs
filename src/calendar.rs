//! The days a NeTEx calendar states, by the rules this crate reads them with.
//!
//! The [index](crate::index) applies these rules; a reader that gathers a
//! delivery its own way applies the same functions, so that every reader of
//! the crate gives a journey the same days. A day is a number of days since
//! 1970-01-01 ([`Date::days_since_epoch`](crate::model::Date::days_since_epoch)).
//!
//! - An operating period gives the days its bits name, counted from its first
//!   day, or every day from its first to its last where it states no bits
//!   ([`period_days`]).
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

use crate::model::{DateTime, Order, PropertyOfDay, ValidBetween};

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

    /// The days of an entity's own `ValidBetween` — the first it states —,
    /// open where it states none.
    pub fn of(valid_between: &[ValidBetween]) -> Span {
        let day = |date: &Option<DateTime>| {
            date.as_ref()
                .and_then(|d| d.date())
                .and_then(|d| d.days_since_epoch())
        };
        valid_between.first().map_or(Span::OPEN, |valid| Span {
            from: day(&valid.from_date),
            to: day(&valid.to_date),
        })
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
/// governs: the days it is valid on, but those of every version beginning
/// later, whatever end the earlier one states. A version without a beginning
/// takes no day from another.
///
/// The rule is read off the Austrian deliveries, whose journeys carry the day
/// bits of the whole timetable year and whose lines change in versions.
///
/// # Panics
///
/// If `position` is not a position of `versions`.
pub fn governed(versions: &[Span], position: usize) -> Governed {
    let own = versions[position];
    let later: Vec<(i64, i64)> = versions
        .iter()
        .filter_map(|other| {
            let from = other
                .from
                .filter(|from| own.from.is_none_or(|own_from| *from > own_from))?;
            Some((from, other.to.unwrap_or(i64::MAX)))
        })
        .collect();
    if own == Span::OPEN && later.is_empty() {
        return Governed::Every;
    }
    let mut spans = vec![(own.from.unwrap_or(i64::MIN), own.to.unwrap_or(i64::MAX))];
    for (from, to) in later {
        spans = spans
            .into_iter()
            .flat_map(|(first, last)| {
                let before = (first < from).then(|| (first, last.min(from - 1)));
                let after = (last > to).then(|| (first.max(to + 1), last));
                before.into_iter().chain(after)
            })
            .filter(|(first, last)| first <= last)
            .collect();
    }
    Governed::Spans(spans)
}

/// The days an operating period gives, and what was off about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeriodDays {
    /// The days, ascending.
    pub days: Vec<i64>,
    /// What the period states that the days were read past, if anything.
    pub issue: Option<PeriodIssue>,
}

/// What a period states that its days are read past.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeriodIssue {
    /// No first day: the period gives no days.
    NoFirstDay,
    /// Day bits other in number than the days from the first to the last
    /// day: the bits are read from the first day as far as they go.
    BitsOtherThanDays {
        /// How many bits the period states.
        bits: usize,
        /// How many days its first to its last day span.
        days: i64,
    },
}

/// The days of an operating period from `from` to `to`, with the bits of a
/// UIC operating period where it states them: the days its bits name,
/// counted from its first day, else every day from its first to its last — the
/// first alone where it states no last.
pub fn period_days(from: Option<i64>, to: Option<i64>, bits: Option<&str>) -> PeriodDays {
    let Some(from) = from else {
        return PeriodDays {
            days: Vec::new(),
            issue: Some(PeriodIssue::NoFirstDay),
        };
    };
    match bits {
        Some(bits) => {
            let issue = to.and_then(|to| {
                let days = to - from + 1;
                (bits.len() as i64 != days).then_some(PeriodIssue::BitsOtherThanDays {
                    bits: bits.len(),
                    days,
                })
            });
            let days = bits
                .bytes()
                .enumerate()
                .filter(|(_, bit)| *bit == b'1')
                .map(|(offset, _)| from + offset as i64)
                .collect();
            PeriodDays { days, issue }
        }
        None => PeriodDays {
            days: (from..=to.unwrap_or(from).max(from)).collect(),
            issue: None,
        },
    }
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
        let more = property.weeks_of_month.is_some()
            || property.month_of_year.is_some()
            || property.day_of_month.is_some()
            || property.day_of_year.is_some()
            || property.country_ref.is_some()
            || property.holiday_types.is_some()
            || property.seasons.is_some()
            || !property.unprocessed.is_empty();
        let named = match &property.days_of_week {
            Some(list) => DaysOfWeek::parse(list),
            None => Some(DaysOfWeek::EVERY),
        };
        match named {
            Some(named) if !more => days.0 |= named.0,
            _ => return Properties::Unread,
        }
    }
    Properties::DaysOfWeek(days)
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

    fn valid(from: i64, to: Option<i64>) -> Span {
        Span {
            from: Some(from),
            to,
        }
    }

    #[test]
    fn a_later_beginning_version_takes_its_days_over_whatever_end_the_earlier_states() {
        // Version 1 from day 10 on, open; version 2 days 20 to 25; version 3 from day 30 on.
        let versions = [valid(10, None), valid(20, Some(25)), valid(30, None)];
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
        assert_eq!(
            period_days(Some(10), Some(14), Some("10100")).days,
            [10, 12]
        );
        assert_eq!(period_days(Some(10), Some(12), None).days, [10, 11, 12]);
        assert_eq!(
            period_days(Some(10), None, None).days,
            [10],
            "the first day alone without a last"
        );
        let short = period_days(Some(10), Some(14), Some("11"));
        assert_eq!(short.days, [10, 11]);
        assert_eq!(
            short.issue,
            Some(PeriodIssue::BitsOtherThanDays { bits: 2, days: 5 })
        );
        assert_eq!(
            period_days(None, Some(14), Some("1")),
            PeriodDays {
                days: Vec::new(),
                issue: Some(PeriodIssue::NoFirstDay)
            }
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
    fn an_assignment_without_an_order_comes_after_every_one_with() {
        assert_eq!(assignment_rank(Some(&Order::from(3))), 3);
        assert_eq!(assignment_rank(None), u32::MAX);
        assert_eq!(assignment_rank(Some(&Order("x".into()))), u32::MAX);
    }
}
