//! What the index decides about what a delivery states, as findings: each
//! named by its concern, the document and entity it is about and what else it
//! names, counted always, and listed in the same order whatever order the
//! documents were read in.

mod support;

use netex_rs::index::{Concern, Index};
use support::timetable::{line_document, merged, reversed, serially, two_calendars, Timetable};

/// The findings of one concern, written out.
fn of(index: &Index, concern: Concern) -> Vec<String> {
    index
        .findings()
        .filter(|finding| finding.concern == concern)
        .map(|finding| finding.to_string())
        .collect()
}

#[test]
fn a_redefinition_with_other_content_is_a_finding_naming_both_documents() {
    let documents = two_calendars();
    let (e25, f26) = (&documents[0].0, &documents[1].0);
    for index in [serially(&documents), merged(&reversed(&documents))] {
        let findings: Vec<_> = index
            .findings()
            .filter(|finding| finding.concern == Concern::RedefinedOtherwise)
            .collect();
        let named: Vec<(&str, &str, &str, Option<&str>)> = findings
            .iter()
            .map(|f| (f.element, f.entity, f.document, f.about))
            .collect();
        assert_eq!(
            named,
            [
                ("DayType", "DT", f26.as_str(), Some(e25.as_str())),
                ("UicOperatingPeriod", "OP", f26.as_str(), Some(e25.as_str())),
            ],
            "the period and the day type it gives days to differ; the journeys \
             and the line, pattern and stop points the documents share do not"
        );
        assert_eq!(index.finding_count(Concern::RedefinedOtherwise), 2);
    }
}

#[test]
fn a_journey_defined_again_with_other_content_is_a_finding_and_found_in_its_document() {
    let documents = vec![
        (
            "A.xml".to_owned(),
            line_document("J", "1000000", "08:00:00"),
        ),
        (
            "B.xml".to_owned(),
            line_document("J", "1000000", "09:00:00"),
        ),
    ];
    for index in [serially(&documents), merged(&reversed(&documents))] {
        assert_eq!(
            of(&index, Concern::RedefinedOtherwise),
            ["redefined with other content: ServiceJourney J in B.xml, kept as A.xml defines it"]
        );
        let in_b = index
            .journey_in_document("J", "B.xml")
            .expect("B's journey");
        assert_eq!(index.journey(in_b).document, "B.xml");
        assert_eq!(
            index.journey(in_b).passing_times[0].departure,
            Some(9 * 3600)
        );
        let by_id = index.journey_by_id("J").expect("the delivery's journey");
        assert_eq!(index.journey(by_id).document, "A.xml");
        assert_eq!(index.journey_in_document("J", "C.xml"), None);
    }
}

/// A stop place another document names otherwise is another stop, and so is
/// its quay: the identifier is that document's own. A third document naming
/// it as the first does is the first one's stop, and no finding.
#[test]
fn a_stop_place_named_otherwise_in_another_document_is_a_stop_of_its_own() {
    let documents = vec![
        (
            "A.xml".to_owned(),
            Timetable::new().stop_place("SP", "Linz Hbf").xml(),
        ),
        (
            "B.xml".to_owned(),
            Timetable::new().stop_place("SP", "Linz Hauptbahnhof").xml(),
        ),
        (
            "C.xml".to_owned(),
            Timetable::new().stop_place("SP", "Linz Hbf").xml(),
        ),
    ];
    for index in [
        serially(&documents),
        serially(&reversed(&documents)),
        merged(&reversed(&documents)),
    ] {
        assert_eq!(index.stop_count(), 4, "two stop places, two quays");
        let place = index.stop_by_id("SP").expect("the stop place");
        assert_eq!(
            index.stop(place).name,
            "Linz Hbf",
            "the identifier alone names the stop of the document named first"
        );
        assert_eq!(
            of(&index, Concern::LocalIdentifier),
            [
                "identifier naming another stop than in another document, a stop of its own: Quay SP:1 in B.xml (A.xml)",
                "identifier naming another stop than in another document, a stop of its own: StopPlace SP in B.xml (A.xml)",
            ],
            "C names it as A does"
        );
        assert!(of(&index, Concern::RedefinedOtherwise).is_empty());
    }
}

/// A stop place another document places otherwise under the same name is
/// the same stop: one stop, as the document named first places it, and a
/// finding.
#[test]
fn a_stop_place_placed_otherwise_in_another_document_is_one_and_a_finding() {
    let documents = vec![
        (
            "A.xml".to_owned(),
            Timetable::new()
                .stop_place_at("SP", "Linz Hbf", (14.2913, 48.2901))
                .xml(),
        ),
        (
            "B.xml".to_owned(),
            Timetable::new()
                .stop_place_at("SP", "Linz Hbf", (14.2920, 48.2905))
                .xml(),
        ),
    ];
    for index in [serially(&documents), merged(&reversed(&documents))] {
        assert_eq!(index.stop_count(), 1);
        let place = index.stop(index.stop_by_id("SP").expect("the stop place"));
        assert_eq!(place.longitude, Some(14.2913));
        assert_eq!(
            of(&index, Concern::RedefinedOtherwise),
            ["redefined with other content: StopPlace SP in B.xml, kept as A.xml defines it"]
        );
        assert!(of(&index, Concern::LocalIdentifier).is_empty());
    }
}

#[test]
fn findings_are_counted_and_listed_alike_whatever_order_the_documents_come_in() {
    let mut documents = two_calendars();
    documents.push((
        "A.xml".to_owned(),
        line_document("J", "1000000", "08:00:00"),
    ));
    documents.push((
        "B.xml".to_owned(),
        line_document("J", "1000000", "09:00:00"),
    ));
    let listed = |index: &Index| -> Vec<String> {
        index
            .findings()
            .map(|finding| finding.to_string())
            .collect()
    };
    let expected = listed(&serially(&documents));
    assert!(!expected.is_empty());
    let index = serially(&documents);
    let findings: Vec<_> = index.findings().collect();
    assert!(
        findings.is_sorted(),
        "listed in the order of Finding's Ord: {findings:?}"
    );
    assert_eq!(listed(&serially(&reversed(&documents))), expected);
    assert_eq!(listed(&merged(&documents)), expected);
    assert_eq!(listed(&merged(&reversed(&documents))), expected);
}
