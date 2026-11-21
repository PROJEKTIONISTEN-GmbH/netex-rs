//! What a reference resolves to where the delivery offers more than one
//! answer, or a nearer answer fails: the version valid on the day the index is
//! built for, whatever order the documents come in; the stop place a walk or
//! an assignment names beside a quay that is not there; a name stated only as
//! texts; a day offset beyond any timetable.

mod support;

use netex_rs::index::{Index, IndexBuilder};
use netex_rs::model::Date;
use support::fragments::publication;
use support::timetable::serially;

/// A scheduled stop point of this version, valid in the period given.
fn stop_point(id: &str, version: &str, valid: Option<(&str, &str)>) -> String {
    let valid = valid.map_or_else(String::new, |(from, to)| {
        format!("<ValidBetween><FromDate>{from}T00:00:00</FromDate><ToDate>{to}T23:59:59</ToDate></ValidBetween>")
    });
    format!(
        r#"<ScheduledStopPoint id="{id}" version="{version}">{valid}<Name>{id} {version}</Name></ScheduledStopPoint>"#
    )
}

/// A line L over the stop points given and a journey J calling at SP-A,
/// naming it in `version`, and at SP-B, every day of October 2026; `offset`
/// is what the departure at SP-A states beside its time.
fn journey_document(stop_points: &str, version: &str, offset: &str) -> String {
    publication(&format!(
        concat!(
            r#"<ServiceFrame id="s" version="1">"#,
            r#"<lines><Line id="L" version="1"><Name>L</Name><TransportMode>bus</TransportMode></Line></lines>"#,
            "<scheduledStopPoints>{stop_points}</scheduledStopPoints>",
            r#"<journeyPatterns><ServiceJourneyPattern id="P" version="1"><RouteView><LineRef ref="L" version="1"/></RouteView><pointsInSequence>"#,
            r#"<StopPointInJourneyPattern id="P1" version="1" order="1"><ScheduledStopPointRef ref="SP-A" version="{version}"/></StopPointInJourneyPattern>"#,
            r#"<StopPointInJourneyPattern id="P2" version="1" order="2"><ScheduledStopPointRef ref="SP-B" version="1"/></StopPointInJourneyPattern>"#,
            "</pointsInSequence></ServiceJourneyPattern></journeyPatterns></ServiceFrame>",
            r#"<ServiceCalendarFrame id="k" version="1">"#,
            r#"<dayTypes><DayType id="DT" version="1"/></dayTypes>"#,
            r#"<operatingPeriods><OperatingPeriod id="OP" version="1"><FromDate>2026-10-01T00:00:00</FromDate><ToDate>2026-10-31T00:00:00</ToDate></OperatingPeriod></operatingPeriods>"#,
            r#"<dayTypeAssignments><DayTypeAssignment id="DTA" version="1" order="1"><OperatingPeriodRef ref="OP" version="1"/><DayTypeRef ref="DT" version="1"/></DayTypeAssignment></dayTypeAssignments>"#,
            "</ServiceCalendarFrame>",
            r#"<TimetableFrame id="t" version="1"><vehicleJourneys>"#,
            r#"<ServiceJourney id="J" version="1"><dayTypes><DayTypeRef ref="DT" version="1"/></dayTypes>"#,
            r#"<ServiceJourneyPatternRef ref="P" version="1"/><LineRef ref="L" version="1"/><passingTimes>"#,
            r#"<TimetabledPassingTime version="1"><StopPointInJourneyPatternRef ref="P1" version="1"/><DepartureTime>08:00:00</DepartureTime>{offset}</TimetabledPassingTime>"#,
            r#"<TimetabledPassingTime version="1"><StopPointInJourneyPatternRef ref="P2" version="1"/><ArrivalTime>08:30:00</ArrivalTime></TimetabledPassingTime>"#,
            "</passingTimes></ServiceJourney></vehicleJourneys></TimetableFrame>"
        ),
        stop_points = stop_points,
        version = version,
        offset = offset,
    ))
}

/// The version of the stop point J first calls at, in the index of
/// `documents` built for 15 October 2026 in the order given.
fn version_called_at_on_15_october(documents: &[(&str, &str)]) -> String {
    let mut builder = IndexBuilder::new().at(&Date::from("2026-10-15"));
    for (name, xml) in documents {
        builder.add(&netex_rs::read(xml).expect("the document reads"), name);
    }
    let index = builder.finish();
    let journey = index.journey_by_id("J").expect("the journey is indexed");
    let first = index.journey(journey).stop_points[0];
    index
        .stop_point_version(first)
        .expect("the stop point states a version")
        .to_owned()
}

#[test]
fn of_two_versions_valid_on_the_day_the_higher_answers_whatever_order_the_documents_come_in() {
    // Version 1 of SP-A is valid all year, version 2 from June: on 15 October
    // both are, and a reference naming any version takes the higher.
    let first = publication(&format!(
        r#"<ServiceFrame id="s1" version="1"><scheduledStopPoints>{}</scheduledStopPoints></ServiceFrame>"#,
        stop_point("SP-A", "1", Some(("2026-01-01", "2026-12-31")))
    ));
    let second = journey_document(
        &format!(
            "{}{}",
            stop_point("SP-A", "2", Some(("2026-06-01", "2026-12-31"))),
            stop_point("SP-B", "1", None)
        ),
        "any",
        "",
    );
    let in_order = version_called_at_on_15_october(&[("a.xml", &first), ("b.xml", &second)]);
    let reversed = version_called_at_on_15_october(&[("b.xml", &second), ("a.xml", &first)]);
    assert_eq!(
        in_order, "2",
        "of the versions valid on the day, the higher"
    );
    assert_eq!(
        reversed, "2",
        "of the versions valid on the day, the higher"
    );
}

/// A stop place SPL whose quay a walk and an assignment name, but which
/// states no such quay, and a stop place SPL2 the walk leads to.
fn quay_named_but_not_stated() -> Index {
    let frames = concat!(
        r#"<SiteFrame id="g" version="1"><stopPlaces>"#,
        r#"<StopPlace id="SPL" version="1"><Name>Platz</Name>"#,
        r#"<quays><Quay id="Q1" version="1"><Name>1</Name></Quay></quays></StopPlace>"#,
        r#"<StopPlace id="SPL2" version="1"><Name>Anderer Platz</Name></StopPlace>"#,
        "</stopPlaces></SiteFrame>",
        r#"<ServiceFrame id="s" version="1">"#,
        r#"<scheduledStopPoints><ScheduledStopPoint id="SSP" version="1"><Name>SSP</Name></ScheduledStopPoint></scheduledStopPoints>"#,
        r#"<connections><SiteConnection id="C" version="1">"#,
        "<WalkTransferDuration><DefaultDuration>PT2M</DefaultDuration></WalkTransferDuration>",
        r#"<From><StopPlaceRef ref="SPL" version="1"/><QuayRef ref="Q-missing" version="1"/></From>"#,
        r#"<To><StopPlaceRef ref="SPL2" version="1"/></To></SiteConnection></connections>"#,
        r#"<stopAssignments><PassengerStopAssignment id="A" version="1" order="1">"#,
        r#"<ScheduledStopPointRef ref="SSP" version="1"/>"#,
        r#"<StopPlaceRef ref="SPL" version="1"/><QuayRef ref="Q-missing" version="1"/>"#,
        "</PassengerStopAssignment></stopAssignments></ServiceFrame>"
    );
    serially(&[("a.xml".to_owned(), publication(frames))])
}

#[test]
fn a_walk_whose_quay_is_not_stated_starts_at_the_stop_place_it_names_beside_it() {
    let index = quay_named_but_not_stated();
    let place = index.stop_by_id("SPL").expect("the stop place is indexed");
    let other = index
        .stop_by_id("SPL2")
        .expect("the other stop place is indexed");
    assert!(
        index
            .transfers_from(place)
            .iter()
            .any(|walk| walk.to == other),
        "the walk from SPL to SPL2 is kept: {:?}",
        index.transfers_from(place)
    );
}

#[test]
fn an_assignment_whose_quay_is_not_stated_assigns_the_stop_place_it_names_beside_it() {
    let index = quay_named_but_not_stated();
    let point = index
        .stop_point_by_id("SSP")
        .expect("the stop point is indexed");
    assert_eq!(
        index.stop_of_stop_point(point),
        index.stop_by_id("SPL"),
        "the stop point keeps the stop place its assignment names"
    );
}

#[test]
fn a_name_stated_as_texts_is_the_stop_point_s_name() {
    // NeTEx 2.0 states a multilingual name as `Text` elements only.
    let frames = concat!(
        r#"<ServiceFrame id="s" version="1"><scheduledStopPoints>"#,
        r#"<ScheduledStopPoint id="SSP" version="1">"#,
        r#"<Name><Text>Haltestelle B</Text><Text lang="en">Stop B</Text></Name>"#,
        "</ScheduledStopPoint></scheduledStopPoints></ServiceFrame>"
    );
    let index = serially(&[("a.xml".to_owned(), publication(frames))]);
    let point = index
        .stop_point_by_id("SSP")
        .expect("the stop point is indexed");
    assert_eq!(index.stop_point(point).name, "Haltestelle B");
}

#[test]
fn a_day_offset_beyond_any_timetable_is_no_panic() {
    // The first offset whose number of seconds no i64 holds.
    let document = journey_document(
        &format!(
            "{}{}",
            stop_point("SP-A", "1", None),
            stop_point("SP-B", "1", None)
        ),
        "1",
        "<DepartureDayOffset>106751991167301</DepartureDayOffset>",
    );
    let built = std::panic::catch_unwind(|| serially(&[("a.xml".to_owned(), document)]));
    assert!(built.is_ok(), "building the index panicked on a day offset");
}

#[test]
fn a_name_stated_as_texts_is_the_text_stating_no_language_before_the_first() {
    // A `Text` stating no language is in the frame's default language.
    let frames = concat!(
        r#"<ServiceFrame id="s" version="1"><scheduledStopPoints>"#,
        r#"<ScheduledStopPoint id="SSP" version="1">"#,
        r#"<Name><Text lang="en">Stop B</Text><Text>Haltestelle B</Text></Name>"#,
        "</ScheduledStopPoint></scheduledStopPoints></ServiceFrame>"
    );
    let index = serially(&[("a.xml".to_owned(), publication(frames))]);
    let point = index
        .stop_point_by_id("SSP")
        .expect("the stop point is indexed");
    assert_eq!(index.stop_point(point).name, "Haltestelle B");
}

#[test]
fn a_quay_named_but_not_stated_stays_a_reference_to_nothing() {
    let index = quay_named_but_not_stated();
    let unresolved: Vec<(&str, &str)> = index
        .unresolved()
        .iter()
        .map(|reference| (reference.referrer.as_str(), reference.id.as_str()))
        .collect();
    assert_eq!(
        unresolved,
        [
            ("PassengerStopAssignment A", "Q-missing"),
            ("SiteConnection C", "Q-missing")
        ]
    );
}

/// The findings of a time outside the day of the index, by journey.
fn times_out_of_range(index: &Index) -> Vec<String> {
    index
        .findings()
        .filter(|finding| finding.concern.code() == "time-out-of-range")
        .map(|finding| finding.entity.to_owned())
        .collect()
}

#[test]
fn a_day_offset_beyond_any_timetable_leaves_the_call_untimed_and_says_so() {
    let document = journey_document(
        &format!(
            "{}{}",
            stop_point("SP-A", "1", None),
            stop_point("SP-B", "1", None)
        ),
        "1",
        "<DepartureDayOffset>106751991167301</DepartureDayOffset>",
    );
    let index = serially(&[("a.xml".to_owned(), document)]);
    assert_eq!(times_out_of_range(&index), ["J"]);
    assert_eq!(
        index.journey_by_id("J"),
        None,
        "one timed call is no journey"
    );
}

#[test]
fn a_departure_before_the_operating_day_begins_is_a_time_the_index_does_not_hold() {
    // A day offset of -1 departs on the day before the operating day: no
    // second since its midnight, which is what the index counts from.
    let document = journey_document(
        &format!(
            "{}{}",
            stop_point("SP-A", "1", None),
            stop_point("SP-B", "1", None)
        ),
        "1",
        "<DepartureDayOffset>-1</DepartureDayOffset>",
    );
    let index = serially(&[("a.xml".to_owned(), document)]);
    assert_eq!(times_out_of_range(&index), ["J"]);
}

#[test]
fn a_day_offset_within_the_index_s_reach_counts_whole_days() {
    let document = journey_document(
        &format!(
            "{}{}",
            stop_point("SP-A", "1", None),
            stop_point("SP-B", "1", None)
        ),
        "1",
        "<DepartureDayOffset>2</DepartureDayOffset>",
    );
    let index = serially(&[("a.xml".to_owned(), document)]);
    assert!(times_out_of_range(&index).is_empty());
    let journey = index.journey_by_id("J").expect("the journey is indexed");
    assert_eq!(
        index.journey(journey).passing_times[0].departure,
        Some(2 * 86_400 + 8 * 3600)
    );
}
