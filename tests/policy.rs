//! Refusing or tolerating what a delivery states: a policy over the counts of
//! the index's findings, fatal by choice; the axis order of a link's
//! geometry; the references that resolve only once their version or class
//! is ignored; and what the repository carries.

mod support;

use std::fs;
use std::path::{Path, PathBuf};

use netex_rs::index::{Concern, Index, IndexBuilder, Policy};
use netex_rs::stream::{Kind, ReadOptions, Selection};
use support::timetable::{line_document, serially, two_calendars, Point, Timetable};

fn findings(index: &Index, concern: Concern) -> Vec<String> {
    index
        .findings()
        .filter(|finding| finding.concern == concern)
        .map(|finding| finding.to_string())
        .collect()
}

#[test]
fn a_policy_tolerates_what_it_does_not_refuse_and_counts_it_either_way() {
    let index = serially(&two_calendars());
    assert_eq!(index.finding_count(Concern::RedefinedOtherwise), 2);
    assert_eq!(Policy::tolerant().check(&index), Ok(()));
    assert_eq!(
        Policy::tolerant()
            .at_most(Concern::RedefinedOtherwise, 2)
            .check(&index),
        Ok(()),
        "as many as it tolerates"
    );
    let refused = Policy::tolerant()
        .refuse(Concern::Unresolved)
        .at_most(Concern::RedefinedOtherwise, 1)
        .check(&index)
        .expect_err("one more than it tolerates");
    assert_eq!(refused.exceeded.len(), 1, "no reference is unresolved");
    let exceeded = refused.exceeded[0];
    assert_eq!(
        (exceeded.concern, exceeded.count, exceeded.limit),
        (Concern::RedefinedOtherwise, 2, 1)
    );
    assert_eq!(
        refused.to_string(),
        "the delivery is refused: 2 findings `redefined with other content`, at most 1 tolerated"
    );
}

#[test]
fn a_policy_refusing_references_to_nothing_refuses_on_the_unresolved_list() {
    let document = Timetable::new()
        .line("L", "1", None)
        .stop_point("A")
        .pattern(
            "P",
            "L",
            &[Point::at("P1", "A"), Point::at("P2", "nowhere")],
        )
        .xml();
    let index = serially(&[("A.xml".to_owned(), document)]);
    assert_eq!(index.finding_count(Concern::Unresolved), 1);
    let refused = Policy::tolerant()
        .refuse(Concern::Unresolved)
        .check(&index)
        .expect_err("a reference to nothing");
    assert_eq!(refused.exceeded[0].count, 1);
}

/// Links between two stop points in the Harz
/// (`tests/fixtures/fragments/frames/axis-order.xml`): one written longitude
/// first, one latitude first, one far away either way, one from a stop point
/// no document defines — its order cannot be told against nothing —, and one
/// without geometry, which is not judged.
#[test]
fn a_link_written_longitude_first_is_a_finding_and_one_that_cannot_be_judged_another() {
    let name = "axis-order.xml";
    let document = support::fragments::document(name);
    let options = ReadOptions::new(Selection::routing().with(Kind::ServiceLink));
    let mut lean = IndexBuilder::new();
    lean.add_streamed(&document, &options, name)
        .expect("the fragment reads");
    let mut full = IndexBuilder::new();
    full.add(
        &netex_rs::read(&document).expect("the fragment reads"),
        name,
    );
    for index in [lean.finish(), full.finish()] {
        assert_eq!(
            findings(&index, Concern::AxisOrderSwapped),
            ["link geometry written longitude first: ServiceLink ex:ServiceLink:longitude-first in axis-order.xml"]
        );
        assert_eq!(
            findings(&index, Concern::AxisOrderUndecided),
            [
                "link geometry whose axis order cannot be told: ServiceLink ex:ServiceLink:far in axis-order.xml",
                "link geometry whose axis order cannot be told: ServiceLink ex:ServiceLink:unanchored in axis-order.xml",
            ]
        );
    }
}

#[test]
fn the_axis_order_of_a_position_is_judged_by_the_start_within_150_metres() {
    use netex_rs::geometry::{axis_order, AxisOrder};
    let klagenfurt = (46.6167, 14.3100);
    assert_eq!(
        axis_order((46.6168, 14.3101), klagenfurt),
        AxisOrder::LatitudeFirst
    );
    assert_eq!(
        axis_order((14.3101, 46.6168), klagenfurt),
        AxisOrder::LongitudeFirst
    );
    assert_eq!(
        axis_order((46.6200, 14.3100), klagenfurt),
        AxisOrder::Undecided,
        "370 m away"
    );
}

#[test]
fn a_reference_resolved_only_by_ignoring_its_version_or_class_is_counted() {
    // Every day type assignment of the Austrian and German deliveries refers
    // to a UicOperatingPeriod through an OperatingPeriodRef.
    let document = line_document("J", "1111111", "08:00:00").replace(
        r#"<LineRef ref="L" version="1"/>"#,
        r#"<LineRef ref="L" version="7"/>"#,
    );
    let index = serially(&[("A.xml".to_owned(), document)]);
    assert_eq!(
        findings(&index, Concern::ResolvedAsOtherClass),
        ["reference resolved to another class than it names: OperatingPeriodRef OP in A.xml (UicOperatingPeriod)"]
    );
    assert_eq!(
        findings(&index, Concern::ResolvedIgnoringVersion),
        ["reference resolved only by ignoring the version it names: LineRef L in A.xml (7)"]
    );
    assert!(index.journey_by_id("J").is_some(), "it still resolves");
}

#[test]
fn every_concern_has_a_name_of_its_own_for_reports_and_command_lines() {
    let names: Vec<&str> = Concern::ALL.iter().map(|concern| concern.code()).collect();
    let mut distinct = names.clone();
    distinct.sort_unstable();
    distinct.dedup();
    assert_eq!(
        distinct.len(),
        names.len(),
        "no two concerns share a name: {names:?}"
    );
    for (&concern, name) in Concern::ALL.iter().zip(&names) {
        assert!(
            name.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
            "{name} is written in lower case with hyphens"
        );
        assert_eq!(Concern::from_code(name), Some(concern));
    }
    assert_eq!(Concern::Unresolved.code(), "unresolved");
    assert_eq!(Concern::RedefinedOtherwise.code(), "redefined-otherwise");
    assert_eq!(Concern::from_code("no-such-concern"), None);
}

/// Names no file of the repository may carry, in its path or its content,
/// compared without regard to case: the crate's tests, tools and documents
/// stand on their own and on the standard. They are written in parts here so
/// that this file does not carry them itself.
const NAMES_NOT_CARRIED: [&str; 3] = [
    concat!("tool", "kit"),
    concat!("flak", "tack"),
    concat!("frem", "d/"),
];

/// What stands before the first name where it is part of another: the XMP
/// metadata of a PDF names the Adobe library that wrote it so, and the
/// official examples carry one such PDF verbatim. Only a PDF is spared.
const XMP_BEFORE: &[u8] = b"xmp ";

/// Every file under `directory`, skipping build output and the repository's
/// own store, and not following symbolic links.
fn files_under(directory: &Path, files: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(directory)
        .unwrap_or_else(|e| panic!("{}: {e}", directory.display()))
        .map(|entry| entry.expect("a directory entry"))
        .collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let kind = entry.file_type().expect("a file type");
        let path = entry.path();
        if kind.is_dir() {
            if !matches!(entry.file_name().to_str(), Some("target" | ".git")) {
                files_under(&path, files);
            }
        } else if kind.is_file() {
            files.push(path);
        }
    }
}

/// Each name not carried that `text` holds, without regard to case, with the
/// line it stands on; a path is one line. `pdf` says whether the text is a
/// PDF's, whose XMP metadata is spared.
fn names_carried(text: &[u8], pdf: bool) -> Vec<(&'static str, usize)> {
    let text = text.to_ascii_lowercase();
    let mut found = Vec::new();
    for name in NAMES_NOT_CARRIED {
        for at in memchr::memmem::find_iter(&text, name) {
            if pdf && name == NAMES_NOT_CARRIED[0] && text[..at].ends_with(XMP_BEFORE) {
                continue;
            }
            found.push((name, memchr::memchr_iter(b'\n', &text[..at]).count() + 1));
        }
    }
    found.sort_by_key(|&(_, line)| line);
    found
}

#[test]
fn the_names_not_carried_match_without_regard_to_case_and_a_pdf_keeps_its_metadata() {
    let upper = NAMES_NOT_CARRIED[0].to_uppercase();
    assert_eq!(
        names_carried(format!("first\nsecond {upper}").as_bytes(), false),
        [(NAMES_NOT_CARRIED[0], 2)]
    );
    assert_eq!(
        names_carried(
            format!("tests/{}x.xml", NAMES_NOT_CARRIED[2]).as_bytes(),
            false
        ),
        [(NAMES_NOT_CARRIED[2], 1)]
    );
    let metadata = format!("x:xmptk='XMP {upper}'");
    assert_eq!(names_carried(metadata.as_bytes(), true), []);
    assert_eq!(
        names_carried(metadata.as_bytes(), false),
        [(NAMES_NOT_CARRIED[0], 1)],
        "only a PDF is spared"
    );
    assert_eq!(
        names_carried(format!("a {upper}, and XMP {upper}").as_bytes(), true),
        [(NAMES_NOT_CARRIED[0], 1)]
    );
    assert_eq!(names_carried(b"a fremde Datei, a tool, a kit", false), []);
}

#[test]
fn no_file_of_the_repository_names_what_it_does_not_carry() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    files_under(root, &mut files);
    assert!(
        files.iter().any(|file| file.ends_with("tests/policy.rs")),
        "the walk reaches the tests"
    );
    let mut carried = Vec::new();
    for file in &files {
        let relative = file.strip_prefix(root).expect("under the root");
        let path = relative.to_string_lossy();
        for (name, _) in names_carried(path.as_bytes(), false) {
            carried.push(format!("{path}: `{name}` in its path"));
        }
        let content = fs::read(file).unwrap_or_else(|e| panic!("{path}: {e}"));
        let pdf = relative
            .extension()
            .is_some_and(|extension| extension == "pdf");
        for (name, line) in names_carried(&content, pdf) {
            carried.push(format!("{path}:{line}: `{name}`"));
        }
    }
    assert!(carried.is_empty(), "{}", carried.join("\n"));
}
