//! The indexes over the DELFI fixtures: stops by DHID, journeys by line and
//! day, walks and interchanges by stop, and references that do not resolve.

mod support;

use netex_rs::index::{IndexBuilder, StopKind};
use netex_rs::model::Date;

fn index_of(names: &[&str]) -> netex_rs::index::Index {
    let mut builder = IndexBuilder::new();
    for fixture in support::delfi_fixtures() {
        if names.iter().any(|name| fixture.name.contains(name)) {
            let document = netex_rs::read(&fixture.xml).expect("the fixture reads");
            builder.add(&document, &fixture.name);
        }
    }
    builder.finish()
}

#[test]
fn stops_are_found_by_dhid_with_their_quays_and_levels() {
    let index = index_of(&["WEB-WEB_HB-511699"]);
    let place = index
        .stop_by_dhid("de:03356:72118")
        .expect("the stop place is indexed");
    let stop = index.stop(place);
    assert_eq!(stop.kind, StopKind::StopPlace);
    assert_eq!(stop.name, "Osterholz-Scharmbeck-Heilshorn Grundschule");
    assert_eq!(stop.id, "DE::StopPlace:9072118_MASTER::");
    assert!(stop.parent.is_none());
    assert!(stop.latitude.is_some_and(|lat| (53.0..54.0).contains(&lat)));

    let quay = index
        .stop_by_dhid("de:03356:72118::1")
        .expect("the quay is indexed");
    let quay = index.stop(quay);
    assert_eq!(quay.kind, StopKind::Quay);
    assert_eq!(quay.parent, Some(place));
    assert_eq!(
        quay.level,
        Some("0"),
        "the quay's LevelRef resolves to the level's short name"
    );

    assert_eq!(
        index.stop_by_id("DE::Quay:176188_MSTMASTER::"),
        index.stop_by_dhid("de:03356:72118::2")
    );
    assert!(index.stop_by_dhid("de:00000:0").is_none());
}

#[test]
fn walks_are_indexed_by_stop_and_match_the_extractor() {
    let index = index_of(&["WEB-WEB_HB-511699"]);
    let from = index.stop_by_dhid("de:03356:72118::1").expect("quay 1");
    let to = index.stop_by_dhid("de:03356:72118::2").expect("quay 2");
    let walks = index.transfers_from(from);
    assert_eq!(walks.len(), 1, "one site connection starts at quay 1");
    assert_eq!(walks[0].to, to);
    assert_eq!(walks[0].seconds, 120);
    assert!(walks[0].both_ways);
    // A two-way walk is reachable from its other end too.
    assert_eq!(index.transfers_from(to).len(), 1);
    assert_eq!(index.transfers_from(to)[0].to, from);
}

#[test]
fn journeys_are_indexed_by_line_and_run_on_the_days_their_bits_say() {
    let index = index_of(&["WEB-WEB_HB-511699"]);
    let line = index
        .line_by_id("DE::Line:6339460::")
        .expect("the line is indexed");
    let journeys = index.journeys_of_line(line);
    assert_eq!(journeys.len(), 2);
    let journey = index.journey(journeys[0]);
    assert_eq!(journey.line, Some(line));
    assert_eq!(journey.stop_points.len(), journey.passing_times.len());
    assert!(!journey.stop_points.is_empty());
    let first = journey.passing_times[0];
    assert_eq!(
        first.departure,
        Some(10 * 3600 + 45 * 60),
        "the first stop is left at 10:45"
    );
    // ValidDayBits 1000000100000010… from Monday 2026-09-07: Mondays only.
    assert!(index.runs_on(journeys[0], &Date::from("2026-09-07")));
    assert!(!index.runs_on(journeys[0], &Date::from("2026-09-08")));
    assert!(index.runs_on(journeys[0], &Date::from("2026-09-14")));
    assert!(
        !index.runs_on(journeys[0], &Date::from("2026-09-06")),
        "before the period"
    );
    assert!(
        !index.runs_on(journeys[0], &Date::from("2027-01-01")),
        "after the period"
    );
    // Every stop point of the pattern is assigned to a stop.
    for stop_point in journey.stop_points {
        assert!(index.stop_of_stop_point(*stop_point).is_some());
    }
}

/// The RE19 line file states 148 interchanges, every one of them from and to
/// journeys of other line files: alone, it yields findings, not interchanges.
#[test]
fn interchanges_to_journeys_of_other_files_are_findings_in_a_single_file_index() {
    let index = index_of(&["DBDB-8015FR-RE19"]);
    assert_eq!(index.journey_count(), 11);
    let line = index
        .line_by_id("DE::Line:162349::")
        .expect("the line is indexed");
    assert_eq!(index.journeys_of_line(line).len(), 11);
    assert_eq!(index.interchange_count(), 0);
    let from: Vec<_> = index
        .unresolved()
        .iter()
        .filter(|u| u.element == "FromJourneyRef")
        .collect();
    let to: Vec<_> = index
        .unresolved()
        .iter()
        .filter(|u| u.element == "ToJourneyRef")
        .collect();
    assert_eq!((from.len(), to.len()), (74, 74));
    assert_eq!(index.unresolved().len(), 148, "nothing else is unresolved");
    assert!(from[0]
        .referrer
        .starts_with("ServiceJourneyInterchange DE::ServiceJourneyInterchange:"));
    assert_eq!(from[0].version.as_deref(), Some("1789823040"));
    // The `versionRef` the delivery writes on the distributor side is read as the version.
    assert_eq!(to[0].version.as_deref(), Some("1789823040"));
}

/// With the line file the RE19 interchanges lead to, every one of them
/// resolves across the two files and is indexed at its stop point.
#[test]
fn interchanges_resolve_across_files_and_are_indexed_by_stop_point() {
    let index = index_of(&["DBDB-8015FR-RE19", "DBDB-87____-TER"]);
    assert_eq!(index.journey_count(), 11 + 78);
    assert_eq!(
        index.interchange_count(),
        148,
        "all of the RE19 file's interchanges"
    );
    let at = index
        .stop_point_by_id("DE::ScheduledStopPoint:8700439_DBDB_::")
        .expect("the stop point both lines share");
    assert_eq!(index.interchanges_at(at).len(), 148);
    let interchange = index.interchanges_at(at)[0];
    assert_eq!(
        index.journey(interchange.from_journey).id,
        "DE::ServiceJourney:3419134594_0::"
    );
    assert_eq!(
        index.journey(interchange.to_journey).id,
        "DE::ServiceJourney:3419352309_0::"
    );
    assert_eq!(interchange.from_stop_point, at);
    assert_eq!(interchange.to_stop_point, at);
    assert!(interchange.stay_seated);
    assert!(!interchange.guaranteed);
    assert_eq!(interchange.maximum_wait_seconds, None);
    // The TER file's own interchanges lead on to a third file: findings again.
    assert!(index
        .unresolved()
        .iter()
        .all(|u| u.origin.contains("DBDB-87____-TER")));
    assert_eq!(index.unresolved().len(), 122);
}

#[test]
fn an_unresolved_reference_is_a_finding_and_not_an_error() {
    let mut builder = IndexBuilder::new();
    let fixture = support::delfi_fixtures()
        .into_iter()
        .find(|f| f.name.contains("vgm_d-VKU_50-12816"))
        .expect("the smallest fixture");
    // Rewrite the stop point reference of the one PassengerStopAssignment.
    let assignment = fixture
        .xml
        .find("<PassengerStopAssignment ")
        .expect("the fixture has an assignment");
    let reference = assignment
        + fixture.xml[assignment..]
            .find("<ScheduledStopPointRef ref=\"")
            .expect("with a stop point reference")
        + "<ScheduledStopPointRef ref=\"".len();
    let end = reference
        + fixture.xml[reference..]
            .find('"')
            .expect("the reference closes");
    let broken = format!(
        "{}DE::ScheduledStopPoint:missing::{}",
        &fixture.xml[..reference],
        &fixture.xml[end..]
    );
    let document = netex_rs::read(&broken).expect("the document still reads");
    builder.add(&document, "broken.xml");
    let index = builder.finish();
    let finding = index
        .unresolved()
        .iter()
        .find(|u| u.id == "DE::ScheduledStopPoint:missing::")
        .expect("the dangling reference is reported");
    assert_eq!(finding.origin, "broken.xml");
    assert_eq!(finding.element, "ScheduledStopPointRef");
    assert_eq!(
        finding.referrer,
        "PassengerStopAssignment DE::PassengerStopAssignment:119914::"
    );
    assert_eq!(finding.version.as_deref(), Some("1789823040"));
    assert_eq!(
        index.stop_count(),
        1,
        "the stop place itself is still indexed"
    );
}

#[test]
fn documents_that_repeat_shared_entities_are_merged_without_duplicates() {
    let names = [
        "vgm_d-VKU_50",
        "DBDB-54____",
        "WEB-WEB_HB",
        "DBDB-8015FR",
        "DBDB-87____",
    ];
    let all = index_of(&names);
    let each: usize = names
        .iter()
        .map(|name| index_of(&[name]).journey_count())
        .sum();
    assert_eq!(
        all.journey_count(),
        each,
        "journeys are per line file, never shared"
    );
    assert_eq!(all.document_count(), 5);
    // The RE19 and TER files both define the stop point their interchanges
    // meet at; the second definition is a redefinition, not a second entity.
    let shared = index_of(&["DBDB-8015FR", "DBDB-87____"]);
    assert_eq!(
        shared.stop_point_count(),
        index_of(&["DBDB-8015FR"]).stop_point_count()
            + index_of(&["DBDB-87____"]).stop_point_count()
            - 1
    );
    assert!(shared.redefinition_count() > 0);
    assert!(all.stop_count() > 0);
    let small = index_of(&["vgm_d-VKU_50", "DBDB-54____", "WEB-WEB_HB"]);
    assert!(
        small.unresolved().is_empty(),
        "{:?}",
        small.unresolved().first()
    );
}
