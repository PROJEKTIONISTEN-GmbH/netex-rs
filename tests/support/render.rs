//! The content of an index as text, one line per entity, sorted: two indexes
//! hold the same content exactly when they render the same, whatever order
//! their entities were numbered in.

use std::fmt::Write as _;

use netex_rs::index::{Index, JourneyId, LineId, StopId, StopPointId};
use netex_rs::model::Date;

/// The days a rendering lists a journey's operation over: 2026-01-01 and the
/// 399 days after it, which cover every calendar of the tests.
fn window() -> Vec<Date> {
    let mut days = Vec::with_capacity(400);
    let (mut year, mut month, mut day) = (2026u32, 1u32, 1u32);
    while days.len() < 400 {
        days.push(Date::from(
            format!("{year:04}-{month:02}-{day:02}").as_str(),
        ));
        let length = match month {
            2 => 28,
            4 | 6 | 9 | 11 => 30,
            _ => 31,
        };
        day += 1;
        if day > length {
            day = 1;
            month += 1;
            if month > 12 {
                month = 1;
                year += 1;
            }
        }
    }
    days
}

/// Renders the index's content.
pub fn render(index: &Index) -> String {
    let days = window();
    let stop_id = |stop: StopId| index.stop(stop).id;
    let point_id = |point: StopPointId| index.stop_point(point).id;
    let mut lines: Vec<String> = Vec::new();
    for stop in index.stops() {
        let s = index.stop(stop);
        lines.push(format!(
            "stop {} dhid={:?} name={:?} kind={:?} at={:?},{:?} parent={:?} level={:?}",
            s.id,
            s.dhid,
            s.name,
            s.kind,
            s.latitude,
            s.longitude,
            s.parent.map(stop_id),
            s.level
        ));
        for walk in index.transfers_from(stop) {
            lines.push(format!(
                "walk {} -> {} {}s both={}",
                stop_id(walk.from),
                stop_id(walk.to),
                walk.seconds,
                walk.both_ways
            ));
        }
    }
    for point in (0..index.stop_point_count() as u32).map(StopPointId) {
        let p = index.stop_point(point);
        lines.push(format!(
            "point {} name={:?} at={:?},{:?} stop={:?}",
            p.id,
            p.name,
            p.latitude,
            p.longitude,
            p.stop.map(stop_id)
        ));
        for interchange in index.interchanges_at(point) {
            lines.push(format!(
                "interchange {} -> {} at {} -> {} seated={} guaranteed={} wait={:?}",
                index.journey(interchange.from_journey).id,
                index.journey(interchange.to_journey).id,
                point_id(interchange.from_stop_point),
                point_id(interchange.to_stop_point),
                interchange.stay_seated,
                interchange.guaranteed,
                interchange.maximum_wait_seconds
            ));
        }
    }
    for line in (0..index.line_count() as u32).map(LineId) {
        let l = index.line(line);
        lines.push(format!(
            "line {} name={:?} code={:?} mode={:?} journeys={}",
            l.id,
            l.name,
            l.public_code,
            l.mode,
            index.journeys_of_line(line).len()
        ));
    }
    for journey in (0..index.journey_count() as u32).map(JourneyId) {
        let j = index.journey(journey);
        let mut text = format!(
            "journey {} line={:?} pattern={:?} calls=",
            j.id,
            j.line.map(|line| index.line(line).id),
            j.pattern
        );
        for (point, time) in j.stop_points.iter().zip(j.passing_times) {
            let _ = write!(
                text,
                "[{} {:?} {:?}]",
                point_id(*point),
                time.arrival,
                time.departure
            );
        }
        text.push_str(" days=");
        for day in days.iter().filter(|day| index.runs_on(journey, day)) {
            let _ = write!(text, "{day} ");
        }
        lines.push(text);
    }
    for unresolved in index.unresolved() {
        lines.push(format!("unresolved {unresolved}"));
    }
    lines.sort();
    lines.join("\n")
}

/// Asserts that two renderings are the same, naming the first line they differ
/// in rather than printing both.
pub fn assert_same(left: &str, right: &str, context: &str) {
    if left == right {
        return;
    }
    let (left_lines, right_lines): (Vec<&str>, Vec<&str>) =
        (left.lines().collect(), right.lines().collect());
    let at = left_lines
        .iter()
        .zip(&right_lines)
        .position(|(l, r)| l != r)
        .unwrap_or(left_lines.len().min(right_lines.len()));
    panic!(
        "{context}: the indexes differ at line {at} of {} and {}:\n  {}\n  {}",
        left_lines.len(),
        right_lines.len(),
        left_lines.get(at).unwrap_or(&"(end)"),
        right_lines.get(at).unwrap_or(&"(end)")
    );
}
