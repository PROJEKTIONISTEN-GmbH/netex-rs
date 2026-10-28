//! `netex extract`: a closed part of a delivery — the journeys chosen by
//! operator, line, area and period with everything they refer to —, written as
//! NeTEx of the source's release and read back as the same journeys.

use std::path::{Path, PathBuf};
use std::process::Command;

use netex2gtfs::{Delivery, Reading};
use netex_cli::extract::{self, Area, Leaving, Report, Selection};
use netex_rs::dataset::Source;
use netex_rs::index::{Index, JourneyId};
use netex_rs::model::Date;
use netex_rs::stream::{ReadOptions, Selection as Kinds};

/// A fixture directory of netex-rs, where the repository has it.
fn core_fixture(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures")
        .join(name);
    if path.is_dir() {
        Some(path)
    } else {
        eprintln!("SKIPPED: netex-rs's fixtures are not at {}", path.display());
        None
    }
}

/// Extracts from the delivery at `path` into a new directory.
fn extracted(path: &Path, selection: &Selection) -> (tempfile::TempDir, Report) {
    let source = Source::open(path).expect("the delivery opens");
    let delivery = Delivery::read(&source, Reading::default()).expect("the delivery reads");
    let to = tempfile::tempdir().expect("a directory");
    let report = extract::extract(&source, &delivery, selection, to.path()).expect("extracts");
    (to, report)
}

fn index(path: &Path) -> Index {
    Index::build(
        &Source::open(path).expect("opens"),
        &ReadOptions::new(Kinds::routing()),
    )
    .expect("reads")
}

/// A journey as read: its stop points, times and days.
fn journey(index: &Index, journey: JourneyId) -> (Vec<String>, Vec<String>, Vec<i64>) {
    let record = index.journey(journey);
    (
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
}

/// Every journey of `part` against the journey of the same identifier and
/// document in `whole`; the identifiers of the part's journeys.
fn reads_as_in_whole(part: &Index, whole: &Index) -> Vec<String> {
    let mut ids = Vec::new();
    for number in 0..part.journey_count() as u32 {
        let record = part.journey(JourneyId(number));
        let same = whole
            .journey_in_document(record.id, record.document)
            .unwrap_or_else(|| panic!("{} is in the whole delivery", record.id));
        assert_eq!(
            journey(part, JourneyId(number)),
            journey(whole, same),
            "{} reads as in the whole delivery",
            record.id
        );
        ids.push(record.id.to_owned());
    }
    ids.sort();
    ids
}

fn finding(report: &Report, class: &str) -> u64 {
    report
        .findings
        .iter()
        .find(|finding| finding.class == class)
        .map_or(0, |finding| finding.count)
}

/// Around Beispielstadt Bahnhof and Markt, not the Klinikum.
fn town_centre() -> Area {
    Area::from_box("9.73,52.37,9.74,52.38").expect("a box")
}

#[test]
fn a_line_extracted_reads_as_its_journeys_of_the_whole_delivery() {
    let Some(delfi) = core_fixture("delfi") else {
        return;
    };
    let selection = Selection {
        lines: vec!["DE::Line:162349::".to_owned()],
        ..Selection::default()
    };
    let (part, report) = extracted(&delfi, &selection);
    let whole = index(&delfi);
    let read = index(part.path());
    let line = whole.line_by_id("DE::Line:162349::").expect("RE 19");
    let mut expected: Vec<String> = whole
        .journeys_of_line(line)
        .iter()
        .map(|&journey| whole.journey(journey).id.to_owned())
        .collect();
    expected.sort();
    assert_eq!(reads_as_in_whole(&read, &whole), expected);
    assert_eq!(report.counts.journeys_selected, expected.len() as u64);
    // The RE 19 file's interchanges to the TER's journeys are left out, the
    // TER's file not written; those between journeys kept stay.
    let kept = |index: &Index, interchange: &netex_rs::index::Interchange| {
        [interchange.from_journey, interchange.to_journey]
            .iter()
            .all(|&journey| expected.contains(&index.journey(journey).id.to_owned()))
    };
    let within = (0..whole.stop_point_count() as u32)
        .flat_map(|point| whole.interchanges_at(netex_rs::index::StopPointId(point)))
        .filter(|interchange| kept(&whole, interchange))
        .count();
    assert_eq!(read.interchange_count(), within);
    assert!(finding(&report, "interchange-left-out") > 0);
    assert_eq!(report.counts.documents_written, 1);
    assert!(
        read.unresolved().is_empty(),
        "{:?}",
        &read.unresolved()[..1]
    );
}

#[test]
fn journeys_calling_in_an_area_are_kept_whole_or_cut_to_it() {
    let Some(epip) = core_fixture("own/epip") else {
        return;
    };
    let whole = index(&epip);
    let selection = Selection {
        area: Some(town_centre()),
        ..Selection::default()
    };
    let (part, report) = extracted(&epip, &selection);
    let read = index(part.path());
    assert_eq!(
        reads_as_in_whole(&read, &whole),
        ["ex:ServiceJourney:12-0700", "ex:ServiceJourney:12-2350"]
    );
    assert_eq!(finding(&report, "journey-leaves-area"), 2);

    let (part, report) = extracted(
        &epip,
        &Selection {
            leaving: Leaving::Cut,
            ..selection
        },
    );
    let read = index(part.path());
    assert_eq!(read.journey_count(), 2);
    let first = read
        .journey_by_id("ex:ServiceJourney:12-0700")
        .expect("the morning journey");
    let (points, times, days) = journey(&read, first);
    assert_eq!(
        points,
        [
            "ex:ScheduledStopPoint:bahnhof",
            "ex:ScheduledStopPoint:markt"
        ]
    );
    assert_eq!(times.len(), 2);
    assert_eq!(
        days,
        journey(
            &whole,
            whole
                .journey_by_id("ex:ServiceJourney:12-0700")
                .expect("in the whole")
        )
        .2
    );
    assert_eq!(finding(&report, "journey-cut"), 2);

    // At the Klinikum the journeys only end: a part of one call is no journey.
    let (part, report) = extracted(
        &epip,
        &Selection {
            area: Some(Area::from_box("9.75,52.36,9.76,52.37").expect("a box")),
            leaving: Leaving::Cut,
            ..Selection::default()
        },
    );
    assert_eq!(finding(&report, "journey-touches-area"), 2);
    assert_eq!(report.counts.journeys_selected, 0);
    assert_eq!(report.counts.documents_written, 0);
    assert!(std::fs::read_dir(part.path())
        .expect("the directory")
        .next()
        .is_none());
}

#[test]
fn references_into_other_documents_are_followed_and_the_period_chooses_by_day() {
    let Some(ch) = core_fixture("own/ch") else {
        return;
    };
    let whole = index(&ch);
    let selection = Selection {
        lines: vec!["ch:ex:Line:IR9".to_owned()],
        ..Selection::default()
    };
    let (part, report) = extracted(&ch, &selection);
    let read = index(part.path());
    assert_eq!(report.counts.documents_written, 2);
    assert!(
        read.unresolved().is_empty(),
        "{:?}",
        &read.unresolved()[..1]
    );
    let ids = reads_as_in_whole(&read, &whole);
    assert!(!ids.is_empty());
    // The line not chosen is not written.
    assert!(read.line_by_id("ch:ex:Line:S1").is_none());

    // A period the delivery does not run in chooses nothing.
    let (_, report) = extracted(
        &ch,
        &Selection {
            period: Some((Date::from("1990-01-01"), Date::from("1990-01-31"))),
            ..Selection::default()
        },
    );
    assert_eq!(report.counts.journeys_selected, 0);
}

#[test]
fn identifiers_of_one_document_are_resolved_in_it() {
    let Some(at) = core_fixture("own/at") else {
        return;
    };
    // Both documents define `at:ex:DayType:1`, for other days: the journeys of
    // line 200 keep their own document's.
    let whole = index(&at);
    let (part, _) = extracted(
        &at,
        &Selection {
            lines: vec!["at:ex:Line:200".to_owned()],
            ..Selection::default()
        },
    );
    let read = index(part.path());
    assert_eq!(reads_as_in_whole(&read, &whole).len(), 1);
}

#[test]
fn what_is_written_is_valid_against_the_schema_of_the_source_s_release() {
    let (Some(epip), Some(schema)) = (
        core_fixture("own/epip"),
        core_fixture("netex-2.0/xsd").map(|xsd| xsd.join("NeTEx_publication.xsd")),
    ) else {
        return;
    };
    if Command::new("xmllint").arg("--version").output().is_err() {
        eprintln!("SKIPPED: no xmllint");
        return;
    }
    let (part, _) = extracted(
        &epip,
        &Selection {
            area: Some(town_centre()),
            leaving: Leaving::Cut,
            ..Selection::default()
        },
    );
    let written = part.path().join("epip-line-12.xml");
    let output = Command::new("xmllint")
        .arg("--noout")
        .arg("--schema")
        .arg(&schema)
        .arg(&written)
        .output()
        .expect("xmllint runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    // The release is the source's.
    let text = std::fs::read_to_string(&written).expect("written");
    let source = std::fs::read_to_string(epip.join("epip-line-12.xml")).expect("the source");
    let root = |text: &str| text.lines().take(3).collect::<Vec<_>>().join("\n");
    assert_eq!(root(&text), root(&source));
}

#[test]
fn the_command_writes_the_part_and_its_report() {
    let Some(epip) = core_fixture("own/epip") else {
        return;
    };
    let to = tempfile::tempdir().expect("a directory");
    let part = to.path().join("part");
    let report = to.path().join("report.json");
    let output = Command::new(env!("CARGO_BIN_EXE_netex"))
        .arg("extract")
        .arg(&epip)
        .args([
            "--bbox",
            "9.73,52.37,9.74,52.38",
            "--leaving",
            "cut",
            "--to",
        ])
        .arg(&part)
        .arg("--report")
        .arg(&report)
        .output()
        .expect("the command runs");
    let printed = String::from_utf8(output.stdout).expect("UTF-8");
    assert_eq!(output.status.code(), Some(0), "{printed}");
    assert!(printed.contains("2 journeys"), "{printed}");
    assert!(printed.contains("journey-cut"), "{printed}");
    assert!(part.join("epip-line-12.xml").is_file());
    let report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(report).expect("the report")).expect("JSON");
    assert_eq!(report["counts"]["journeys_selected"], 2);
}

/// A part of a national delivery against the whole, both named by the
/// environment — the part written by `netex extract` beforehand:
///
/// ```sh
/// netex extract delfi.zip --bbox 9.35,52.2,10.15,52.65 --to region-hannover
/// NETEX_CLI_WHOLE=delfi.zip NETEX_CLI_PART=region-hannover NETEX_CLI_BOX=9.35,52.2,10.15,52.65 \
///   cargo test --release -p netex-cli --test extract -- --ignored --nocapture
/// ```
///
/// Every journey of the part reads as the journey of the same identifier and
/// document of the whole, and the part holds every journey of the whole
/// calling in the box.
#[test]
#[ignore = "needs a national delivery and a part of it: NETEX_CLI_WHOLE, NETEX_CLI_PART, NETEX_CLI_BOX"]
fn a_region_of_a_national_delivery_reads_as_its_journeys_of_the_whole() {
    let (Some(whole), Some(part), Some(area)) = (
        std::env::var_os("NETEX_CLI_WHOLE"),
        std::env::var_os("NETEX_CLI_PART"),
        std::env::var("NETEX_CLI_BOX").ok(),
    ) else {
        eprintln!("SKIPPED: NETEX_CLI_WHOLE, NETEX_CLI_PART and NETEX_CLI_BOX name no delivery");
        return;
    };
    let area = Area::from_box(&area).expect("a box");
    let started = std::time::Instant::now();
    let whole = index(Path::new(&whole));
    let read = index(Path::new(&part));
    eprintln!("read both in {:.0?}", started.elapsed());
    let calling = (0..whole.journey_count() as u32)
        .map(JourneyId)
        .filter(|&journey| {
            whole.journey(journey).stop_points.iter().any(|&point| {
                let stop = whole.stop_of_stop_point(point).map(|stop| whole.stop(stop));
                let record = whole.stop_point(point);
                stop.and_then(|stop| stop.longitude.zip(stop.latitude))
                    .or(record.longitude.zip(record.latitude))
                    .is_some_and(|(longitude, latitude)| area.contains(longitude, latitude))
            })
        })
        .count();
    let ids = reads_as_in_whole(&read, &whole);
    eprintln!(
        "whole: {} journeys, {} calling in the box; part: {} journeys, each read as in the whole; \
         unresolved references: whole {}, part {}; interchanges: whole {}, part {}",
        whole.journey_count(),
        calling,
        ids.len(),
        whole.unresolved().len(),
        read.unresolved().len(),
        whole.interchange_count(),
        read.interchange_count()
    );
    assert_eq!(ids.len(), calling);
}
