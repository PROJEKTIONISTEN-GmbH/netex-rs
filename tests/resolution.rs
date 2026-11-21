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

/// A stop place X in versions `2.0`, `10` and `9`, stated in this order in
/// one document: the version a reference naming none resolves to.
fn highest_of_three_versions(order: [&str; 3]) -> String {
    let places: String = order
        .iter()
        .map(|version| {
            format!(r#"<StopPlace id="X" version="{version}"><Name>X {version}</Name></StopPlace>"#)
        })
        .collect();
    let frames =
        format!(r#"<SiteFrame id="g" version="1"><stopPlaces>{places}</stopPlaces></SiteFrame>"#);
    let index = serially(&[("a.xml".to_owned(), publication(&frames))]);
    let place = index.stop_by_id("X").expect("the stop place is indexed");
    index
        .stop_version(place)
        .expect("the stop place states a version")
        .to_owned()
}

#[test]
fn the_highest_of_versions_written_in_digits_and_otherwise_does_not_depend_on_their_order() {
    // Digits alone by their number, below versions written otherwise:
    // 9 < 10 < 2.0, in whatever order the document states them.
    let orders = [
        ["2.0", "10", "9"],
        ["2.0", "9", "10"],
        ["10", "2.0", "9"],
        ["10", "9", "2.0"],
        ["9", "2.0", "10"],
        ["9", "10", "2.0"],
    ];
    for order in orders {
        assert_eq!(highest_of_three_versions(order), "2.0", "{order:?}");
    }
}

#[test]
fn a_name_stated_as_text_beside_its_translations_is_read_and_written_so() {
    // NeTEx 2.0's `MultilingualString` is mixed content: a text in the
    // default language may stand beside the translations.
    let xml = r#"<StopPlace xmlns="http://www.netex.org.uk/netex" id="X" version="1"><Name>Hannover<Text lang="en">Hanover</Text></Name></StopPlace>"#;
    let limits = netex_rs::Limits::default();
    let place: netex_rs::model::StopPlace =
        netex_rs::read_element(xml, "StopPlace", &limits).expect("the stop place reads");
    let name = place.name.as_ref().expect("a name");
    assert_eq!(name.as_str(), "Hannover");
    assert_eq!(name.default_text(), "Hannover");
    assert_eq!(name.translations.len(), 1);
    assert_eq!(name.translations[0].as_str(), "Hanover");
    let written = netex_rs::write_element(&place, "StopPlace");
    let again: netex_rs::model::StopPlace =
        netex_rs::read_element(&written, "StopPlace", &limits).expect("what is written reads");
    assert_eq!(again, place, "{written}");

    let frames = format!(
        r#"<SiteFrame id="g" version="1"><stopPlaces>{}</stopPlaces></SiteFrame>"#,
        xml.replace(r#" xmlns="http://www.netex.org.uk/netex""#, "")
    );
    let index = serially(&[("a.xml".to_owned(), publication(&frames))]);
    let stop = index.stop_by_id("X").expect("the stop place is indexed");
    assert_eq!(index.stop(stop).name, "Hannover");
}

#[test]
fn a_level_named_but_not_stated_is_a_reference_to_nothing() {
    let frames = concat!(
        r#"<SiteFrame id="g" version="1"><stopPlaces>"#,
        r#"<StopPlace id="SPL" version="1"><Name>Platz</Name>"#,
        r#"<levels><Level id="L0" version="1"><Name>Erdgeschoss</Name></Level></levels>"#,
        r#"<quays><Quay id="Q1" version="1"><Name>1</Name><LevelRef ref="L-missing" version="1"/></Quay>"#,
        r#"<Quay id="Q2" version="1"><Name>2</Name><LevelRef ref="L0" version="any"/></Quay></quays>"#,
        "</StopPlace></stopPlaces></SiteFrame>"
    );
    let index = serially(&[("a.xml".to_owned(), publication(frames))]);
    let unresolved: Vec<String> = index.unresolved().iter().map(ToString::to_string).collect();
    assert_eq!(
        unresolved,
        ["a.xml: <LevelRef> in Quay Q1 refers to L-missing, version 1"]
    );
    let level = |quay: &str| index.stop(index.stop_by_id(quay).expect("the quay")).level;
    assert_eq!(level("Q1"), None);
    assert_eq!(level("Q2"), Some("Erdgeschoss"));
}

/// A stop place X of the DHID de:01:1, named `name`.
fn place_of_the_dhid(name: &str) -> String {
    publication(&format!(
        concat!(
            r#"<SiteFrame id="g" version="1"><stopPlaces><StopPlace id="X" version="1">"#,
            "<keyList><KeyValue><Key>GlobalID</Key><Value>de:01:1</Value></KeyValue></keyList>",
            "<Name>{}</Name></StopPlace></stopPlaces></SiteFrame>"
        ),
        name
    ))
}

#[test]
fn a_dhid_two_documents_name_otherwise_finds_the_stop_its_identifier_does_whatever_their_order() {
    let documents = [
        ("a.xml".to_owned(), place_of_the_dhid("Alpha")),
        ("b.xml".to_owned(), place_of_the_dhid("Beta")),
    ];
    for order in [
        documents.to_vec(),
        documents.iter().rev().cloned().collect(),
    ] {
        let index = serially(&order);
        let by_dhid = index.stop_by_dhid("de:01:1").expect("the DHID is indexed");
        assert_eq!(Some(by_dhid), index.stop_by_id("X"));
        assert_eq!(index.stop(by_dhid).name, "Alpha");
    }
}

/// A document of stop places X and Y and a walk between them of two
/// minutes, both ways or not.
fn walk_document(id: &str, both_ways: bool) -> String {
    publication(&format!(
        concat!(
            r#"<SiteFrame id="g" version="1"><stopPlaces>"#,
            r#"<StopPlace id="X" version="1"><Name>X</Name></StopPlace><StopPlace id="Y" version="1"><Name>Y</Name></StopPlace>"#,
            "</stopPlaces></SiteFrame>",
            r#"<ServiceFrame id="s" version="1"><connections><SiteConnection id="{id}" version="1">"#,
            "<WalkTransferDuration><DefaultDuration>PT2M</DefaultDuration></WalkTransferDuration>",
            r#"<From><StopPlaceRef ref="X" version="1"/></From><To><StopPlaceRef ref="Y" version="1"/></To>"#,
            "<BothWays>{both_ways}</BothWays></SiteConnection></connections></ServiceFrame>"
        ),
        id = id,
        both_ways = both_ways,
    ))
}

#[test]
fn walks_alike_but_for_their_direction_are_listed_in_one_order_whatever_order_they_come_in() {
    let documents = [
        ("a.xml".to_owned(), walk_document("C1", false)),
        ("b.xml".to_owned(), walk_document("C2", true)),
    ];
    let walks = |documents: &[(String, String)]| {
        let index = serially(documents);
        let x = index.stop_by_id("X").expect("X");
        index.transfers_from(x).to_vec()
    };
    let in_order = walks(&documents);
    let reversed: Vec<(String, String)> = documents.iter().rev().cloned().collect();
    assert_eq!(in_order.len(), 2, "{in_order:?}");
    assert_eq!(walks(&reversed), in_order);
}
