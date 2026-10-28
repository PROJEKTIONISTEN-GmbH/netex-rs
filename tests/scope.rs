//! Where a reference resolves: in the document it stands in first, then in the
//! delivery — so that a delivery reusing an identifier per document, as the
//! Linz AG's does with its calendars, is read the same whatever order its
//! documents come in, and the index of a delivery does not depend on the
//! number of threads that read it.

mod support;

use netex_rs::dataset::Source;
use netex_rs::index::Index;
use netex_rs::model::Date;
use netex_rs::stream::{ReadOptions, Selection};
use support::render::{assert_same, render};
use support::timetable::{
    line_document, merged, reversed, serially, two_calendars, Point, Timetable,
};

#[test]
fn each_document_reads_its_own_calendar_whatever_order_the_documents_come_in() {
    let documents = two_calendars();
    let (monday, tuesday) = (Date::from("2026-10-12"), Date::from("2026-10-13"));
    for index in [
        serially(&documents),
        serially(&reversed(&documents)),
        merged(&documents),
        merged(&reversed(&documents)),
    ] {
        let e25 = index.journey_by_id("J-e25").expect("the first journey");
        let f26 = index.journey_by_id("J-f26").expect("the second journey");
        assert!(
            index.runs_on(e25, &monday) && !index.runs_on(e25, &tuesday),
            "the e25 journey runs on the days of the e25 document's calendar"
        );
        assert!(
            index.runs_on(f26, &tuesday) && !index.runs_on(f26, &monday),
            "the f26 journey runs on the days of the f26 document's calendar"
        );
    }
}

#[test]
fn a_document_naming_what_it_does_not_define_resolves_it_in_the_delivery() {
    // The Swiss layout: the calendar in a document of its own, the journeys
    // in documents that name it.
    let calendar = Timetable::new()
        .day_type("DT")
        .uic_period("OP", "2026-10-12", "2026-10-18", "0010000")
        .assignment("DTA", 1, "OP", "DT", true)
        .xml();
    let journeys = Timetable::new()
        .line("L", "1", None)
        .stop_point("A")
        .stop_point("B")
        .pattern("P", "L", &[Point::at("P1", "A"), Point::at("P2", "B")])
        .journey(
            "J",
            ("L", "1"),
            "P",
            &["DT"],
            &[
                ("P1", None, Some("08:00:00")),
                ("P2", Some("08:30:00"), None),
            ],
        )
        .xml();
    let documents = vec![
        ("SERVICECALENDAR_1.xml".to_owned(), calendar),
        ("TIMETABLE_1.xml".to_owned(), journeys),
    ];
    for index in [serially(&documents), merged(&reversed(&documents))] {
        let journey = index.journey_by_id("J").expect("the journey");
        assert!(index.runs_on(journey, &Date::from("2026-10-14")));
        assert!(!index.runs_on(journey, &Date::from("2026-10-12")));
        assert!(index.unresolved().is_empty(), "{:?}", index.unresolved());
    }
}

#[test]
fn a_journey_defined_again_with_other_content_is_kept_as_each_document_states_it() {
    // One identifier, one version, other times in each document.
    let first = line_document("J", "1000000", "08:00:00");
    let second = line_document("J", "1000000", "09:00:00");
    let documents = vec![
        ("A.xml".to_owned(), first.clone()),
        ("B.xml".to_owned(), second),
    ];
    for index in [serially(&documents), merged(&reversed(&documents))] {
        assert_eq!(index.journey_count(), 2, "both runs are in the index");
        let departures: Vec<Option<u32>> = (0..2)
            .map(|j| index.journey(netex_rs::index::JourneyId(j)).passing_times[0].departure)
            .collect();
        assert!(departures.contains(&Some(8 * 3600)) && departures.contains(&Some(9 * 3600)));
        let by_id = index.journey_by_id("J").expect("the journey");
        assert_eq!(
            index.journey(by_id).passing_times[0].departure,
            Some(8 * 3600),
            "the delivery answers with the definition of the document named first"
        );
    }
    // Stated twice alike, it is one journey.
    let documents = vec![
        ("A.xml".to_owned(), first.clone()),
        ("B.xml".to_owned(), first),
    ];
    let index = serially(&documents);
    assert_eq!(index.journey_count(), 1);
    assert!(index.redefinition_count() > 0);
}

/// Every document of `documents` as a file of `directory`.
fn as_files(directory: &std::path::Path, documents: &[(String, String)]) {
    std::fs::create_dir_all(directory).expect("the scratch directory");
    for (name, xml) in documents {
        std::fs::write(directory.join(name), xml).expect("the document is written");
    }
}

#[test]
fn the_index_holds_the_same_whatever_order_and_threads_the_documents_are_read_in() {
    let mut documents: Vec<(String, String)> = support::delfi_fixtures()
        .into_iter()
        .map(|fixture| (fixture.name, fixture.xml))
        .collect();
    documents.extend(two_calendars());
    documents.push((
        "NX-PI_01_at_esg_LINE_esg_1-1-A-e25_20260923.xml".to_owned(),
        line_document("J-A", "0010000", "10:00:00"),
    ));
    let expected = render(&serially(&documents));
    assert_same(
        &render(&serially(&reversed(&documents))),
        &expected,
        "read in reverse",
    );
    assert_same(&render(&merged(&documents)), &expected, "merged");
    assert_same(
        &render(&merged(&reversed(&documents))),
        &expected,
        "merged in reverse",
    );
    let directory = support::scratch_dir("scope-threads");
    as_files(&directory, &documents);
    let source = Source::open(&directory).expect("the directory opens");
    for threads in [1, 3, 8] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect("a pool");
        let index = pool
            .install(|| Index::build(&source, &ReadOptions::new(Selection::routing())))
            .expect("the delivery reads");
        assert_same(&render(&index), &expected, &format!("on {threads} threads"));
    }
}
