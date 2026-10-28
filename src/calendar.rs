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

use crate::model::{DateTime, Order, ValidBetween};

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
    fn an_assignment_without_an_order_comes_after_every_one_with() {
        assert_eq!(assignment_rank(Some(&Order::from(3))), 3);
        assert_eq!(assignment_rank(None), u32::MAX);
        assert_eq!(assignment_rank(Some(&Order("x".into()))), u32::MAX);
    }
}
