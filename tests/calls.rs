//! Which points of its pattern a journey calls at, and what a passenger may do
//! there: a point the vehicle passes through is no call, a point before the
//! journey's first or after its last passing time is none either, and each
//! call carries whether one may board and alight there.

mod support;

use netex_rs::index::{CallUse, Concern, Index, JourneyId};
use support::timetable::{merged, serially, Point, Timetable};

/// The stop point identifiers a journey calls at.
fn calls(index: &Index, journey: JourneyId) -> Vec<&str> {
    index
        .journey(journey)
        .stop_points
        .iter()
        .map(|&point| index.stop_point(point).id)
        .collect()
}

fn findings(index: &Index, concern: Concern) -> Vec<String> {
    index
        .findings()
        .filter(|finding| finding.concern == concern)
        .map(|finding| finding.to_string())
        .collect()
}

/// A train of the Austrian railway's kind: four points, the two in the middle
/// passed through — `StopUse` `passthrough`, though `ForBoarding` and
/// `ForAlighting` say `true` —, passing times at the first and the last only.
fn passing_train() -> Vec<(String, String)> {
    let point = |id, stop_point, stop_use| Point {
        stop_use,
        boarding: Some(true),
        alighting: Some(true),
        ..Point::at(id, stop_point)
    };
    let document = Timetable::new()
        .line("L", "1", None)
        .stop_point("A")
        .stop_point("B")
        .stop_point("C")
        .stop_point("D")
        .pattern(
            "P",
            "L",
            &[
                point("P1", "A", Some("access")),
                point("P2", "B", Some("passthrough")),
                point("P3", "C", Some("passthrough")),
                point("P4", "D", Some("access")),
            ],
        )
        .day_type("DT")
        .uic_period("OP", "2026-10-12", "2026-10-18", "1111111")
        .assignment("DTA", 1, "OP", "DT", true)
        .journey(
            "J",
            ("L", "1"),
            "P",
            &["DT"],
            &[
                ("P1", None, Some("20:00:00")),
                ("P4", Some("20:40:00"), None),
            ],
        )
        .xml();
    vec![(
        "NX-PI_01_at_obb_LINE_obb_3-499-M-j26.xml".to_owned(),
        document,
    )]
}

#[test]
fn a_point_the_vehicle_passes_through_is_no_call() {
    let documents = passing_train();
    for index in [serially(&documents), merged(&documents)] {
        let journey = index.journey_by_id("J").expect("the train");
        assert_eq!(calls(&index, journey), ["A", "D"]);
        let times: Vec<_> = index
            .journey(journey)
            .passing_times
            .iter()
            .map(|time| (time.arrival, time.departure))
            .collect();
        assert_eq!(
            times,
            [(None, Some(20 * 3600)), (Some(20 * 3600 + 40 * 60), None)]
        );
        assert_eq!(
            findings(&index, Concern::PassedThrough),
            [
                "point passed through, no call: StopPointInJourneyPattern P2 in NX-PI_01_at_obb_LINE_obb_3-499-M-j26.xml (P)",
                "point passed through, no call: StopPointInJourneyPattern P3 in NX-PI_01_at_obb_LINE_obb_3-499-M-j26.xml (P)",
            ]
        );
    }
}

#[test]
fn each_call_says_whether_one_may_board_and_alight() {
    let point = |id, stop_point, stop_use, boarding, alighting| Point {
        stop_use,
        boarding,
        alighting,
        ..Point::at(id, stop_point)
    };
    let document = Timetable::new()
        .line("L", "1", None)
        .stop_point("A")
        .stop_point("B")
        .stop_point("C")
        .stop_point("D")
        .pattern(
            "P",
            "L",
            &[
                point("P1", "A", None, None, Some(false)),
                point(
                    "P2",
                    "B",
                    Some("noBoardingOrAlighting"),
                    Some(true),
                    Some(true),
                ),
                point("P3", "C", Some("access"), Some(false), None),
                point("P4", "D", None, Some(false), None),
            ],
        )
        .journey(
            "J",
            ("L", "1"),
            "P",
            &[],
            &[
                ("P1", None, Some("08:00:00")),
                ("P2", Some("08:10:00"), Some("08:11:00")),
                ("P3", Some("08:20:00"), Some("08:21:00")),
                ("P4", Some("08:30:00"), None),
            ],
        )
        .xml();
    let index = serially(&[("D.xml".to_owned(), document)]);
    let journey = index.journey(index.journey_by_id("J").expect("the journey"));
    let uses: Vec<(bool, bool)> = journey
        .uses
        .iter()
        .map(
            |CallUse {
                 boarding,
                 alighting,
             }| (*boarding, *alighting),
        )
        .collect();
    assert_eq!(
        uses,
        [(true, false), (false, false), (false, true), (false, true)],
        "as stated, both true where nothing is; noBoardingOrAlighting is neither, whatever the flags say"
    );
}

/// A run of an Upper Austrian kind: the pattern of the whole line, five
/// points, and passing times for the second to the fourth only.
fn short_run(times: &[(&'static str, Option<&'static str>, Option<&'static str>)]) -> String {
    Timetable::new()
        .line("L", "1", None)
        .stop_point("A")
        .stop_point("B")
        .stop_point("C")
        .stop_point("D")
        .stop_point("E")
        .pattern(
            "P",
            "L",
            &[
                Point::at("P1", "A"),
                Point::at("P2", "B"),
                Point::at("P3", "C"),
                Point::at("P4", "D"),
                Point::at("P5", "E"),
            ],
        )
        .journey("J", ("L", "1"), "P", &[], times)
        .xml()
}

#[test]
fn a_journey_shorter_than_its_pattern_calls_from_its_first_to_its_last_passing_time() {
    let document = short_run(&[
        ("P2", None, Some("11:00:00")),
        ("P3", None, None),
        ("P4", Some("11:51:00"), None),
    ]);
    let index = serially(&[("NX-PI_01_at_oov_LINE_oov_12-1-j26.xml".to_owned(), document)]);
    let journey = index.journey_by_id("J").expect("the short run is kept");
    assert_eq!(
        calls(&index, journey),
        ["B", "C", "D"],
        "an untimed point between two timed ones stays a call"
    );
    assert_eq!(index.journey(journey).passing_times[1].departure, None);
    assert_eq!(
        findings(&index, Concern::UntimedEnds),
        ["pattern points before its first or after its last passing time, no calls: ServiceJourney J in NX-PI_01_at_oov_LINE_oov_12-1-j26.xml"]
    );
}

#[test]
fn a_journey_without_two_timed_calls_is_left_out() {
    let document = short_run(&[("P2", None, Some("11:00:00"))]);
    let index = serially(&[("A.xml".to_owned(), document)]);
    assert_eq!(index.journey_count(), 0);
    assert_eq!(index.journey_by_id("J"), None);
    assert_eq!(
        findings(&index, Concern::TooFewTimedCalls),
        ["journey without two timed calls, left out: ServiceJourney J in A.xml"]
    );
}

/// Three journeys over a pattern of three points whose passing times name no
/// point, as NeTEx 1.x allows and the French deliveries write them: each time
/// is at the point of its position. Two times on three points are the first
/// two, and three points for four times take the first three; either is a
/// finding.
#[test]
fn a_passing_time_naming_no_point_is_at_the_point_of_its_position() {
    let document = Timetable::new()
        .line("L", "1", None)
        .stop_point("A")
        .stop_point("B")
        .stop_point("C")
        .pattern(
            "P",
            "L",
            &[
                Point::at("P1", "A"),
                Point::at("P2", "B"),
                Point::at("P3", "C"),
            ],
        )
        .journey_by_position(
            "J",
            ("L", "1"),
            "P",
            &[],
            &[
                (None, Some("08:00:00")),
                (Some("08:10:00"), Some("08:11:00")),
                (Some("08:20:00"), None),
            ],
        )
        .journey_by_position(
            "SHORT",
            ("L", "1"),
            "P",
            &[],
            &[(None, Some("09:00:00")), (Some("09:10:00"), None)],
        )
        .journey_by_position(
            "LONG",
            ("L", "1"),
            "P",
            &[],
            &[
                (None, Some("10:00:00")),
                (Some("10:10:00"), Some("10:11:00")),
                (Some("10:20:00"), Some("10:21:00")),
                (Some("10:30:00"), None),
            ],
        )
        .xml();
    let documents = [("A.xml".to_owned(), document)];
    for index in [serially(&documents), merged(&documents)] {
        let times = |id: &str| -> Vec<(Option<u32>, Option<u32>)> {
            let journey = index.journey_by_id(id).expect("the journey");
            index
                .journey(journey)
                .passing_times
                .iter()
                .map(|time| (time.arrival, time.departure))
                .collect()
        };
        let journey = index.journey_by_id("J").expect("the journey");
        assert_eq!(calls(&index, journey), ["A", "B", "C"]);
        assert_eq!(
            times("J"),
            [
                (None, Some(8 * 3600)),
                (Some(8 * 3600 + 600), Some(8 * 3600 + 660)),
                (Some(8 * 3600 + 1200), None)
            ]
        );
        let short = index.journey_by_id("SHORT").expect("the short run");
        assert_eq!(calls(&index, short), ["A", "B"]);
        let long = index.journey_by_id("LONG").expect("the long run");
        assert_eq!(calls(&index, long), ["A", "B", "C"]);
        assert_eq!(
            times("LONG")[2],
            (Some(10 * 3600 + 1200), Some(10 * 3600 + 1260))
        );
        assert_eq!(
            findings(&index, Concern::PassingTimesOtherThanPoints),
            [
                "passing times naming no point, other in number than the pattern's points: ServiceJourney LONG in A.xml (P)",
                "passing times naming no point, other in number than the pattern's points: ServiceJourney SHORT in A.xml (P)"
            ]
        );
        assert!(index.unresolved().is_empty(), "{:?}", index.unresolved());
    }
}

/// Journeys over one pattern (`tests/fixtures/fragments/frames/journeys-and-blocks.xml`):
/// two with a time at every point, one naming its vehicle type and one taking
/// it from its block, and a third stating nothing, which is left out.
#[test]
fn of_the_journeys_over_a_pattern_the_bare_one_is_left_out() {
    let name = "journeys-and-blocks.xml";
    let index = serially(&[(name.to_owned(), support::fragments::document(name))]);
    let mut kept: Vec<&str> = (0..index.journey_count() as u32)
        .map(|j| index.journey(JourneyId(j)).id)
        .collect();
    kept.sort_unstable();
    assert_eq!(
        kept,
        ["ex:ServiceJourney:block-type", "ex:ServiceJourney:own-type"]
    );
    for id in kept {
        let journey = index.journey_by_id(id).expect("kept");
        assert_eq!(
            calls(&index, journey),
            [
                "ex:ScheduledStopPoint:Rathaus",
                "ex:ScheduledStopPoint:Schule",
                "ex:ScheduledStopPoint:Freibad"
            ]
        );
    }
    assert_eq!(
        findings(&index, Concern::TooFewTimedCalls),
        [format!(
            "journey without two timed calls, left out: ServiceJourney ex:ServiceJourney:nothing in {name}"
        )]
    );
}
