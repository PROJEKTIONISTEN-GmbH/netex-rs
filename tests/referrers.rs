//! Who refers to what: the references of a delivery read backwards, built only
//! when asked for, each resolved as the index resolves it — in its own
//! document first —, and the definitions nothing refers to.

mod support;

use std::collections::BTreeSet;

use netex_rs::dataset::Source;
use netex_rs::index::{Index, IndexBuilder, ObjectId, Referrers};
use netex_rs::stream::{ReadOptions, Selection};
use support::timetable::{reversed, two_calendars};

/// The index of documents given by name and text, lean, one builder per
/// document, merged in the order given; with the references kept.
fn with_referrers(documents: &[(String, String)]) -> Index {
    let options = ReadOptions::new(Selection::routing());
    let mut merged = IndexBuilder::new().with_referrers();
    for (name, xml) in documents {
        let mut builder = IndexBuilder::new();
        builder
            .add_streamed(xml, &options, name)
            .expect("the document reads");
        merged.merge(builder);
    }
    merged.finish()
}

fn own(profile: &str) -> Vec<(String, String)> {
    support::own_fixtures(profile)
        .into_iter()
        .map(|fixture| (fixture.name, fixture.xml))
        .collect()
}

/// An object as text: `element id in document`, or `element id` for what is
/// gathered once.
fn describe(referrers: &Referrers<'_>, object: ObjectId) -> String {
    let object = referrers.object(object);
    match object.document {
        Some(document) => format!("{} {} in {document}", object.element, object.id),
        None => format!("{} {}", object.element, object.id),
    }
}

/// What refers to the one object of this element and identifier, as text:
/// `via from referrer`.
fn referrers_of(
    index: &Index,
    element: &str,
    id: &str,
    document: Option<&str>,
) -> BTreeSet<String> {
    let referrers = index.referrers().expect("the references were kept");
    let objects: Vec<ObjectId> = referrers.find(element, id, document).collect();
    assert_eq!(objects.len(), 1, "one {element} {id}: {objects:?}");
    referrers
        .of(objects[0])
        .map(|reference| {
            format!(
                "{} from {}",
                reference.via,
                describe(&referrers, reference.object)
            )
        })
        .collect()
}

fn set(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|item| (*item).to_owned()).collect()
}

#[test]
fn an_index_keeps_no_references_unless_asked_to() {
    let documents = own("ch");
    let mut builder = IndexBuilder::new();
    for (name, xml) in &documents {
        builder
            .add_streamed(xml, &ReadOptions::new(Selection::routing()), name)
            .expect("the document reads");
    }
    assert!(builder.finish().referrers().is_none());
    assert!(with_referrers(&documents).referrers().is_some());
}

#[test]
fn what_refers_to_an_object_is_found_across_the_delivery() {
    // The Swiss layout: the timetable document names the stop points, lines
    // and availability conditions of the shared document.
    let index = with_referrers(&own("ch"));
    assert_eq!(
        referrers_of(&index, "ScheduledStopPoint", "ch:ex:ScheduledStopPoint:B", None),
        set(&[
            "ScheduledStopPointRef from ServiceJourney ch:ex:ServiceJourney:IR9-2148 in ch-timetable.xml",
            "ScheduledStopPointRef from ServiceJourney ch:ex:ServiceJourney:S1 in ch-timetable.xml",
        ])
    );
    assert_eq!(
        referrers_of(&index, "Line", "ch:ex:Line:IR9", Some("ch-service.xml")),
        set(&["LineRef from ServiceJourney ch:ex:ServiceJourney:IR9-2148 in ch-timetable.xml"])
    );
    assert_eq!(
        referrers_of(
            &index,
            "AvailabilityCondition",
            "ch:ex:AvailabilityCondition:weekdays",
            Some("ch-service.xml")
        ),
        set(&["AvailabilityConditionRef from ServiceJourney ch:ex:ServiceJourney:S1 in ch-timetable.xml"])
    );
    // The passed-through point is no call, but its stop point is referred to all the same.
    assert_eq!(
        referrers_of(&index, "ScheduledStopPoint", "ch:ex:ScheduledStopPoint:C", None),
        set(&[
            "ScheduledStopPointRef from ServiceJourney ch:ex:ServiceJourney:IR9-2148 in ch-timetable.xml",
            "ScheduledStopPointRef from ServiceJourney ch:ex:ServiceJourney:S1 in ch-timetable.xml",
        ])
    );
}

#[test]
fn a_reference_is_read_back_where_it_resolved_in_its_own_document() {
    // Both documents define `DT` with other days; each journey names its own
    // document's, so each day type is referred to from its own document only.
    let documents = two_calendars();
    let first = &documents[0].0;
    let second = &documents[1].0;
    for index in [
        with_referrers(&documents),
        with_referrers(&reversed(&documents)),
    ] {
        assert_eq!(
            referrers_of(&index, "DayType", "DT", Some(first)),
            set(&[
                &format!("DayTypeRef from ServiceJourney J-e25 in {first}"),
                &format!("DayTypeRef from DayTypeAssignment DTA in {first}"),
            ])
        );
        assert_eq!(
            referrers_of(&index, "DayType", "DT", Some(second)),
            set(&[
                &format!("DayTypeRef from ServiceJourney J-f26 in {second}"),
                &format!("DayTypeRef from DayTypeAssignment DTA in {second}"),
            ])
        );
        assert_eq!(
            referrers_of(&index, "UicOperatingPeriod", "OP", Some(first)),
            set(&[&format!(
                "OperatingPeriodRef from DayTypeAssignment DTA in {first}"
            )])
        );
    }
}

#[test]
fn the_references_of_an_object_lead_to_what_it_refers_to() {
    // From a stop assignment to the stop point it places, from there to the
    // pattern calling at it and on to the journey following the pattern: the
    // way a checker finds the journeys a defect in an assignment touches.
    let index = with_referrers(&own("at"));
    let referrers = index.referrers().expect("the references were kept");
    let document = "NX-PI_01_at_ex_LINE_ex_100_20261001.xml";
    let assignment = referrers
        .find(
            "PassengerStopAssignment",
            "at:ex:PassengerStopAssignment:1",
            Some(document),
        )
        .next()
        .expect("the assignment refers, so the graph holds it");
    let placed: BTreeSet<String> = referrers
        .from(assignment)
        .map(|reference| {
            format!(
                "{} to {}",
                reference.via,
                describe(&referrers, reference.object)
            )
        })
        .collect();
    assert_eq!(
        placed,
        set(&[
            "ScheduledStopPointRef to ScheduledStopPoint at:ex:ScheduledStopPoint:1",
            "QuayRef to Quay at:ex:Quay:1:1",
        ])
    );
    let stop_point = referrers
        .find("ScheduledStopPoint", "at:ex:ScheduledStopPoint:1", None)
        .next()
        .expect("the stop point");
    let patterns: Vec<ObjectId> = referrers
        .of(stop_point)
        .filter(|reference| referrers.object(reference.object).element == "ServiceJourneyPattern")
        .map(|reference| reference.object)
        .collect();
    assert_eq!(patterns.len(), 1);
    let journeys: BTreeSet<String> = referrers
        .of(patterns[0])
        .map(|reference| describe(&referrers, reference.object))
        .collect();
    assert_eq!(
        journeys,
        set(&[
            &format!("ServiceJourney at:ex:ServiceJourney:100-v1 in {document}"),
            &format!("ServiceJourney at:ex:ServiceJourney:100-v2 in {document}"),
        ])
    );
}

/// The Italian line offer's train names only its route, which names the
/// line: the pattern refers to the route, the route to the line.
#[test]
fn a_route_stands_between_the_pattern_naming_it_and_its_line() {
    let index = with_referrers(&own("it"));
    let document = "it-ith1-line-offer.xml";
    assert_eq!(
        referrers_of(&index, "Route", "IT:ITH1:Route:R1-H:", Some(document)),
        set(&[&format!(
            "RouteRef from ServiceJourneyPattern IT:ITH1:ServiceJourneyPattern:R1-H: in {document}"
        )])
    );
    assert_eq!(
        referrers_of(&index, "Line", "IT:ITH1:Line:R1:", Some(document)),
        set(&[&format!(
            "LineRef from Route IT:ITH1:Route:R1-H: in {document}"
        )])
    );
}

/// A route no pattern and no journey names is among what nothing refers to.
#[test]
fn a_route_nothing_names_is_listed_as_unreferenced() {
    let xml = support::own_fixtures("it").remove(0).xml.replace(
        r#"<RouteRef ref="IT:ITH1:Route:R1-H:" version="1"/>"#,
        r#"<RouteView><LineRef ref="IT:ITH1:Line:R1:" version="1"/></RouteView>"#,
    );
    let index = with_referrers(&[("line.xml".to_owned(), xml)]);
    let referrers = index.referrers().expect("the references were kept");
    let routes: Vec<String> = referrers
        .unreferenced()
        .filter(|&object| referrers.object(object).element == "Route")
        .map(|object| describe(&referrers, object))
        .collect();
    assert_eq!(routes, ["Route IT:ITH1:Route:R1-H: in line.xml"]);
}

#[test]
fn the_objects_of_a_journey_and_a_stop_are_those_the_index_holds() {
    let index = with_referrers(&own("ch"));
    let referrers = index.referrers().expect("the references were kept");
    let journey = index
        .journey_by_id("ch:ex:ServiceJourney:IR9-2148")
        .expect("the journey");
    let objects: Vec<String> = referrers
        .journey(journey)
        .map(|object| describe(&referrers, object))
        .collect();
    assert_eq!(
        objects,
        ["ServiceJourney ch:ex:ServiceJourney:IR9-2148 in ch-timetable.xml"]
    );
    let line = index.line_by_id("ch:ex:Line:S1").expect("the line");
    let lines: Vec<String> = referrers
        .line(line)
        .map(|object| describe(&referrers, object))
        .collect();
    assert_eq!(lines, ["Line ch:ex:Line:S1 in ch-service.xml"]);
    let point = index
        .stop_point_by_id("ch:ex:ScheduledStopPoint:D")
        .expect("the stop point");
    assert_eq!(
        describe(&referrers, referrers.stop_point(point)),
        "ScheduledStopPoint ch:ex:ScheduledStopPoint:D"
    );
}

#[test]
fn what_nothing_refers_to_is_listed_but_a_stop_place_whose_quay_is_used() {
    // A stop place nothing places a stop point at, a stop point no pattern
    // calls at, a day type no journey and no assignment names; and a stop
    // place only whose quay an assignment names, which is in use.
    let xml = r#"<PublicationDelivery xmlns="http://www.netex.org.uk/netex" version="ntx:1.1">
      <PublicationTimestamp>2026-10-14T00:00:00</PublicationTimestamp>
      <ParticipantRef>test</ParticipantRef>
      <dataObjects><CompositeFrame id="c" version="1"><frames>
        <SiteFrame id="g" version="1"><stopPlaces>
          <StopPlace id="SP-used" version="1"><Name>Used</Name>
            <quays><Quay id="Q-used" version="1"><Name>Used 1</Name></Quay></quays></StopPlace>
          <StopPlace id="SP-idle" version="1"><Name>Idle</Name></StopPlace>
        </stopPlaces></SiteFrame>
        <ServiceFrame id="s" version="1">
          <lines><Line id="L" version="1"><Name>L</Name></Line></lines>
          <scheduledStopPoints>
            <ScheduledStopPoint id="A" version="1"><Name>A</Name></ScheduledStopPoint>
            <ScheduledStopPoint id="B" version="1"><Name>B</Name></ScheduledStopPoint>
            <ScheduledStopPoint id="Z" version="1"><Name>Z</Name></ScheduledStopPoint>
          </scheduledStopPoints>
          <stopAssignments>
            <PassengerStopAssignment id="PSA" version="1" order="1">
              <ScheduledStopPointRef ref="A" version="1"/><QuayRef ref="Q-used" version="1"/>
            </PassengerStopAssignment>
          </stopAssignments>
          <journeyPatterns><ServiceJourneyPattern id="P" version="1">
            <RouteView><LineRef ref="L" version="1"/></RouteView>
            <pointsInSequence>
              <StopPointInJourneyPattern id="P1" version="1" order="1"><ScheduledStopPointRef ref="A" version="1"/></StopPointInJourneyPattern>
              <StopPointInJourneyPattern id="P2" version="1" order="2"><ScheduledStopPointRef ref="B" version="1"/></StopPointInJourneyPattern>
            </pointsInSequence>
          </ServiceJourneyPattern></journeyPatterns>
        </ServiceFrame>
        <ServiceCalendarFrame id="k" version="1">
          <dayTypes>
            <DayType id="DT" version="1"/>
            <DayType id="DT-idle" version="1"/>
          </dayTypes>
          <operatingPeriods><UicOperatingPeriod id="OP" version="1"><FromDate>2026-10-12T00:00:00</FromDate><ToDate>2026-10-18T00:00:00</ToDate><ValidDayBits>1111111</ValidDayBits></UicOperatingPeriod></operatingPeriods>
          <dayTypeAssignments><DayTypeAssignment id="DTA" version="1" order="1"><OperatingPeriodRef ref="OP" version="1"/><DayTypeRef ref="DT" version="1"/></DayTypeAssignment></dayTypeAssignments>
        </ServiceCalendarFrame>
        <TimetableFrame id="t" version="1"><vehicleJourneys>
          <ServiceJourney id="J" version="1">
            <dayTypes><DayTypeRef ref="DT" version="1"/></dayTypes>
            <ServiceJourneyPatternRef ref="P" version="1"/>
            <passingTimes>
              <TimetabledPassingTime version="1"><StopPointInJourneyPatternRef ref="P1" version="1"/><DepartureTime>08:00:00</DepartureTime></TimetabledPassingTime>
              <TimetabledPassingTime version="1"><StopPointInJourneyPatternRef ref="P2" version="1"/><ArrivalTime>08:10:00</ArrivalTime></TimetabledPassingTime>
            </passingTimes>
          </ServiceJourney>
        </vehicleJourneys></TimetableFrame>
      </frames></CompositeFrame></dataObjects>
    </PublicationDelivery>"#;
    let index = with_referrers(&[("line.xml".to_owned(), xml.to_owned())]);
    let referrers = index.referrers().expect("the references were kept");
    let unreferenced: BTreeSet<String> = referrers
        .unreferenced()
        .map(|object| describe(&referrers, object))
        .collect();
    assert_eq!(
        unreferenced,
        set(&[
            "StopPlace SP-idle",
            "ScheduledStopPoint Z",
            "DayType DT-idle in line.xml",
        ])
    );
}

/// Stops in the French profile's form: each stop place names its quays, each
/// quay its stop place. Those references are in the graph, but they state
/// what a stop is made of, not that it is used: a stop place and its quay no
/// stop point is placed at are listed as unreferenced all the same.
#[test]
fn a_stop_place_and_its_quay_naming_each_other_are_no_use_of_either() {
    let xml = r#"<PublicationDelivery xmlns="http://www.netex.org.uk/netex" version="1.10:FR-NETEX-2.3">
      <PublicationTimestamp>2026-10-14T00:00:00</PublicationTimestamp>
      <ParticipantRef>test</ParticipantRef>
      <dataObjects><GeneralFrame id="g" version="1"><members>
        <StopPlace id="SP-used" version="1"><Name>Used</Name>
          <quays><QuayRef ref="Q-used" version="1"/></quays></StopPlace>
        <StopPlace id="SP-idle" version="1"><Name>Idle</Name>
          <quays><QuayRef ref="Q-idle" version="1"/></quays></StopPlace>
        <Quay id="Q-used" version="1"><Name>Used 1</Name><SiteRef ref="SP-used" version="1"/></Quay>
        <Quay id="Q-idle" version="1"><Name>Idle 1</Name><SiteRef ref="SP-idle" version="1"/></Quay>
        <ScheduledStopPoint id="A" version="1"><Name>A</Name></ScheduledStopPoint>
        <PassengerStopAssignment id="PSA" version="1" order="1">
          <ScheduledStopPointRef ref="A" version="1"/><QuayRef ref="Q-used" version="1"/>
        </PassengerStopAssignment>
      </members></GeneralFrame></dataObjects>
    </PublicationDelivery>"#;
    let index = with_referrers(&[("stop.xml".to_owned(), xml.to_owned())]);
    assert_eq!(
        referrers_of(&index, "Quay", "Q-idle", None),
        set(&["QuayRef from StopPlace SP-idle"])
    );
    assert_eq!(
        referrers_of(&index, "StopPlace", "SP-idle", None),
        set(&["SiteRef from Quay Q-idle"])
    );
    let referrers = index.referrers().expect("the references were kept");
    let unreferenced: BTreeSet<String> = referrers
        .unreferenced()
        .map(|object| describe(&referrers, object))
        .collect();
    assert_eq!(unreferenced, set(&["StopPlace SP-idle", "Quay Q-idle"]));
}

#[test]
fn the_graph_is_the_same_whatever_order_and_threads_the_documents_are_read_in() {
    // Each object as text with what refers to it, as text: one list for the
    // delivery, however it was read.
    let listing = |index: &Index| -> Vec<String> {
        let referrers = index.referrers().expect("the references were kept");
        let mut lines: Vec<String> = referrers
            .objects()
            .map(|object| {
                let mut from: Vec<String> = referrers
                    .of(object)
                    .map(|reference| {
                        format!(
                            "{} {}",
                            reference.via,
                            describe(&referrers, reference.object)
                        )
                    })
                    .collect();
                from.sort();
                format!("{} <- {}", describe(&referrers, object), from.join(", "))
            })
            .collect();
        lines.sort();
        lines
    };
    let documents = own("");
    let expected = listing(&with_referrers(&documents));
    assert_eq!(listing(&with_referrers(&reversed(&documents))), expected);
    let source = Source::open(support::fixtures_dir().join("own")).expect("the directory opens");
    let options = ReadOptions::new(Selection::routing());
    let builders = source
        .read_each_with(
            || IndexBuilder::new().with_referrers(),
            |builder, entry, text| {
                builder
                    .add_streamed(text.expect("the document loads"), &options, &entry.name)
                    .expect("the document reads");
            },
        )
        .expect("the directory lists");
    let from_directory = IndexBuilder::merge_all(builders).finish();
    assert_eq!(listing(&from_directory), expected);
}
