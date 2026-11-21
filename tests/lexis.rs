//! What a date, a day bit, the bound of a period and a duration say, read by
//! the lexis of their schema types: no day its month does not have, no day
//! bit of white space, no bound read a day off, no duration from a form that
//! is none — a form that says nothing gives nothing, and a finding where the
//! index reads it.

mod support;

use netex_rs::calendar::{bound_day, period_days, PeriodIssue, FIRST_DAY, LAST_DAY};
use netex_rs::index::Index;
use netex_rs::model::{Date, DateTime, Duration};
use support::fragments::publication;
use support::timetable::{merged, serially, Point, Timetable};

/// The days the journey `id` runs on, as dates.
fn dates(index: &Index, id: &str) -> Vec<String> {
    let journey = index.journey_by_id(id).expect("the journey is indexed");
    index
        .days_of(journey)
        .map(|day| Date::from_days_since_epoch(day).as_str().to_owned())
        .collect()
}

/// The findings of the concern named `code`, as text.
fn findings(index: &Index, code: &str) -> Vec<String> {
    index
        .findings()
        .filter(|finding| finding.concern.code() == code)
        .map(|finding| finding.to_string())
        .collect()
}

const TIMES: [(&str, Option<&str>, Option<&str>); 2] = [
    ("P1", None, Some("08:00:00")),
    ("P2", Some("08:30:00"), None),
];

/// A line `L` of two stop points with its pattern, ready for journeys.
fn line_frame() -> Timetable {
    Timetable::new()
        .line("L", "1", None)
        .stop_point("A")
        .stop_point("B")
        .pattern("P", "L", &[Point::at("P1", "A"), Point::at("P2", "B")])
}

#[test]
fn a_date_its_month_does_not_have_is_no_day() {
    for text in [
        "2026-02-29",
        "2026-02-30",
        "2026-04-31",
        "2026-11-31",
        "2100-02-29",
    ] {
        assert_eq!(Date::from(text).ymd(), None, "{text}");
        assert_eq!(Date::from(text).days_since_epoch(), None, "{text}");
        let instant = format!("{text}T00:00:00");
        assert_eq!(DateTime::from(instant.as_str()).date(), None, "{text}");
    }
    for text in ["2024-02-29", "2000-02-29", "2026-04-30", "2026-12-31"] {
        assert!(Date::from(text).days_since_epoch().is_some(), "{text}");
    }
}

#[test]
fn an_assignment_to_a_date_its_month_does_not_have_gives_no_day_and_says_so() {
    // 31 November is no day; read as one, it was the first of December.
    let document = line_frame()
        .day_type("DT")
        .date_assignment("DTA-1", 1, "2026-11-30", "DT", true)
        .date_assignment("DTA-2", 2, "2026-11-31", "DT", true)
        .journey("J", ("L", "1"), "P", &["DT"], &TIMES)
        .xml();
    let documents = [("A.xml".to_owned(), document)];
    for index in [serially(&documents), merged(&documents)] {
        assert_eq!(dates(&index, "J"), ["2026-11-30"]);
        assert_eq!(
            findings(&index, "date-of-no-day"),
            ["date that is no day of the calendar, no day there: DayTypeAssignment DT in A.xml"]
        );
    }
}

#[test]
fn a_period_bounded_by_a_date_its_month_does_not_have_gives_no_day_and_says_so() {
    let document = line_frame()
        .day_type("DT")
        .period("OP", Some("2026-11-28"), "2026-11-31")
        .assignment("DTA", 1, "OP", "DT", true)
        .journey("J", ("L", "1"), "P", &["DT"], &TIMES)
        .xml();
    let documents = [("A.xml".to_owned(), document)];
    for index in [serially(&documents), merged(&documents)] {
        assert_eq!(dates(&index, "J"), Vec::<String>::new());
        assert_eq!(
            findings(&index, "date-of-no-day"),
            ["date that is no day of the calendar, no day there: OperatingPeriod OP in A.xml"]
        );
    }
}

#[test]
fn white_space_among_day_bits_is_no_day() {
    // `ValidDayBits` is a normalized string: a line break or an indent within
    // it is no bit.
    let all = FIRST_DAY..=LAST_DAY;
    let wrapped = period_days(Some(100), Some(102), Some("\n  101\n"), all.clone());
    assert_eq!(wrapped.days, [100, 102]);
    assert_eq!(wrapped.issue, None);
    let split = period_days(Some(100), Some(104), Some("10 1\t01"), all);
    assert_eq!(split.days, [100, 102, 104]);
    assert_eq!(split.issue, None);
}

#[test]
fn a_day_bit_other_than_zero_or_one_is_no_day_and_a_finding() {
    let document = line_frame()
        .day_type("DT")
        .uic_period("OP", "2026-10-12", "2026-10-14", "1x1")
        .assignment("DTA", 1, "OP", "DT", true)
        .journey("J", ("L", "1"), "P", &["DT"], &TIMES)
        .xml();
    let documents = [("A.xml".to_owned(), document)];
    for index in [serially(&documents), merged(&documents)] {
        assert_eq!(dates(&index, "J"), ["2026-10-12", "2026-10-14"]);
        assert_eq!(
            findings(&index, "day-bit-other-than-zero-or-one"),
            ["day bit other than 0 or 1, no day there: UicOperatingPeriod OP in A.xml"]
        );
    }
}

fn bound(text: &str) -> Option<String> {
    bound_day(&DateTime::from(text)).map(|day| Date::from_days_since_epoch(day).as_str().to_owned())
}

#[test]
fn a_bound_is_the_next_day_only_where_it_may_be_a_local_midnight_written_in_utc() {
    // The Belgian deliveries' bounds: the midnight beginning a local day,
    // written in UTC.
    assert_eq!(bound("2026-10-21T22:00:00Z").as_deref(), Some("2026-10-22"));
    assert_eq!(
        bound("2026-10-29T23:00:00+00:00").as_deref(),
        Some("2026-10-30")
    );
    // The end of a day, or any time written in a zone of its own, is the day
    // of its date.
    for text in [
        "2026-12-12T23:59:59+01:00",
        "2026-12-12T23:59:59Z",
        "2026-12-12T12:00:00+01:00",
        "2026-12-12T22:30:00Z",
        "2026-12-12T23:00:00+01:00",
        "2026-12-12T00:00:00Z",
        "2026-12-12T23:00:00",
    ] {
        assert_eq!(bound(text).as_deref(), Some("2026-12-12"), "{text}");
    }
}

#[test]
fn an_availability_condition_counts_its_bits_from_the_day_a_period_of_the_same_bound_does() {
    // Both state their first day as the midnight beginning 12 October in
    // Brussels, in UTC; both give their days from 12 October on.
    let condition = concat!(
        r#"<ServiceFrame id="s" version="1"><lines><Line id="L" version="1"><Name>L</Name><TransportMode>bus</TransportMode></Line></lines>"#,
        r#"<scheduledStopPoints><ScheduledStopPoint id="A" version="1"><Name>A</Name></ScheduledStopPoint><ScheduledStopPoint id="B" version="1"><Name>B</Name></ScheduledStopPoint></scheduledStopPoints>"#,
        r#"<journeyPatterns><ServiceJourneyPattern id="P" version="1"><RouteView><LineRef ref="L" version="1"/></RouteView><pointsInSequence>"#,
        r#"<StopPointInJourneyPattern id="P1" version="1" order="1"><ScheduledStopPointRef ref="A" version="1"/></StopPointInJourneyPattern>"#,
        r#"<StopPointInJourneyPattern id="P2" version="1" order="2"><ScheduledStopPointRef ref="B" version="1"/></StopPointInJourneyPattern>"#,
        "</pointsInSequence></ServiceJourneyPattern></journeyPatterns></ServiceFrame>",
        r#"<TimetableFrame id="t" version="1"><vehicleJourneys><ServiceJourney id="J" version="1">"#,
        r#"<validityConditions><AvailabilityCondition id="AC" version="1"><FromDate>2026-10-11T22:00:00Z</FromDate><ToDate>2026-10-17T22:00:00Z</ToDate><ValidDayBits>1010000</ValidDayBits></AvailabilityCondition></validityConditions>"#,
        r#"<ServiceJourneyPatternRef ref="P" version="1"/><LineRef ref="L" version="1"/><passingTimes>"#,
        r#"<TimetabledPassingTime version="1"><StopPointInJourneyPatternRef ref="P1" version="1"/><DepartureTime>08:00:00</DepartureTime></TimetabledPassingTime>"#,
        r#"<TimetabledPassingTime version="1"><StopPointInJourneyPatternRef ref="P2" version="1"/><ArrivalTime>08:30:00</ArrivalTime></TimetabledPassingTime>"#,
        "</passingTimes></ServiceJourney></vehicleJourneys></TimetableFrame>"
    );
    let documents = [("A.xml".to_owned(), publication(condition))];
    for index in [serially(&documents), merged(&documents)] {
        assert_eq!(dates(&index, "J"), ["2026-10-12", "2026-10-14"]);
    }
}

#[test]
fn a_line_version_begins_on_the_day_its_bound_names_as_a_period_s_does() {
    // Version 2 begins at the midnight beginning 15 October in Brussels,
    // written in UTC: it governs 15 October, version 1 the 14th.
    let document = Timetable::new()
        .line("L", "1", Some(("2026-10-01", None)))
        .line("L", "2", Some(("2026-10-15", None)))
        .stop_point("A")
        .stop_point("B")
        .pattern("P", "L", &[Point::at("P1", "A"), Point::at("P2", "B")])
        .day_type("DT")
        .uic_period("OP", "2026-10-14", "2026-10-15", "11")
        .assignment("DTA", 1, "OP", "DT", true)
        .journey("J1", ("L", "1"), "P", &["DT"], &TIMES)
        .journey("J2", ("L", "2"), "P", &["DT"], &TIMES)
        .xml()
        .replace(
            "<FromDate>2026-10-15T00:00:00</FromDate>",
            "<FromDate>2026-10-14T22:00:00Z</FromDate>",
        );
    let documents = [("A.xml".to_owned(), document)];
    for index in [serially(&documents), merged(&documents)] {
        assert_eq!(dates(&index, "J1"), ["2026-10-14"]);
        assert_eq!(dates(&index, "J2"), ["2026-10-15"]);
    }
}

#[test]
fn a_duration_is_read_in_the_order_of_its_parts_or_not_at_all() {
    let milliseconds = |text: &str| Duration::from(text).milliseconds();
    assert_eq!(milliseconds("PT1H30M"), Some(5_400_000));
    assert_eq!(milliseconds("P1DT2H"), Some(93_600_000));
    assert_eq!(milliseconds("PT0.5S"), Some(500));
    assert_eq!(milliseconds("PT1.25S"), Some(1250));
    assert_eq!(milliseconds("P2D"), Some(172_800_000));
    assert_eq!(milliseconds("PT0S"), Some(0));
    // White space around the form is collapsed away, as the schema type says.
    assert_eq!(milliseconds(" PT2M\n"), Some(120_000));
    for text in [
        "PT1.5H",
        "PT0.5M",
        "P",
        "PT",
        "P1DT",
        "PT1H1H",
        "PT1S1H",
        "PT1M1H",
        "P1D1D",
        "PT1 H",
        "P999999999999999D",
        "PT18446744073709551615S",
        "-PT5M",
        "P1M",
        "P1Y",
    ] {
        assert_eq!(milliseconds(text), None, "{text}");
    }
}

#[test]
fn a_walk_of_a_duration_that_is_none_is_no_walk_and_a_finding() {
    let walk = concat!(
        r#"<SiteFrame id="g" version="1"><stopPlaces>"#,
        r#"<StopPlace id="X" version="1"><Name>X</Name></StopPlace><StopPlace id="Y" version="1"><Name>Y</Name></StopPlace>"#,
        "</stopPlaces></SiteFrame>",
        r#"<ServiceFrame id="s" version="1"><connections><SiteConnection id="C" version="1">"#,
        "<WalkTransferDuration><DefaultDuration>PT1.5M</DefaultDuration></WalkTransferDuration>",
        r#"<From><StopPlaceRef ref="X" version="1"/></From><To><StopPlaceRef ref="Y" version="1"/></To>"#,
        "</SiteConnection></connections></ServiceFrame>"
    );
    let documents = [("A.xml".to_owned(), publication(walk))];
    for index in [serially(&documents), merged(&documents)] {
        assert_eq!(index.transfer_count(), 0);
        assert_eq!(
            findings(&index, "walk-without-duration"),
            ["walk stating no duration, no transfer: SiteConnection C in A.xml"]
        );
    }
}

#[test]
fn a_day_of_a_period_without_a_bit_is_available_and_a_bit_past_its_last_day_no_day() {
    // "If bit is missing, assume available": the schema's `ValidDayBits`.
    let all = FIRST_DAY..=LAST_DAY;
    let short = period_days(Some(100), Some(104), Some("10"), all.clone());
    assert_eq!(short.days, [100, 102, 103, 104]);
    assert_eq!(
        short.issue,
        Some(PeriodIssue::BitsOtherThanDays { bits: 2, days: 5 })
    );
    let long = period_days(Some(100), Some(102), Some("10111"), all.clone());
    assert_eq!(long.days, [100, 102], "the period ends on its last day");
    let unbounded = period_days(Some(100), None, Some("1011"), all);
    assert_eq!(
        unbounded.days,
        [100, 102, 103],
        "without a last day, its bits"
    );
}
