//! `netex stats`: what a delivery holds, by the operator running it — stops,
//! lines, journeys, journey days — over its whole calendar or a period of it.

use std::collections::BTreeMap;

use netex2gtfs::Delivery;
use netex_rs::index::{JourneyId, StopPointId};
use netex_rs::model::Date;
use rustc_hash::FxHashSet;
use serde::Serialize;

/// The figures of one operator, or of the whole delivery.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Figures {
    /// The operator's NeTEx identifier; none for the whole delivery, and for
    /// journeys whose line and journey name no organisation.
    pub operator: Option<String>,
    /// The operator's name.
    pub name: Option<String>,
    /// Stops called at: stop places and quays, and stop points placed at none.
    pub stops: u64,
    /// Lines with a journey.
    pub lines: u64,
    /// Journeys running on at least one day of the period; a template
    /// journey counts once, however many runs it stands for.
    pub journeys: u64,
    /// Journey days: each journey once for every day of the period it runs on.
    pub journey_days: u64,
    /// The first and last day of the period any of the journeys runs on.
    pub first_day: Option<String>,
    /// See `first_day`.
    pub last_day: Option<String>,
}

/// The statistics of a delivery.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Stats {
    /// The source read.
    pub source: String,
    /// The period asked for, first and last day; none for the whole calendar.
    pub period: Option<(String, String)>,
    /// The whole delivery.
    pub total: Figures,
    /// By operator, the most journey days first.
    pub operators: Vec<Figures>,
}

impl Stats {
    /// The statistics for a person: one line for the delivery, one per operator.
    pub fn human(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        let period = self
            .period
            .as_ref()
            .map_or(String::from("the whole calendar"), |(first, last)| {
                format!("{first} to {last}")
            });
        let _ = writeln!(out, "{} — {period}", self.source);
        let line = |figures: &Figures| {
            format!(
                "{} stops, {} lines, {} journeys, {} journey days, {} to {}",
                figures.stops,
                figures.lines,
                figures.journeys,
                figures.journey_days,
                figures.first_day.as_deref().unwrap_or("-"),
                figures.last_day.as_deref().unwrap_or("-"),
            )
        };
        let _ = writeln!(out, "all: {}", line(&self.total));
        for operator in &self.operators {
            let who = match (&operator.name, &operator.operator) {
                (Some(name), Some(id)) => format!("{name} ({id})"),
                (None, Some(id)) => id.clone(),
                _ => "no operator named".to_owned(),
            };
            let _ = writeln!(out, "{who}: {}", line(operator));
        }
        out
    }
}

/// What the journeys of one operator add up to.
#[derive(Default)]
struct Sum {
    name: Option<String>,
    stops: FxHashSet<(bool, u32)>,
    lines: FxHashSet<u32>,
    journeys: u64,
    journey_days: u64,
    first: Option<i64>,
    last: Option<i64>,
}

impl Sum {
    fn add(&mut self, delivery: &Delivery, journey: JourneyId, days: &[i64]) {
        let index = delivery.index();
        let record = index.journey(journey);
        for &point in record.stop_points {
            self.stops.insert(place(delivery, point));
        }
        if let Some(line) = record.line {
            self.lines.insert(line.0);
        }
        self.journeys += 1;
        self.journey_days += days.len() as u64;
        if let (Some(&first), Some(&last)) = (days.first(), days.last()) {
            self.first = Some(self.first.map_or(first, |known| known.min(first)));
            self.last = Some(self.last.map_or(last, |known| known.max(last)));
        }
    }

    fn figures(&self, operator: Option<String>) -> Figures {
        Figures {
            operator,
            name: self.name.clone(),
            stops: self.stops.len() as u64,
            lines: self.lines.len() as u64,
            journeys: self.journeys,
            journey_days: self.journey_days,
            first_day: self
                .first
                .map(|day| Date::from_days_since_epoch(day).as_str().to_owned()),
            last_day: self
                .last
                .map(|day| Date::from_days_since_epoch(day).as_str().to_owned()),
        }
    }
}

/// Where a journey calls at a stop point: its stop, else the stop point itself.
fn place(delivery: &Delivery, point: StopPointId) -> (bool, u32) {
    match delivery.index().stop_of_stop_point(point) {
        Some(stop) => (true, stop.0),
        None => (false, point.0),
    }
}

/// The statistics of `delivery` read from `source`, over the days from
/// `first` to `last` where given.
pub fn stats(delivery: &Delivery, source: &str, period: Option<(&Date, &Date)>) -> Stats {
    let index = delivery.index();
    let bounds = period.map(|(first, last)| {
        (
            first.days_since_epoch().unwrap_or(i64::MIN),
            last.days_since_epoch().unwrap_or(i64::MAX),
        )
    });
    let mut total = Sum::default();
    let mut by_operator: BTreeMap<Option<String>, Sum> = BTreeMap::new();
    for journey in (0..index.journey_count() as u32).map(JourneyId) {
        let days: Vec<i64> = index
            .days_of(journey)
            .filter(|&day| bounds.is_none_or(|(first, last)| first <= day && day <= last))
            .collect();
        if days.is_empty() {
            continue;
        }
        let agency = delivery.agency_of_journey(journey);
        let sum = by_operator
            .entry(agency.map(|agency| agency.id.to_owned()))
            .or_default();
        if sum.name.is_none() {
            sum.name = agency.map(|agency| agency.name.to_owned());
        }
        sum.add(delivery, journey, &days);
        total.add(delivery, journey, &days);
    }
    let mut operators: Vec<Figures> = by_operator
        .into_iter()
        .map(|(operator, sum)| sum.figures(operator))
        .collect();
    operators.sort_by(|a, b| {
        b.journey_days
            .cmp(&a.journey_days)
            .then_with(|| a.operator.cmp(&b.operator))
    });
    Stats {
        source: source.to_owned(),
        period: period.map(|(first, last)| (first.as_str().to_owned(), last.as_str().to_owned())),
        total: total.figures(None),
        operators,
    }
}
