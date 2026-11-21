//! A period nothing names gives no journey a day, however far it reaches; so
//! it costs the index no more when it reaches the year 9999 — which
//! deliveries write for "until further notice" — than when it ends within a
//! month. A period that is named costs the days it gives, and no more than
//! them: the day types it gives none keep calendars of their own days. And the
//! calendar holds the days of the years 1 to 9999; a period reaching beyond
//! is read to the end of the year 9999, and is a finding.

mod support;

use netex_rs::calendar::period_days;
use netex_rs::index::{Concern, Index, IndexBuilder};
use netex_rs::model::Date;
use support::timetable::{Point, Timetable};

/// A journey every day of October 2026, and beside it an operating period
/// from 1 October 2026 to `to` that no assignment names.
fn with_a_period_named_by_none_until(to: &str) -> netex_rs::Document {
    let xml = Timetable::new()
        .line("L", "1", None)
        .stop_point("A")
        .stop_point("B")
        .pattern("P", "L", &[Point::at("P1", "A"), Point::at("P2", "B")])
        .day_type("DT")
        .period("OP", Some("2026-10-01"), "2026-10-31")
        .period("UNNAMED", Some("2026-10-01"), to)
        .assignment("DTA", 1, "OP", "DT", true)
        .journey(
            "J",
            ("L", "1"),
            "P",
            &["DT"],
            &[
                ("P1", None, Some("08:00:00")),
                ("P2", Some("08:30:00"), None),
            ],
        )
        .xml();
    netex_rs::read(&xml).expect("the document reads")
}

/// The bytes building the index of `document` allocates.
fn bytes_to_index(document: &netex_rs::Document) -> u64 {
    allocation_counter::measure(|| {
        let mut builder = IndexBuilder::new();
        builder.add(document, "a.xml");
        builder.finish();
    })
    .bytes_total
}

#[test]
fn a_period_named_by_none_costs_the_index_no_more_for_reaching_a_far_year() {
    let month = bytes_to_index(&with_a_period_named_by_none_until("2026-10-31"));
    let far = bytes_to_index(&with_a_period_named_by_none_until("9999-12-31"));
    assert!(
        far < 2 * month,
        "building the index allocates {far} bytes beside a period to 9999 nothing names, {month} beside one of a month"
    );
}

/// A journey every day of October 2026 on each of `day_types` day types, each
/// assigned its own operating period of the month; the first day type
/// assigned, beside, the period from 1 October 2026 to `to`.
fn with_a_period_named_until(to: &str, day_types: usize) -> netex_rs::Document {
    let mut timetable = Timetable::new()
        .line("L", "1", None)
        .stop_point("A")
        .stop_point("B")
        .pattern("P", "L", &[Point::at("P1", "A"), Point::at("P2", "B")])
        .period("FAR", Some("2026-10-01"), to)
        .assignment("DTA-FAR", 1, "FAR", "DT0", true);
    for number in 0..day_types {
        let (day_type, period) = (format!("DT{number}"), format!("OP{number}"));
        timetable = timetable
            .day_type(&day_type)
            .period(&period, Some("2026-10-01"), "2026-10-31")
            .assignment(&format!("DTA{number}"), 1, &period, &day_type, true)
            .journey(
                &format!("J{number}"),
                ("L", "1"),
                "P",
                &[&day_type],
                &[
                    ("P1", None, Some("08:00:00")),
                    ("P2", Some("08:30:00"), None),
                ],
            );
    }
    netex_rs::read(&timetable.xml()).expect("the document reads")
}

fn index_of(document: &netex_rs::Document) -> Index {
    let mut builder = IndexBuilder::new();
    builder.add(document, "a.xml");
    builder.finish()
}

fn day(date: &str) -> i64 {
    Date::from(date).days_since_epoch().expect("a date")
}

#[test]
fn a_period_named_to_a_far_year_costs_the_day_type_it_gives_days_not_every_other() {
    // A bit for each of the 2.9 million days to the end of 9999 is 364 kB:
    // the period's, its day type's and its journeys' calendar. Every other
    // day type and period of the month keeps a calendar of its own days.
    let month = bytes_to_index(&with_a_period_named_until("2026-10-31", 200));
    let far = bytes_to_index(&with_a_period_named_until("9999-12-31", 200));
    assert!(
        far < month + 8_000_000,
        "building the index allocates {far} bytes beside a period to 9999 one day type is assigned, {month} beside one of a month"
    );
}

#[test]
fn a_period_named_beyond_the_year_9999_is_read_to_its_end_and_is_a_finding() {
    let index = index_of(&with_a_period_named_until("2147483647-12-31", 2));
    assert_eq!(index.finding_count(Concern::DaysOutsideCalendar), 1);
    let first = index
        .journey_by_id("J0")
        .expect("the journey of the far period");
    let days: Vec<i64> = index.days_of(first).collect();
    assert_eq!(days.first(), Some(&day("2026-10-01")));
    assert_eq!(days.last(), Some(&day("9999-12-31")));
    assert_eq!(days.len() as i64, day("9999-12-31") - day("2026-10-01") + 1);
    let other = index.journey_by_id("J1").expect("the journey of a month");
    assert_eq!(index.days_of(other).count(), 31);
}

#[test]
fn the_days_of_a_period_are_those_within_the_span_asked_for() {
    let october = day("2026-10-01")..=day("2026-10-31");
    let days = period_days(
        Some(day("2026-10-30")),
        Some(day("2147483647-12-31")),
        None,
        october.clone(),
    );
    assert_eq!(days.days, [day("2026-10-30"), day("2026-10-31")]);
    // From 29 September: the 29th, the 30th, not the 1st, the 2nd, the 3rd.
    let bits = period_days(Some(day("2026-09-29")), None, Some("11011"), october);
    assert_eq!(bits.days, [day("2026-10-02"), day("2026-10-03")]);
}
