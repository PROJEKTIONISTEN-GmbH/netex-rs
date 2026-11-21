//! The days an entity is valid on, from every validity it states: each of
//! its `ValidBetween` — the schema allows any number —, those of its frames,
//! written directly or within their `validityConditions`, and an entity's
//! own within its frames'.

mod support;

use netex_rs::index::{Index, IndexBuilder};
use netex_rs::model::Date;
use netex_rs::stream::{self, ReadOptions, Selection};
use support::fragments::publication;
use support::timetable::{merged, serially, Point, Timetable};

fn day(text: &str) -> i64 {
    Date::from(text).days_since_epoch().expect("a date")
}

/// The days the journey `id` runs on, as dates of October 2026.
fn october_days(index: &Index, id: &str) -> Vec<i64> {
    let journey = index.journey_by_id(id).expect("the journey is indexed");
    index
        .days_of(journey)
        .map(|days| days - day("2026-09-30"))
        .collect()
}

fn valid_between(from: &str, to: &str) -> String {
    format!("<ValidBetween><FromDate>{from}T00:00:00</FromDate><ToDate>{to}T23:59:59</ToDate></ValidBetween>")
}

const TIMES: [(&str, Option<&str>, Option<&str>); 2] = [
    ("P1", None, Some("08:00:00")),
    ("P2", Some("08:30:00"), None),
];

#[test]
fn a_line_valid_in_two_seasons_governs_the_days_of_both() {
    // Valid from January to March and from September to December: a journey
    // of the week of 12 October runs on every day of it.
    let document = Timetable::new()
        .line("L", "1", Some(("2026-01-01", Some("2026-03-31"))))
        .stop_point("A")
        .stop_point("B")
        .pattern("P", "L", &[Point::at("P1", "A"), Point::at("P2", "B")])
        .day_type("DT")
        .uic_period("OP", "2026-10-12", "2026-10-18", "1111111")
        .assignment("DTA", 1, "OP", "DT", true)
        .journey("J", ("L", "1"), "P", &["DT"], &TIMES)
        .xml()
        .replacen(
            "</ValidBetween>",
            &format!(
                "</ValidBetween>{}",
                valid_between("2026-09-01", "2026-12-31")
            ),
            1,
        );
    let documents = [("A.xml".to_owned(), document)];
    for index in [serially(&documents), merged(&documents)] {
        assert_eq!(october_days(&index, "J"), [12, 13, 14, 15, 16, 17, 18]);
    }
}

#[test]
fn a_day_type_valid_in_two_seasons_runs_on_its_days_of_the_week_in_both() {
    // The French profile's day type: its validity and days of the week.
    let document = Timetable::new()
        .line("L", "1", None)
        .stop_point("A")
        .stop_point("B")
        .pattern("P", "L", &[Point::at("P1", "A"), Point::at("P2", "B")])
        .day_type_with(
            "WEEKEND",
            Some(("2026-10-10", "2026-10-14")),
            &["<DaysOfWeek>Saturday Sunday</DaysOfWeek>"],
        )
        .journey("J", ("L", "1"), "P", &["WEEKEND"], &TIMES)
        .xml()
        .replacen(
            "</ValidBetween>",
            &format!(
                "</ValidBetween>{}",
                valid_between("2026-10-20", "2026-10-25")
            ),
            1,
        );
    let documents = [("A.xml".to_owned(), document)];
    for index in [serially(&documents), merged(&documents)] {
        assert_eq!(october_days(&index, "J"), [10, 11, 24, 25]);
    }
}

/// A document of one service frame, its validity as `frame_validity` writes
/// it, holding stop point SP-A in `version` with `own_validity`.
fn stop_point_document(frame_validity: &str, version: &str, own_validity: &str) -> String {
    publication(&format!(
        concat!(
            r#"<ServiceFrame id="s-{version}" version="1">{frame_validity}"#,
            r#"<scheduledStopPoints><ScheduledStopPoint id="SP-A" version="{version}">{own_validity}<Name>A {version}</Name></ScheduledStopPoint></scheduledStopPoints>"#,
            "</ServiceFrame>"
        ),
        frame_validity = frame_validity,
        version = version,
        own_validity = own_validity,
    ))
}

/// A journey J every day of October 2026 calling at SP-A, naming no version
/// of it, and SP-B.
fn journey_document() -> String {
    Timetable::new()
        .line("L", "1", None)
        .stop_point("SP-B")
        .pattern(
            "P",
            "L",
            &[Point::at("P1", "SP-A"), Point::at("P2", "SP-B")],
        )
        .day_type("DT")
        .uic_period("OP", "2026-10-01", "2026-10-31", &"1".repeat(31))
        .assignment("DTA", 1, "OP", "DT", true)
        .journey("J", ("L", "1"), "P", &["DT"], &TIMES)
        .xml()
        .replace(
            r#"<ScheduledStopPointRef ref="SP-A" version="1"/>"#,
            r#"<ScheduledStopPointRef ref="SP-A" version="any"/>"#,
        )
}

/// The version of SP-A the journey calls at, in the index built for
/// 15 October 2026 from `documents` in both orders, read whole and lean:
/// one answer however they are read.
fn version_called_at_on_15_october(documents: &[(&str, String)]) -> String {
    let at = Date::from("2026-10-15");
    let options = ReadOptions::new(Selection::routing());
    let mut answers = Vec::new();
    for order in [
        documents.to_vec(),
        documents.iter().rev().cloned().collect(),
    ] {
        let mut whole = IndexBuilder::new().at(&at);
        let mut lean = IndexBuilder::new().at(&at);
        for (name, xml) in &order {
            whole.add(&netex_rs::read(xml).expect("the document reads"), name);
            lean.add_streamed(xml, &options, name)
                .expect("the document streams");
        }
        for index in [whole.finish(), lean.finish()] {
            let journey = index.journey_by_id("J").expect("the journey is indexed");
            let first = index.journey(journey).stop_points[0];
            answers.push(
                index
                    .stop_point_version(first)
                    .expect("the stop point states a version")
                    .to_owned(),
            );
        }
    }
    answers.dedup();
    assert_eq!(answers.len(), 1, "answers differ: {answers:?}");
    answers.remove(0)
}

#[test]
fn a_frame_valid_by_its_validity_conditions_alone_bounds_its_entities_to_them() {
    // Version 2 stands in a frame valid from January to June, as its
    // validity conditions state; version 1 in one valid from July on: on
    // 15 October, version 1 is valid.
    let first_half = format!(
        "<validityConditions>{}</validityConditions>",
        valid_between("2026-01-01", "2026-06-30")
    );
    let documents = [
        ("a.xml", stop_point_document(&first_half, "2", "")),
        (
            "b.xml",
            stop_point_document(&valid_between("2026-07-01", "2026-12-31"), "1", ""),
        ),
        ("c.xml", journey_document()),
    ];
    assert_eq!(version_called_at_on_15_october(&documents), "1");
}

#[test]
fn an_entity_is_valid_within_its_frame_whatever_its_own_validity_says() {
    // Version 2 states itself valid all year, in a frame valid from January
    // to March alone: on 15 October, version 1 is valid.
    let documents = [
        (
            "a.xml",
            stop_point_document(
                &valid_between("2026-01-01", "2026-03-31"),
                "2",
                &valid_between("2026-01-01", "2026-12-31"),
            ),
        ),
        ("b.xml", stop_point_document("", "1", "")),
        ("c.xml", journey_document()),
    ];
    assert_eq!(version_called_at_on_15_october(&documents), "1");
}

#[test]
fn a_frame_s_validity_is_each_it_states_directly_or_within_its_validity_conditions() {
    let document = stop_point_document(
        &format!(
            "<validityConditions>{}</validityConditions>{}",
            valid_between("2026-01-01", "2026-03-31"),
            valid_between("2026-09-01", "2026-12-31")
        ),
        "1",
        "",
    );
    let mut stated = Vec::new();
    stream::read(
        &document,
        &ReadOptions::new(Selection::routing()),
        |_, scope| {
            let frame = scope.frames().last().expect("the stop point's frame");
            stated = frame
                .valid_between
                .iter()
                .map(|valid| {
                    valid
                        .from_date
                        .as_ref()
                        .and_then(|from| from.date())
                        .expect("a first day")
                        .as_str()
                        .to_owned()
                })
                .collect();
            Ok(())
        },
    )
    .expect("the document streams");
    assert_eq!(stated, ["2026-01-01", "2026-09-01"]);
}
