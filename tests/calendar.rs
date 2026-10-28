//! The days a journey runs on, as every reader of the crate gets them: day
//! type assignments in their order, a later one over an earlier one; the days
//! of operating periods; and of those, the days the version of its line the
//! journey names governs.

mod support;

use netex_rs::calendar::{governed, Governed, Span};
use netex_rs::index::{Concern, Index};
use netex_rs::model::Date;
use support::timetable::{serially, Point, Timetable};

fn day(text: &str) -> i64 {
    Date::from(text).days_since_epoch().expect("a date")
}

/// The days the journey `id` runs on, as dates of October 2026.
fn october_days(index: &Index, id: &str) -> Vec<i64> {
    let journey = index.journey_by_id(id).expect("the journey");
    index
        .days_of(journey)
        .map(|days| days - day("2026-09-30"))
        .collect()
}

fn findings(index: &Index, concern: Concern) -> Vec<String> {
    index
        .findings()
        .filter(|finding| finding.concern == concern)
        .map(|finding| finding.to_string())
        .collect()
}

/// A line `L` of two stop points with its pattern, ready for journeys.
fn line_frame(timetable: Timetable) -> Timetable {
    timetable.stop_point("A").stop_point("B").pattern(
        "P",
        "L",
        &[Point::at("P1", "A"), Point::at("P2", "B")],
    )
}

const TIMES: [(&str, Option<&str>, Option<&str>); 2] = [
    ("P1", None, Some("08:00:00")),
    ("P2", Some("08:30:00"), None),
];

#[test]
fn assignments_apply_in_their_order_a_later_one_over_an_earlier_one() {
    // Written in reverse: the assignment taking 13 and 14 October away comes
    // first in the document but second in order.
    let document = line_frame(Timetable::new().line("L", "1", None))
        .day_type("DT")
        .uic_period("WEEK", "2026-10-12", "2026-10-18", "1111111")
        .uic_period("TWO", "2026-10-13", "2026-10-14", "11")
        .assignment("DTA-2", 2, "TWO", "DT", false)
        .assignment("DTA-1", 1, "WEEK", "DT", true)
        .journey("J", ("L", "1"), "P", &["DT"], &TIMES)
        .xml();
    let index = serially(&[("A.xml".to_owned(), document)]);
    assert_eq!(october_days(&index, "J"), [12, 15, 16, 17, 18]);
}

/// Versions of a line as the Austrian deliveries write them: version 1 from
/// 12 October on, version 2 on 14 and 15 October, version 3 from 17 October on.
fn three_versions() -> Timetable {
    let timetable = Timetable::new()
        .line("L", "1", Some(("2026-10-12", None)))
        .line("L", "2", Some(("2026-10-14", Some("2026-10-15"))))
        .line("L", "3", Some(("2026-10-17", None)));
    line_frame(timetable)
        .day_type("WEEK")
        .day_type("WEEKEND")
        .uic_period("OP-WEEK", "2026-10-12", "2026-10-18", "1111111")
        .uic_period("OP-WEEKEND", "2026-10-12", "2026-10-18", "0000011")
        .assignment("DTA-1", 1, "OP-WEEK", "WEEK", true)
        .assignment("DTA-2", 1, "OP-WEEKEND", "WEEKEND", true)
}

#[test]
fn a_journey_runs_on_the_days_the_version_of_its_line_it_names_governs() {
    let document = three_versions()
        .journey("J1", ("L", "1"), "P", &["WEEK"], &TIMES)
        .journey("J2", ("L", "2"), "P", &["WEEK"], &TIMES)
        .journey("J3", ("L", "3"), "P", &["WEEK"], &TIMES)
        .xml();
    let index = serially(&[("NX-PI_01_at_ktn_LINE_ktn_0100A.xml".to_owned(), document)]);
    // A version governs the days it is valid on, but those a version
    // beginning later is valid on: version 1 loses 14 and 15 October to
    // version 2 and every day from 17 October on to version 3.
    assert_eq!(october_days(&index, "J1"), [12, 13, 16]);
    assert_eq!(october_days(&index, "J2"), [14, 15]);
    assert_eq!(october_days(&index, "J3"), [17, 18]);
    assert_eq!(
        findings(&index, Concern::DaysOutsideLineVersion),
        [
            "days its line version does not govern, taken away: ServiceJourney J1 in NX-PI_01_at_ktn_LINE_ktn_0100A.xml (L)",
            "days its line version does not govern, taken away: ServiceJourney J2 in NX-PI_01_at_ktn_LINE_ktn_0100A.xml (L)",
            "days its line version does not govern, taken away: ServiceJourney J3 in NX-PI_01_at_ktn_LINE_ktn_0100A.xml (L)",
        ]
    );
}

#[test]
fn a_journey_on_no_day_its_line_version_governs_is_left_out() {
    // Version 2 governs 14 and 15 October; the journey runs at weekends.
    let document = three_versions()
        .journey("J", ("L", "2"), "P", &["WEEKEND"], &TIMES)
        .xml();
    let index = serially(&[("A.xml".to_owned(), document)]);
    assert_eq!(index.journey_by_id("J"), None);
    assert_eq!(index.journey_count(), 0);
    assert_eq!(
        findings(&index, Concern::JourneyOutsideLineVersion),
        ["no day its line version governs, left out: ServiceJourney J in A.xml (L)"]
    );
}

#[test]
fn a_line_valid_until_august_governs_no_day_in_october() {
    // The Linz AG's `e25` lines: one version, valid to 30 August; their
    // journeys' day types run on into the autumn.
    let document =
        line_frame(Timetable::new().line("L", "any", Some(("2026-01-02", Some("2026-08-30")))))
            .day_type("DT")
            .uic_period("OP", "2026-08-24", "2026-10-18", &"1".repeat(56))
            .assignment("DTA", 1, "OP", "DT", true)
            .journey("J", ("L", "any"), "P", &["DT"], &TIMES)
            .xml();
    let index = serially(&[("A.xml".to_owned(), document)]);
    let journey = index.journey_by_id("J").expect("it runs in August");
    let last = index.days_of(journey).last().expect("some day");
    assert_eq!(last, day("2026-08-30"));
    assert_eq!(index.days_of(journey).count(), 7, "24 to 30 August");
}

#[test]
fn a_line_without_validity_governs_every_day() {
    let document = line_frame(Timetable::new().line("L", "1", None))
        .day_type("DT")
        .uic_period("OP", "2026-10-12", "2026-10-18", "1010101")
        .assignment("DTA", 1, "OP", "DT", true)
        .journey("J", ("L", "1"), "P", &["DT"], &TIMES)
        .xml();
    let index = serially(&[("A.xml".to_owned(), document)]);
    assert_eq!(october_days(&index, "J"), [12, 14, 16, 18]);
    for concern in [
        Concern::DaysOutsideLineVersion,
        Concern::JourneyOutsideLineVersion,
    ] {
        assert_eq!(index.finding_count(concern), 0);
    }
}

#[test]
fn a_period_without_a_first_day_or_with_other_bits_than_days_is_a_finding() {
    let document = line_frame(Timetable::new().line("L", "1", None))
        .day_type("DT")
        .period("PLAIN", None, "2026-10-18")
        .uic_period("SHORT", "2026-10-12", "2026-10-18", "111")
        .assignment("DTA-1", 1, "PLAIN", "DT", true)
        .assignment("DTA-2", 2, "SHORT", "DT", true)
        .journey("J", ("L", "1"), "P", &["DT"], &TIMES)
        .xml();
    let index = serially(&[("A.xml".to_owned(), document)]);
    assert_eq!(
        october_days(&index, "J"),
        [12, 13, 14],
        "the bits are read from the first day as far as they go"
    );
    assert_eq!(
        findings(&index, Concern::PeriodWithoutFirstDay),
        ["operating period without a first day, no days: OperatingPeriod PLAIN in A.xml"]
    );
    assert_eq!(
        findings(&index, Concern::DayBitsLengthDiffers),
        ["day bits other than the period's days, read from its first day: UicOperatingPeriod SHORT in A.xml"]
    );
}

#[test]
fn a_journey_without_an_operating_day_is_kept_and_a_finding() {
    let document = line_frame(Timetable::new().line("L", "1", None))
        .journey("J", ("L", "1"), "P", &[], &TIMES)
        .xml();
    let index = serially(&[("A.xml".to_owned(), document)]);
    assert!(index.journey_by_id("J").is_some());
    assert_eq!(
        findings(&index, Concern::JourneyWithoutDays),
        ["journey without an operating day: ServiceJourney J in A.xml"]
    );
}

#[test]
fn the_rule_of_line_versions_as_a_function() {
    let open = Span {
        from: None,
        to: None,
    };
    assert_eq!(governed(&[open], 0), Governed::Every);
    assert!(Governed::Every.contains(i64::MIN));
    let span = |from, to| Span {
        from: Some(from),
        to,
    };
    // An earlier-beginning version takes nothing from a later one, and one
    // without a beginning takes nothing from any.
    let versions = [
        span(15, Some(40)),
        span(5, Some(17)),
        Span {
            from: None,
            to: Some(50),
        },
    ];
    assert_eq!(governed(&versions, 0), Governed::Spans(vec![(15, 40)]));
    assert_eq!(governed(&versions, 1), Governed::Spans(vec![(5, 14)]));
    assert_eq!(
        governed(&versions, 2),
        Governed::Spans(vec![(i64::MIN, 4), (41, 50)])
    );
}

/// Calendars of day types and UIC periods, read with journeys of ours on each
/// of their day types: two day types sharing one period, one of them given a
/// second (`tests/fixtures/fragments/frames/calendar-periods.xml`), and a day
/// type of Wednesdays whose period sets every seventh day
/// (`calendar-weekday.xml`).
#[test]
fn calendars_of_uic_periods_give_the_days_their_periods_state() {
    let journeys = |day_types: &[&str]| {
        let mut timetable = line_frame(Timetable::new().line("L", "1", None));
        for day_type in day_types {
            timetable = timetable.journey(
                &format!("J-{day_type}"),
                ("L", "1"),
                "P",
                &[day_type],
                &TIMES,
            );
        }
        timetable.xml()
    };
    let index = serially(&[
        (
            "A.xml".to_owned(),
            support::fragments::document("calendar-periods.xml"),
        ),
        (
            "B.xml".to_owned(),
            journeys(&["ex:DayType:A", "ex:DayType:B"]),
        ),
    ]);
    let days = |id: &str| -> Vec<i64> {
        let journey = index.journey_by_id(id).expect("the journey");
        index
            .days_of(journey)
            .map(|d| d - day("2026-03-01"))
            .collect()
    };
    // The Tuesday and Thursday of the week of 2 March, the Saturday and
    // Sunday of the week after.
    assert_eq!(days("J-ex:DayType:A"), [2, 4, 13, 14]);
    assert_eq!(days("J-ex:DayType:B"), [2, 4]);
    assert!(index
        .unresolved()
        .iter()
        .any(|u| u.element == "DayTypeRef" && u.id == "ex:DayType:gone"));
    let index = serially(&[
        (
            "A.xml".to_owned(),
            support::fragments::document("calendar-weekday.xml"),
        ),
        ("B.xml".to_owned(), journeys(&["ex:DayType:wednesday"])),
    ]);
    let journey = index
        .journey_by_id("J-ex:DayType:wednesday")
        .expect("the journey");
    let wednesdays: Vec<i64> = index.days_of(journey).collect();
    assert_eq!(
        wednesdays,
        ["2026-02-04", "2026-02-11", "2026-02-18", "2026-02-25"].map(day)
    );
}

#[test]
fn journeys_running_on_the_same_days_share_one_calendar() {
    let index = support::timetable::serially(&support::timetable::two_calendars());
    let e25 = index.journey_by_id("J-e25").expect("the first journey");
    let f26 = index.journey_by_id("J-f26").expect("the second journey");
    let (first, second) = (index.calendar_of(e25), index.calendar_of(f26));
    assert_ne!(first, second, "Monday and Tuesday are other days");
    assert_eq!(index.calendar_count(), 2);
    for (journey, calendar) in [(e25, first), (f26, second)] {
        assert_eq!(
            index.calendar_days(calendar).collect::<Vec<_>>(),
            index.days_of(journey).collect::<Vec<_>>()
        );
    }
    assert_eq!(
        index.calendar_days(first).collect::<Vec<_>>(),
        [day("2026-10-12")]
    );
}

#[test]
fn a_day_since_1970_is_the_date_it_counts_to() {
    for text in [
        "1970-01-01",
        "1969-12-31",
        "2000-02-29",
        "2026-10-12",
        "2100-03-01",
    ] {
        let date = Date::from(text);
        let days = date.days_since_epoch().expect("a date");
        assert_eq!(Date::from_days_since_epoch(days), date, "{text}");
    }
    assert_eq!(Date::from_days_since_epoch(0).as_str(), "1970-01-01");
    assert_eq!(Date::from_days_since_epoch(20_738).as_str(), "2026-10-12");
}
