//! The line of a journey that names none, nor its pattern a line: the line of
//! the route it or its pattern names. A route is a document's own, as a line
//! is, and resolves in the document naming it first; the line stated where
//! the journey or the pattern stands comes before the line of a route.

mod support;

use netex_rs::index::{Concern, Index};
use support::timetable::{merged, reversed, serially};

/// A document of routes, lines, two stop points, journey patterns and
/// journeys over them, every day of the week of 12 October 2026.
fn document(routes: &str, lines: &str, patterns: &str, journeys: &str) -> String {
    format!(
        concat!(
            r#"<PublicationDelivery xmlns="http://www.netex.org.uk/netex" version="1.10">"#,
            "<PublicationTimestamp>2026-10-01T00:00:00</PublicationTimestamp>",
            "<ParticipantRef>test</ParticipantRef>",
            r#"<dataObjects><CompositeFrame id="c" version="1"><frames>"#,
            r#"<ServiceFrame id="s" version="1">"#,
            "<routes>{routes}</routes>",
            "<lines>{lines}</lines>",
            "<scheduledStopPoints>",
            r#"<ScheduledStopPoint id="A" version="1"><Name>A</Name></ScheduledStopPoint>"#,
            r#"<ScheduledStopPoint id="B" version="1"><Name>B</Name></ScheduledStopPoint>"#,
            "</scheduledStopPoints>",
            "<journeyPatterns>{patterns}</journeyPatterns>",
            "</ServiceFrame>",
            r#"<ServiceCalendarFrame id="k" version="1">"#,
            r#"<dayTypes><DayType id="DT" version="1"/></dayTypes>"#,
            r#"<operatingPeriods><UicOperatingPeriod id="OP" version="1"><FromDate>2026-10-12T00:00:00</FromDate><ToDate>2026-10-18T00:00:00</ToDate><ValidDayBits>1111111</ValidDayBits></UicOperatingPeriod></operatingPeriods>"#,
            r#"<dayTypeAssignments><DayTypeAssignment id="DTA" version="1" order="1"><UicOperatingPeriodRef ref="OP" version="1"/><DayTypeRef ref="DT" version="1"/></DayTypeAssignment></dayTypeAssignments>"#,
            "</ServiceCalendarFrame>",
            r#"<TimetableFrame id="t" version="1"><vehicleJourneys>{journeys}</vehicleJourneys></TimetableFrame>"#,
            "</frames></CompositeFrame></dataObjects></PublicationDelivery>"
        ),
        routes = routes,
        lines = lines,
        patterns = patterns,
        journeys = journeys,
    )
}

/// A route naming `line`.
fn route(id: &str, line: &str) -> String {
    format!(r#"<Route id="{id}" version="1"><LineRef ref="{line}" version="1"/></Route>"#)
}

fn line(id: &str) -> String {
    format!(
        r#"<Line id="{id}" version="1"><Name>{id}</Name><TransportMode>bus</TransportMode></Line>"#
    )
}

/// A pattern from A to B; `route` is what it states before its points: a
/// `RouteRef`, a `RouteView`, or nothing.
fn pattern(id: &str, route: &str) -> String {
    format!(
        concat!(
            r#"<ServiceJourneyPattern id="{id}" version="1">{route}<pointsInSequence>"#,
            r#"<StopPointInJourneyPattern id="{id}-1" version="1" order="1"><ScheduledStopPointRef ref="A" version="1"/></StopPointInJourneyPattern>"#,
            r#"<StopPointInJourneyPattern id="{id}-2" version="1" order="2"><ScheduledStopPointRef ref="B" version="1"/></StopPointInJourneyPattern>"#,
            "</pointsInSequence></ServiceJourneyPattern>"
        ),
        id = id,
        route = route,
    )
}

/// A journey along `pattern` every day of the calendar; `stated` is what it
/// states between its pattern and its passing times — a `LineRef`, say, as
/// the schema orders them — and `route` its `RouteRef`, before its pattern.
fn journey(id: &str, route: &str, pattern: &str, stated: &str) -> String {
    format!(
        concat!(
            r#"<ServiceJourney id="{id}" version="1">"#,
            r#"<dayTypes><DayTypeRef ref="DT" version="1"/></dayTypes>"#,
            r#"{route}<ServiceJourneyPatternRef ref="{pattern}" version="1"/>{stated}"#,
            "<passingTimes>",
            r#"<TimetabledPassingTime version="1"><StopPointInJourneyPatternRef ref="{pattern}-1" version="1"/><DepartureTime>08:00:00</DepartureTime></TimetabledPassingTime>"#,
            r#"<TimetabledPassingTime version="1"><StopPointInJourneyPatternRef ref="{pattern}-2" version="1"/><ArrivalTime>08:10:00</ArrivalTime></TimetabledPassingTime>"#,
            "</passingTimes></ServiceJourney>"
        ),
        id = id,
        route = route,
        pattern = pattern,
        stated = stated,
    )
}

fn route_ref(route: &str) -> String {
    format!(r#"<RouteRef ref="{route}" version="1"/>"#)
}

fn line_ref(line: &str) -> String {
    format!(r#"<LineRef ref="{line}" version="1"/>"#)
}

/// The indexes of `documents` the four ways a delivery is read — the full
/// model and the lean stream, in the order given and reversed —, each with
/// the line every journey runs in.
fn every_way(documents: &[(String, String)]) -> Vec<Index> {
    vec![
        serially(documents),
        serially(&reversed(documents)),
        merged(documents),
        merged(&reversed(documents)),
    ]
}

/// The line `journey` runs in, by identifier.
fn line_of<'a>(index: &'a Index, journey: &str) -> Option<&'a str> {
    let journey = index.journey_by_id(journey).expect("the journey");
    index.journey(journey).line.map(|line| index.line(line).id)
}

fn one(xml: String) -> Vec<(String, String)> {
    vec![("LINE.xml".to_owned(), xml)]
}

#[test]
fn a_pattern_naming_only_its_route_runs_in_the_line_the_route_names() {
    let documents = one(document(
        &route("R", "L"),
        &line("L"),
        &pattern("P", &route_ref("R")),
        &journey("J", "", "P", ""),
    ));
    for index in every_way(&documents) {
        assert_eq!(line_of(&index, "J"), Some("L"));
        let line = index.line_by_id("L").expect("the line");
        assert_eq!(index.journeys_of_line(line).len(), 1);
        assert!(index.unresolved().is_empty(), "{:?}", index.unresolved());
    }
}

#[test]
fn a_route_view_naming_only_its_route_gives_the_line_the_route_names() {
    let documents = one(document(
        &route("R", "L"),
        &line("L"),
        &pattern("P", &format!("<RouteView>{}</RouteView>", route_ref("R"))),
        &journey("J", "", "P", ""),
    ));
    for index in every_way(&documents) {
        assert_eq!(line_of(&index, "J"), Some("L"));
    }
}

#[test]
fn a_journey_naming_its_route_runs_in_the_line_the_route_names() {
    let documents = one(document(
        &route("R", "L"),
        &line("L"),
        &pattern("P", ""),
        &journey("J", &route_ref("R"), "P", ""),
    ));
    for index in every_way(&documents) {
        assert_eq!(line_of(&index, "J"), Some("L"));
    }
}

/// The journey's own line comes before the line of its route, its route's
/// before its pattern's, and a pattern's line view before the pattern's route.
#[test]
fn the_line_stated_nearest_the_journey_comes_first() {
    let documents = one(document(
        &[route("R-1", "L-1"), route("R-2", "L-2")].concat(),
        &[line("L-1"), line("L-2"), line("L-3")].concat(),
        &[
            pattern("P-route", &route_ref("R-1")),
            pattern(
                "P-both",
                &format!(
                    "<RouteView>{}{}</RouteView>",
                    route_ref("R-1"),
                    line_ref("L-3")
                ),
            ),
        ]
        .concat(),
        &[
            journey("J-own-line", &route_ref("R-2"), "P-route", &line_ref("L-3")),
            journey("J-own-route", &route_ref("R-2"), "P-route", ""),
            journey("J-pattern-view", "", "P-both", ""),
        ]
        .concat(),
    ));
    for index in every_way(&documents) {
        assert_eq!(line_of(&index, "J-own-line"), Some("L-3"));
        assert_eq!(line_of(&index, "J-own-route"), Some("L-2"));
        assert_eq!(line_of(&index, "J-pattern-view"), Some("L-3"));
    }
}

/// Both documents define route `R` with a line of their own: each pattern
/// names its own document's route, and the other definition is a finding.
#[test]
fn a_route_resolves_in_the_document_naming_it_first() {
    let documents = vec![
        (
            "A.xml".to_owned(),
            document(
                &route("R", "L-a"),
                &line("L-a"),
                &pattern("P-a", &route_ref("R")),
                &journey("J-a", "", "P-a", ""),
            ),
        ),
        (
            "B.xml".to_owned(),
            document(
                &route("R", "L-b"),
                &line("L-b"),
                &pattern("P-b", &route_ref("R")),
                &journey("J-b", "", "P-b", ""),
            ),
        ),
    ];
    for index in every_way(&documents) {
        assert_eq!(line_of(&index, "J-a"), Some("L-a"));
        assert_eq!(line_of(&index, "J-b"), Some("L-b"));
        let redefined: Vec<String> = index
            .findings()
            .filter(|finding| finding.concern == Concern::RedefinedOtherwise)
            .map(|finding| finding.to_string())
            .collect();
        assert_eq!(
            redefined,
            ["redefined with other content: Route R in B.xml, kept as A.xml defines it"]
        );
    }
}

/// The Swiss layout: routes and lines in a shared document, the patterns and
/// journeys in a document naming them.
#[test]
fn a_route_another_document_defines_resolves_in_the_delivery() {
    let documents = vec![
        (
            "SHARED.xml".to_owned(),
            document(&route("R", "L"), &line("L"), "", ""),
        ),
        (
            "TIMETABLE.xml".to_owned(),
            document(
                "",
                "",
                &pattern("P", &route_ref("R")),
                &journey("J", "", "P", ""),
            ),
        ),
    ];
    for index in every_way(&documents) {
        assert_eq!(line_of(&index, "J"), Some("L"));
        assert!(index.unresolved().is_empty(), "{:?}", index.unresolved());
    }
}

#[test]
fn a_route_naming_a_line_no_document_defines_is_unresolved() {
    let documents = one(document(
        &route("R", "L-missing"),
        &line("L"),
        &pattern("P", &route_ref("R")),
        &journey("J", "", "P", ""),
    ));
    for index in every_way(&documents) {
        assert_eq!(line_of(&index, "J"), None);
        let unresolved: Vec<String> = index.unresolved().iter().map(|u| u.to_string()).collect();
        assert_eq!(
            unresolved,
            ["LINE.xml: <LineRef> in Route R refers to L-missing version 1, which no document defines"]
        );
    }
}
