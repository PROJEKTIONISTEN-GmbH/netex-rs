//! The merge of several deliveries into one (`netex_rs::merge`) — stops paired by identifier
//! and by global identifier, what differs between paired stops found, objects
//! stated alike kept once, identifiers taken renamed into the delivery's
//! codespace, journeys stated in two deliveries kept once.

use std::path::{Path, PathBuf};

use netex_rs::dataset::Source;
use netex_rs::index::{Index, JourneyId};
use netex_rs::merge::{self, Input, Report};
use netex_rs::stream::{ReadOptions, Selection};

/// The Austrian-style example delivery of netex-rs, where the repository has it.
fn austrian() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/own/at");
    if path.is_dir() {
        Some(path)
    } else {
        eprintln!(
            "SKIPPED: netex-rs's own examples are not at {}",
            path.display()
        );
        None
    }
}

/// A copy of the delivery at `from`, each document's text changed by `change`.
fn altered(from: &Path, change: impl Fn(&str, String) -> String) -> tempfile::TempDir {
    let copy = tempfile::tempdir().expect("a directory");
    for entry in std::fs::read_dir(from).expect("the delivery") {
        let path = entry.expect("a file").path();
        let name = path
            .file_name()
            .expect("a name")
            .to_string_lossy()
            .into_owned();
        let text = std::fs::read_to_string(&path).expect("a document");
        std::fs::write(copy.path().join(&name), change(&name, text)).expect("written");
    }
    copy
}

fn merged(deliveries: &[(&str, &Path)]) -> (tempfile::TempDir, Report) {
    let inputs: Vec<Input> = deliveries
        .iter()
        .map(|(label, path)| {
            Input::new(label, Source::open(path).expect("the delivery opens")).expect("a label")
        })
        .collect();
    let to = tempfile::tempdir().expect("a directory");
    let report = merge::merge(&inputs, to.path()).expect("merges");
    (to, report)
}

fn index(path: &Path) -> Index {
    Index::build(
        &Source::open(path).expect("opens"),
        &ReadOptions::new(Selection::routing()),
    )
    .expect("reads")
}

/// A journey as read: its identifier, stop points, times and days.
type Journey = (String, Vec<String>, Vec<String>, Vec<i64>);

/// Every journey, sorted.
fn journeys(index: &Index) -> Vec<Journey> {
    let mut all: Vec<_> = (0..index.journey_count() as u32)
        .map(JourneyId)
        .map(|journey| {
            let record = index.journey(journey);
            (
                record.id.to_owned(),
                record
                    .stop_points
                    .iter()
                    .map(|&point| index.stop_point(point).id.to_owned())
                    .collect(),
                record
                    .passing_times
                    .iter()
                    .map(|time| format!("{:?}-{:?}", time.arrival, time.departure))
                    .collect(),
                index.days_of(journey).collect(),
            )
        })
        .collect();
    all.sort();
    all
}

fn finding(report: &Report, class: &str) -> u64 {
    report
        .findings
        .iter()
        .find(|finding| finding.class == class)
        .map_or(0, |finding| finding.count)
}

#[test]
fn a_delivery_merged_with_itself_reads_as_itself() {
    let Some(at) = austrian() else {
        return;
    };
    let (to, report) = merged(&[("a", &at), ("b", &at)]);
    let read = index(to.path());
    assert_eq!(journeys(&read), journeys(&index(&at)));
    assert!(read.unresolved().is_empty(), "{:?}", read.unresolved());
    // The three stop places and four quays of the second are the first's.
    assert_eq!(finding(&report, "stop-paired-by-identifier"), 7);
    assert_eq!(finding(&report, "stop-named-otherwise"), 0);
    assert!(finding(&report, "object-alike") > 0);
    // Both documents define `at:ex:DayType:1`, for other days. The first
    // delivery answers the identifier with its line 100 document's; the
    // second's line 200 calendar is no such and would need another
    // identifier — but the journey using it is the first delivery's alike,
    // and the calendar is needed no longer.
    assert!(finding(&report, "object-no-longer-needed") > 0);
    assert_eq!(finding(&report, "identifier-renamed"), 0);
    assert_eq!(report.deliveries[1].documents_written, 0);
}

#[test]
fn stops_pair_by_global_identifier_differences_are_found_and_journeys_kept_once() {
    let Some(at) = austrian() else {
        return;
    };
    let other = altered(&at, |name, text| {
        if name.contains("_200_") {
            // Another identifier for Freibad and its quay, the same GlobalIDs;
            // the journey under another identifier, alike in all else.
            text.replace("at:ex:StopPlace:3\"", "at:ex:StopPlace:drei\"")
                .replace("at:ex:Quay:3:1\"", "at:ex:Quay:drei:1\"")
                .replace(
                    "at:ex:ServiceJourney:200-1\"",
                    "at:ex:ServiceJourney:200-1b\"",
                )
        } else {
            // Hauptplatz named otherwise; a journey half an hour later.
            text.replace(
                "<Name>Beispieldorf Hauptplatz</Name>",
                "<Name>Hauptplatz Beispieldorf</Name>",
            )
            .replace(
                "at:ex:ServiceJourney:100-v1\"",
                "at:ex:ServiceJourney:100-v1-spaet\"",
            )
            .replacen("<DepartureTime>08:00:00", "<DepartureTime>08:30:00", 1)
            .replacen("<ArrivalTime>08:12:00", "<ArrivalTime>08:42:00", 1)
        }
    });
    let (to, report) = merged(&[("a", &at), ("b", other.path())]);
    assert_eq!(finding(&report, "stop-paired-by-global-id"), 2);
    assert_eq!(finding(&report, "stop-named-otherwise"), 1);
    let named = report
        .findings
        .iter()
        .find(|finding| finding.class == "stop-named-otherwise")
        .expect("the finding");
    assert!(
        named.examples[0].contains("Beispieldorf Hauptplatz")
            && named.examples[0].contains("Hauptplatz Beispieldorf"),
        "{:?}",
        named.examples
    );
    assert_eq!(finding(&report, "journey-in-both"), 1);

    let read = index(to.path());
    assert!(read.unresolved().is_empty(), "{:?}", read.unresolved());
    let all = journeys(&read);
    let mut expected = journeys(&index(&at));
    let later = journeys(&index(other.path()))
        .into_iter()
        .find(|journey| journey.0 == "at:ex:ServiceJourney:100-v1-spaet")
        .expect("the later journey");
    expected.push(later);
    expected.sort();
    assert_eq!(all, expected);
    // The stop place kept is the first delivery's, under its name.
    let stop = read.stop_by_id("at:ex:StopPlace:1").expect("Hauptplatz");
    assert_eq!(read.stop(stop).name, "Beispieldorf Hauptplatz");
    assert!(read.stop_by_id("at:ex:StopPlace:drei").is_none());
}

#[test]
fn an_identifier_taken_with_other_content_is_renamed_into_the_delivery_s_codespace() {
    let Some(at) = austrian() else {
        return;
    };
    // The second delivery's line 200 runs on weekdays under the same
    // identifiers of its calendar, and under another journey identifier.
    let other = altered(&at, |name, text| {
        if name.contains("_200_") {
            text.replace("<ValidDayBits>0000011", "<ValidDayBits>1111100")
                .replace(
                    "at:ex:ServiceJourney:200-1\"",
                    "at:ex:ServiceJourney:200-werktags\"",
                )
        } else {
            text
        }
    });
    let (to, report) = merged(&[("a", &at), ("b", other.path())]);
    assert!(finding(&report, "identifier-renamed") > 0);
    let renamed = report
        .findings
        .iter()
        .find(|finding| finding.class == "identifier-renamed")
        .expect("the finding");
    assert!(
        renamed
            .examples
            .iter()
            .any(|example| example.contains("b:at:ex:UicOperatingPeriod:1")),
        "{:?}",
        renamed.examples
    );
    let read = index(to.path());
    assert!(read.unresolved().is_empty(), "{:?}", read.unresolved());
    let weekdays = read
        .journey_by_id("at:ex:ServiceJourney:200-werktags")
        .expect("the weekday journey");
    let other_index = index(other.path());
    let stated = other_index
        .journey_by_id("at:ex:ServiceJourney:200-werktags")
        .expect("in the second delivery");
    assert_eq!(
        read.days_of(weekdays).collect::<Vec<_>>(),
        other_index.days_of(stated).collect::<Vec<_>>()
    );
    assert_eq!(read.journey_count(), 4);
}

/// Adds the park and ride at Bahnhof, named `name`, to line 200's document.
fn with_parking(name: &'static str) -> impl Fn(&str, String) -> String {
    move |file, text| {
        if !file.contains("_200_") {
            return text;
        }
        let parking = format!(
            "          </stopPlaces>
          <parkings>
            <Parking id=\"at:ex:Parking:1\" version=\"1\">
              <keyList><KeyValue><Key>GlobalID</Key><Value>at:99:9</Value></KeyValue></keyList>
              <Name>{name}</Name>
            </Parking>
          </parkings>"
        );
        text.replacen("          </stopPlaces>", &parking, 1)
    }
}

#[test]
fn a_place_of_the_same_identifier_and_global_identifier_is_one_stated_otherwise() {
    let Some(at) = austrian() else {
        return;
    };
    // Both deliveries state the park and ride at Bahnhof, under the same
    // identifier and GlobalID, named otherwise.
    let first = altered(&at, with_parking("P+R Bahnhof"));
    let second = altered(&at, with_parking("Park and Ride Bahnhof"));
    let (to, report) = merged(&[("a", first.path()), ("b", second.path())]);
    assert_eq!(finding(&report, "place-stated-otherwise"), 1);
    assert_eq!(finding(&report, "identifier-renamed"), 0);
    assert_eq!(report.deliveries[1].documents_written, 0);
    assert_eq!(report.deliveries[1].stop_places_paired, 3);
    let written: String = ["1-a", "2-b"]
        .iter()
        .filter_map(|directory| std::fs::read_dir(to.path().join(directory)).ok())
        .flatten()
        .map(|entry| std::fs::read_to_string(entry.expect("a file").path()).expect("a document"))
        .collect();
    assert_eq!(written.matches("<Parking ").count(), 1);
    assert!(written.contains("P+R Bahnhof"));
}

#[test]
fn a_quay_pairs_by_its_own_identifier_or_global_identifier_and_a_place_kept_apart_stays() {
    let Some(at) = austrian() else {
        return;
    };
    // The second delivery names Hauptplatz's quay otherwise, with the same
    // GlobalID; and states Seehof, a stop place only a park and ride refers
    // to, with a quay the first does not have.
    let seehof = |quays: &str| {
        String::from(
            "          </stopPlaces>
          <parkings>
            <Parking id=\"at:ex:Parking:9\" version=\"1\">
              <keyList><KeyValue><Key>GlobalID</Key><Value>at:99:9:1</Value></KeyValue></keyList>
              <Name>P+R Seehof</Name>
              <ParentSiteRef ref=\"at:ex:StopPlace:9\" version=\"1\"/>
            </Parking>
          </parkings>",
        )
        .replacen(
            "          </stopPlaces>",
            &format!(
                "            <StopPlace id=\"at:ex:StopPlace:9\" version=\"1\">
              <Name>Seehof</Name>
              <quays>{quays}</quays>
            </StopPlace>
          </stopPlaces>"
            ),
            1,
        )
    };
    let first = altered(&at, |name, text| {
        if name.contains("_200_") {
            text.replacen(
                "          </stopPlaces>",
                &seehof("<Quay id=\"at:ex:Quay:9:1\" version=\"1\"><Name>Seehof 1</Name></Quay>"),
                1,
            )
        } else {
            text
        }
    });
    let second = altered(&at, |name, text| {
        if name.contains("_200_") {
            text.replacen(
                "          </stopPlaces>",
                &seehof(
                    "<Quay id=\"at:ex:Quay:9:1\" version=\"1\"><Name>Seehof 1</Name></Quay>\
                     <Quay id=\"at:ex:Quay:9:2\" version=\"1\"><Name>Seehof 2</Name></Quay>",
                ),
                1,
            )
        } else {
            text.replace("at:ex:Quay:1:1\"", "at:ex:Quay:1:eins\"")
        }
    });
    let (to, report) = merged(&[("a", first.path()), ("b", second.path())]);
    // Hauptplatz by identifier, its quay by GlobalID.
    assert_eq!(finding(&report, "stop-paired-by-global-id"), 1);
    assert_eq!(finding(&report, "stop-kept-apart"), 1);
    // Line 200's calendar of the second delivery is needed no longer, as
    // when a delivery is merged with itself; Seehof is not.
    assert!(report
        .findings
        .iter()
        .filter(|finding| finding.class == "object-no-longer-needed")
        .flat_map(|finding| &finding.examples)
        .all(|example| !example.contains("StopPlace")));
    let read = index(to.path());
    assert!(read.unresolved().is_empty(), "{:?}", read.unresolved());
    assert!(read.stop_by_id("b:at:ex:StopPlace:9").is_some());
    assert!(read.stop_by_id("at:ex:Quay:1:eins").is_none());
}

/// A delivery in the Nordic profile, with the stop register it names read
/// along, merged with a delivery of the European profile, in either rank:
/// no identifier of the one is the other's, so nothing is paired, left out
/// or renamed; every stop and journey of both is in the merge, as each
/// delivery reads alone.
#[test]
fn a_nordic_delivery_and_an_epip_delivery_merge_without_touching_each_other() {
    let own = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/own");
    let nordic = || {
        Source::open(own.join("nordic"))
            .and_then(|delivery| delivery.with(Source::open(own.join("nordic-stops"))?))
            .expect("the delivery and its register open")
    };
    let epip = || Source::open(own.join("epip")).expect("the delivery opens");
    let alone = |source: Source| {
        Index::build(&source, &ReadOptions::new(Selection::routing())).expect("reads")
    };
    let mut expected = journeys(&alone(nordic()));
    expected.extend(journeys(&alone(epip())));
    expected.sort();
    let stops = alone(nordic()).stop_count() + alone(epip()).stop_count();
    for nordic_first in [true, false] {
        let (nordic, epip) = (
            Input::new("nordic", nordic()).expect("a label"),
            Input::new("epip", epip()).expect("a label"),
        );
        let inputs = if nordic_first {
            [nordic, epip]
        } else {
            [epip, nordic]
        };
        let to = tempfile::tempdir().expect("a directory");
        let report = merge::merge(&inputs, to.path()).expect("merges");
        let classes: Vec<&str> = report.findings.iter().map(|f| f.class).collect();
        assert_eq!(
            classes,
            Vec::<&str>::new(),
            "nothing paired, alike or renamed"
        );
        for delivery in &report.deliveries {
            assert_eq!(delivery.objects_left_out, 0, "{}", delivery.label);
            assert_eq!(delivery.objects_renamed, 0, "{}", delivery.label);
        }
        let read = index(to.path());
        assert!(read.unresolved().is_empty(), "{:?}", read.unresolved());
        assert_eq!(read.stop_count(), stops);
        assert_eq!(journeys(&read), expected);
    }
}
