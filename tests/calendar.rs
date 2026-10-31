//! The days a journey runs on, as every reader of the crate gets them: day
//! type assignments in their order, a later one over an earlier one; the days
//! of operating periods; and of those, the days the version of its line the
//! journey names governs.

mod support;

use netex_rs::calendar::{governed, Governed, Span};
use netex_rs::index::{Concern, Index};
use netex_rs::model::Date;
use support::timetable::{merged, serially, Point, Timetable};

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

/// A journey `J-<day type>` on each of `day_types`, over the line frame.
fn journeys_on(timetable: Timetable, day_types: &[&str]) -> Vec<(String, String)> {
    let timetable = day_types.iter().fold(
        line_frame(timetable.line("L", "1", None)),
        |timetable, day_type| {
            timetable.journey(
                &format!("J-{day_type}"),
                ("L", "1"),
                "P",
                &[day_type],
                &TIMES,
            )
        },
    );
    vec![("A.xml".to_owned(), timetable.xml())]
}

/// The French profile's day type: a validity and days of the week, its
/// assignments its exceptions. The weekend from 10 to 25 October runs on its
/// Saturdays and Sundays, but Sunday 18 taken away, and on Friday 16 added; a
/// day type of Sundays added Friday 16 runs on its Sundays and that Friday;
/// without a validity, the days of the week give no day.
#[test]
fn a_day_type_with_a_validity_and_days_of_the_week_runs_on_them_its_assignments_its_exceptions() {
    let documents = journeys_on(
        Timetable::new()
            .day_type_with(
                "WEEKEND",
                Some(("2026-10-10", "2026-10-25")),
                &["<DaysOfWeek>Saturday Sunday</DaysOfWeek>"],
            )
            .date_assignment("WEEKEND-1", 1, "2026-10-18", "WEEKEND", false)
            .date_assignment("WEEKEND-2", 2, "2026-10-16", "WEEKEND", true)
            .day_type_with(
                "SUNDAY",
                Some(("2026-10-10", "2026-10-25")),
                &["<DaysOfWeek>Sunday</DaysOfWeek>"],
            )
            .date_assignment("SUNDAY-1", 1, "2026-10-16", "SUNDAY", true)
            .day_type_with("OPEN", None, &["<DaysOfWeek>Saturday</DaysOfWeek>"]),
        &["WEEKEND", "SUNDAY", "OPEN"],
    );
    for index in [serially(&documents), merged(&documents)] {
        assert_eq!(october_days(&index, "J-WEEKEND"), [10, 11, 16, 17, 24, 25]);
        assert_eq!(october_days(&index, "J-SUNDAY"), [11, 16, 18, 25]);
        assert_eq!(october_days(&index, "J-OPEN"), Vec::<i64>::new());
        assert_eq!(
            findings(&index, Concern::JourneyWithoutDays),
            ["journey without an operating day: ServiceJourney J-OPEN in A.xml"]
        );
    }
}

/// A day type whose assignments give it a day of its own runs on its
/// assignments' days alone, its validity and days of the week describing
/// them: school days Monday to Friday from 12 to 23 October assigned 12, 13
/// and 19 October run on those three; a weekend day type assigned a UIC
/// period runs on the days of its bits, a Monday and a Tuesday among them.
#[test]
fn a_day_type_whose_assignments_give_it_its_own_days_runs_on_those_alone() {
    let documents = journeys_on(
        Timetable::new()
            .day_type_with(
                "SCHOOL",
                Some(("2026-10-12", "2026-10-23")),
                &["<DaysOfWeek>Monday Tuesday Wednesday Thursday Friday</DaysOfWeek>"],
            )
            .date_assignment("SCHOOL-1", 1, "2026-10-12", "SCHOOL", true)
            .date_assignment("SCHOOL-2", 1, "2026-10-13", "SCHOOL", true)
            .date_assignment("SCHOOL-3", 1, "2026-10-19", "SCHOOL", true)
            .day_type_with(
                "BITS",
                Some(("2026-10-10", "2026-10-25")),
                &["<DaysOfWeek>Weekend</DaysOfWeek>"],
            )
            .uic_period("WEEK", "2026-10-12", "2026-10-18", "1100011")
            .assignment("BITS-1", 1, "WEEK", "BITS", true),
        &["SCHOOL", "BITS"],
    );
    for index in [serially(&documents), merged(&documents)] {
        assert_eq!(october_days(&index, "J-SCHOOL"), [12, 13, 19]);
        assert_eq!(october_days(&index, "J-BITS"), [12, 13, 17, 18]);
    }
}

/// An operating period without day bits gives a day type the days of the
/// week its properties name, each property of day an alternative: the
/// Wednesdays of 12 to 25 October, its Mondays and Fridays; a day type naming
/// no day of the week gets every day of the period.
#[test]
fn an_operating_period_without_day_bits_gives_the_days_of_the_week_its_day_type_names() {
    let documents = journeys_on(
        Timetable::new()
            .day_type_with("WEDNESDAY", None, &["<DaysOfWeek>Wednesday</DaysOfWeek>"])
            .day_type_with(
                "MONDAY-FRIDAY",
                None,
                &[
                    "<DaysOfWeek>Monday</DaysOfWeek>",
                    "<DaysOfWeek>Friday</DaysOfWeek>",
                ],
            )
            .day_type("EVERY")
            .period("TWO-WEEKS", Some("2026-10-12"), "2026-10-25")
            .period("THREE-DAYS", Some("2026-10-12"), "2026-10-14")
            .assignment("WEDNESDAY-1", 1, "TWO-WEEKS", "WEDNESDAY", true)
            .assignment("MONDAY-FRIDAY-1", 1, "TWO-WEEKS", "MONDAY-FRIDAY", true)
            .assignment("EVERY-1", 1, "THREE-DAYS", "EVERY", true),
        &["WEDNESDAY", "MONDAY-FRIDAY", "EVERY"],
    );
    for index in [serially(&documents), merged(&documents)] {
        assert_eq!(october_days(&index, "J-WEDNESDAY"), [14, 21]);
        assert_eq!(october_days(&index, "J-MONDAY-FRIDAY"), [12, 16, 19, 23]);
        assert_eq!(october_days(&index, "J-EVERY"), [12, 13, 14]);
    }
}

/// Properties naming more than days of the week are not read, and where
/// they would decide the days — a day type of summer Saturdays with a
/// validity and no assignment, one of autumn days assigned a period without
/// day bits — that is a finding; the assignments stand as they are. A
/// holiday day type assigned a single date needs no property read.
#[test]
fn properties_naming_more_than_days_of_the_week_are_a_finding_where_they_would_decide() {
    let documents = journeys_on(
        Timetable::new()
            .day_type_with(
                "SUMMER",
                Some(("2026-10-10", "2026-10-25")),
                &["<DaysOfWeek>Saturday</DaysOfWeek><Seasons>Summer</Seasons>"],
            )
            .day_type_with("AUTUMN", None, &["<Seasons>Autumn</Seasons>"])
            .period("THREE-DAYS", Some("2026-10-12"), "2026-10-14")
            .assignment("AUTUMN-1", 1, "THREE-DAYS", "AUTUMN", true)
            .day_type_with(
                "HOLIDAY",
                None,
                &["<HolidayTypes>NationalHoliday</HolidayTypes>"],
            )
            .date_assignment("HOLIDAY-1", 1, "2026-10-31", "HOLIDAY", true),
        &["SUMMER", "AUTUMN", "HOLIDAY"],
    );
    for index in [serially(&documents), merged(&documents)] {
        assert_eq!(october_days(&index, "J-SUMMER"), Vec::<i64>::new());
        assert_eq!(october_days(&index, "J-AUTUMN"), [12, 13, 14]);
        assert_eq!(october_days(&index, "J-HOLIDAY"), [31]);
        assert_eq!(
            findings(&index, Concern::PropertiesOfDayUnread),
            [
                "day type whose properties name more than days of the week, its assignments' days alone: DayType AUTUMN in A.xml",
                "day type whose properties name more than days of the week, its assignments' days alone: DayType SUMMER in A.xml"
            ]
        );
    }
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
