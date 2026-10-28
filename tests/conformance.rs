//! Conformance harness.
//!
//! For every official NeTEx example document of both schema releases this
//! * reads it into the crate's types,
//! * writes those types back out,
//! * compares the result to the original element by element, and
//! * validates the result against the official schemas of its release.
//!
//! Then it crosses the releases: a document of the one release is written for
//! the other, what it loses is reported, and the result is validated against
//! the other release's schemas. The `should-fail` examples must be rejected.
//!
//! A document that loses content, invents content, reorders content or fails
//! validation fails the suite. That is what this crate means by "conformant".
//!
//! The official schemas and examples are part of the repository under their
//! own licence (GPL-3.0), not of the crate's package: a test that needs them
//! reads them where they are and is skipped, saying so, where they are not.

mod support;

use std::collections::BTreeMap;

use netex_rs::version::detect;
use netex_rs::{Document, Fragment, Release, Written};
use support::{
    compare, parse, report, validate_all, validate_all_batched, Candidate, Fixture,
    VALIDATOR_MISSING,
};

/// How a fixture is read: as a typed publication, or as a fragment rooted at
/// another NeTEx element and carried through as it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reading {
    Publication,
    Fragment,
}

/// Reads a fixture and writes it straight back out, as it is.
fn round_trip(fixture: &Fixture) -> Result<(Reading, String), String> {
    if fixture.root() == "PublicationDelivery" {
        let document = netex_rs::read(&fixture.xml).map_err(|e| e.to_string())?;
        Ok((Reading::Publication, document.write()))
    } else {
        let fragment = Fragment::read(&fixture.xml).map_err(|e| e.to_string())?;
        Ok((Reading::Fragment, fragment.write()))
    }
}

/// Reads a fixture and writes it for `release`.
fn rewrite_for(fixture: &Fixture, release: Release) -> Result<Written, String> {
    if fixture.root() == "PublicationDelivery" {
        let document: Document = netex_rs::read(&fixture.xml).map_err(|e| e.to_string())?;
        Ok(document.write_for(release))
    } else {
        let fragment = Fragment::read(&fixture.xml).map_err(|e| e.to_string())?;
        Ok(fragment.write_for(release))
    }
}

/// A report's losses on one line.
fn losses(report: &netex_rs::Report) -> String {
    report
        .losses()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("; ")
}

/// The NeTEx fixtures of a release, with the others listed as skipped.
fn netex_fixtures(release: Release) -> Vec<Fixture> {
    let mut kept = Vec::new();
    let mut skipped = Vec::new();
    for fixture in support::fixtures(release) {
        if fixture.is_netex() {
            kept.push(fixture);
        } else {
            skipped.push(format!("{} (root <{}>)", fixture.name, fixture.root()));
        }
    }
    println!(
        "{}: {} NeTEx documents; {} documents in other namespaces skipped:\n  {}",
        release.label(),
        kept.len(),
        skipped.len(),
        skipped.join("\n  ")
    );
    kept
}

fn round_trip_differences(release: Release) -> Vec<String> {
    let mut failures = Vec::new();
    let mut typed = 0;
    let mut verbatim = 0;
    for fixture in netex_fixtures(release) {
        let (reading, written) = match round_trip(&fixture) {
            Ok(written) => written,
            Err(complaint) => {
                failures.push(format!("{}: cannot round-trip: {complaint}", fixture.name));
                continue;
            }
        };
        match reading {
            Reading::Publication => typed += 1,
            Reading::Fragment => verbatim += 1,
        }
        if let Err(difference) = compare(&parse(&fixture.xml), &parse(&written)) {
            failures.push(format!("{}: {difference}", fixture.name));
        }
    }
    println!(
        "{}: {typed} publications read into the typed model, {verbatim} fragment documents carried through verbatim",
        release.label()
    );
    failures
}

#[test]
fn every_official_1x_example_round_trips_without_loss() {
    if !support::official_fixtures(Release::V1) {
        return;
    }
    let failures = round_trip_differences(Release::V1);
    assert!(
        failures.is_empty(),
        "{}",
        report("1.3 examples did not round-trip", &failures)
    );
}

#[test]
fn every_official_20_example_round_trips_without_loss() {
    if !support::official_fixtures(Release::V2) {
        return;
    }
    let failures = round_trip_differences(Release::V2);
    assert!(
        failures.is_empty(),
        "{}",
        report("2.0 examples did not round-trip", &failures)
    );
}

/// The fixtures of a release whose originals the validator accepts, and the
/// complaints about the others.
fn valid_originals(release: Release) -> (Vec<Fixture>, BTreeMap<String, String>) {
    let fixtures = netex_fixtures(release);
    let candidates: Vec<Candidate> = fixtures
        .iter()
        .map(|f| Candidate {
            name: f.name.clone(),
            xml: f.xml.clone(),
        })
        .collect();
    let complaints = validate_all(release, &candidates);
    let valid = fixtures
        .into_iter()
        .filter(|f| !complaints.contains_key(&f.name))
        .collect();
    (valid, complaints)
}

fn written_documents_the_validator_rejects(release: Release) -> Vec<String> {
    let (valid, invalid_originals) = valid_originals(release);
    println!(
        "{}: {} originals validate; {} do not and are not expected to after writing:\n  {}",
        release.label(),
        valid.len(),
        invalid_originals.len(),
        invalid_originals
            .iter()
            .map(|(name, complaint)| format!("{name}: {}", complaint.lines().next().unwrap_or("")))
            .collect::<Vec<_>>()
            .join("\n  ")
    );
    let mut failures = Vec::new();
    let mut candidates = Vec::new();
    for fixture in &valid {
        match round_trip(fixture) {
            Ok((_, written)) => candidates.push(Candidate {
                name: fixture.name.clone(),
                xml: written,
            }),
            Err(complaint) => {
                failures.push(format!("{}: cannot round-trip: {complaint}", fixture.name))
            }
        }
    }
    for (name, complaint) in validate_all(release, &candidates) {
        failures.push(format!("{name}:\n{complaint}"));
    }
    println!(
        "{}: {} of {} written documents validate",
        release.label(),
        candidates.len() - failures.len(),
        candidates.len()
    );
    failures
}

#[test]
fn every_written_1x_example_validates_against_the_1x_schemas() {
    if !support::official_fixtures(Release::V1) {
        return;
    }
    assert!(support::validator_available(), "{VALIDATOR_MISSING}");
    let failures = written_documents_the_validator_rejects(Release::V1);
    assert!(
        failures.is_empty(),
        "{}",
        report("written 1.3 documents were rejected", &failures)
    );
}

#[test]
fn every_written_20_example_validates_against_the_20_schemas() {
    if !support::official_fixtures(Release::V2) {
        return;
    }
    assert!(support::validator_available(), "{VALIDATOR_MISSING}");
    let failures = written_documents_the_validator_rejects(Release::V2);
    assert!(
        failures.is_empty(),
        "{}",
        report("written 2.0 documents were rejected", &failures)
    );
}

/// The five DELFI line files: the profile the crate is built for.
///
/// The delivery is not schema-valid as published (a `PassengerCapacity` has no
/// `id`), so what is asserted is that writing adds no defect: the validator's
/// complaints about the written document are the complaints about the original.
/// The round trip is checked always, the validator's complaints where the 1.x
/// schemas are there.
#[test]
fn every_delfi_fixture_round_trips_and_adds_no_defect() {
    let fixtures = support::delfi_fixtures();
    let mut failures = Vec::new();
    let mut originals = Vec::new();
    let mut candidates = Vec::new();
    for fixture in &fixtures {
        originals.push(Candidate {
            name: fixture.name.clone(),
            xml: fixture.xml.clone(),
        });
        match round_trip(fixture) {
            Ok((Reading::Publication, written)) => {
                if let Err(difference) = compare(&parse(&fixture.xml), &parse(&written)) {
                    failures.push(format!("{}: {difference}", fixture.name));
                }
                candidates.push(Candidate {
                    name: fixture.name.clone(),
                    xml: written,
                });
            }
            Ok((Reading::Fragment, _)) => {
                failures.push(format!("{}: not read as a publication", fixture.name))
            }
            Err(complaint) => {
                failures.push(format!("{}: cannot round-trip: {complaint}", fixture.name))
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{}",
        report("DELFI fixtures did not round-trip", &failures)
    );
    if !support::official_fixtures(Release::V1) {
        return;
    }
    assert!(support::validator_available(), "{VALIDATOR_MISSING}");
    let before = validate_all(Release::V1, &originals);
    let after = validate_all(Release::V1, &candidates);
    for fixture in &fixtures {
        let defects = |complaints: &BTreeMap<String, String>| -> Vec<String> {
            complaints
                .get(&fixture.name)
                .map(|c| {
                    c.lines()
                        .filter(|l| l.contains("validity error"))
                        .map(strip_line_number)
                        .collect()
                })
                .unwrap_or_default()
        };
        let (before, after) = (defects(&before), defects(&after));
        println!(
            "{}: {} defect(s) in the original, {} after writing",
            fixture.name,
            before.len(),
            after.len()
        );
        if before != after {
            failures.push(format!(
                "{}: writing changed the validator's complaints:\n  before: {:?}\n  after:  {:?}",
                fixture.name, before, after
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{}",
        report("writing DELFI fixtures added defects", &failures)
    );
}

/// A validator complaint without the line number in front, which writing changes.
fn strip_line_number(line: &str) -> String {
    match line.split_once(": element ") {
        Some((_, rest)) => format!("element {rest}"),
        None => line.to_owned(),
    }
}

/// Writes every valid original of `from` for the release `to`: a same-release
/// write must lose nothing, and a cross-release write must lose exactly what
/// the other release does not know, and validate there once it has.
fn cross_write(from: Release, to: Release) -> Vec<String> {
    let (valid, _) = valid_originals(from);
    let mut failures = Vec::new();
    let mut lossless = Vec::new();
    let mut lossy = Vec::new();
    let mut candidates = Vec::new();
    for fixture in &valid {
        let written = match rewrite_for(fixture, to) {
            Ok(written) => written,
            Err(complaint) => {
                failures.push(format!(
                    "{}: cannot write for {}: {complaint}",
                    fixture.name,
                    to.label()
                ));
                continue;
            }
        };
        if written.report.is_lossless() {
            lossless.push(fixture.name.clone());
        } else {
            lossy.push(format!("{}: {}", fixture.name, losses(&written.report)));
        }
        if from == to && !written.report.is_lossless() {
            failures.push(format!(
                "{}: validates against the {} schemas, yet writing it for {} reports losses: {}",
                fixture.name,
                from.label(),
                to.label(),
                losses(&written.report)
            ));
        }
        candidates.push(Candidate {
            name: fixture.name.clone(),
            xml: written.xml,
        });
    }
    // A same-release write is asserted valid, so its complaints are re-checked
    // singly; the cross-release count is reported, batch-validated, and may
    // include an identity-constraint complaint a neighbour caused.
    let complaints = if from == to {
        validate_all(to, &candidates)
    } else {
        validate_all_batched(to, &candidates)
    };
    println!(
        "{} -> {}: {} documents written, {} lossless, {} with losses; {} validate against the {} schemas{}",
        from.label(),
        to.label(),
        candidates.len(),
        lossless.len(),
        lossy.len(),
        candidates.len() - complaints.len(),
        to.label(),
        if from == to { "" } else { " (batch-validated)" }
    );
    if !lossy.is_empty() {
        println!("losses reported:\n  {}", lossy.join("\n  "));
    }
    if !complaints.is_empty() {
        println!(
            "rejected by the {} schemas:\n  {}",
            to.label(),
            complaints
                .iter()
                .map(|(name, complaint)| format!(
                    "{name}: {}",
                    complaint.lines().next().unwrap_or("")
                ))
                .collect::<Vec<_>>()
                .join("\n  ")
        );
    }
    if from == to {
        for (name, complaint) in complaints {
            failures.push(format!("{name}:\n{complaint}"));
        }
    }
    failures
}

#[test]
fn writing_1x_examples_for_1x_loses_nothing_and_validates() {
    if !support::official_fixtures(Release::V1) {
        return;
    }
    assert!(support::validator_available(), "{VALIDATOR_MISSING}");
    let failures = cross_write(Release::V1, Release::V1);
    assert!(failures.is_empty(), "{}", report("1.3 -> 1.3", &failures));
}

#[test]
fn writing_20_examples_for_20_loses_nothing_and_validates() {
    if !support::official_fixtures(Release::V2) {
        return;
    }
    assert!(support::validator_available(), "{VALIDATOR_MISSING}");
    let failures = cross_write(Release::V2, Release::V2);
    assert!(failures.is_empty(), "{}", report("2.0 -> 2.0", &failures));
}

#[test]
fn writing_1x_examples_for_20_reports_every_loss() {
    if !(support::official_fixtures(Release::V1) && support::official_fixtures(Release::V2)) {
        return;
    }
    assert!(support::validator_available(), "{VALIDATOR_MISSING}");
    let failures = cross_write(Release::V1, Release::V2);
    assert!(failures.is_empty(), "{}", report("1.3 -> 2.0", &failures));
}

#[test]
fn writing_20_examples_for_1x_reports_every_loss() {
    if !(support::official_fixtures(Release::V1) && support::official_fixtures(Release::V2)) {
        return;
    }
    assert!(support::validator_available(), "{VALIDATOR_MISSING}");
    let failures = cross_write(Release::V2, Release::V1);
    assert!(failures.is_empty(), "{}", report("2.0 -> 1.3", &failures));
}

/// A document the schemas of its release accept never carries structural
/// evidence for the other release.
fn detection_contradictions(release: Release) -> Vec<String> {
    let other = match release {
        Release::V1 => Release::V2,
        Release::V2 => Release::V1,
    };
    let (valid, _) = valid_originals(release);
    let mut failures = Vec::new();
    let mut declared = BTreeMap::new();
    let mut structural = 0;
    for fixture in &valid {
        let detection = detect(&fixture.xml);
        *declared
            .entry(detection.declared_version.clone().unwrap_or_default())
            .or_insert(0) += 1;
        if detection.structural == Some(release) {
            structural += 1;
        }
        if detection.structural == Some(other) {
            failures.push(format!(
                "{}: validates against the {} schemas, yet is detected as {} because of {}",
                fixture.name,
                release.label(),
                other.label(),
                detection
                    .evidence
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    println!(
        "{}: {} valid documents; {structural} carry structural evidence of their release; declared versions: {declared:?}",
        release.label(),
        valid.len()
    );
    failures
}

#[test]
fn release_detection_never_contradicts_the_1x_schemas() {
    if !support::official_fixtures(Release::V1) {
        return;
    }
    assert!(support::validator_available(), "{VALIDATOR_MISSING}");
    let failures = detection_contradictions(Release::V1);
    assert!(
        failures.is_empty(),
        "{}",
        report("1.3 documents detected as 2.0", &failures)
    );
}

#[test]
fn release_detection_never_contradicts_the_20_schemas() {
    if !support::official_fixtures(Release::V2) {
        return;
    }
    assert!(support::validator_available(), "{VALIDATOR_MISSING}");
    let failures = detection_contradictions(Release::V2);
    assert!(
        failures.is_empty(),
        "{}",
        report("2.0 documents detected as 1.3", &failures)
    );
}

#[test]
fn the_delfi_delivery_is_detected_as_1x() {
    for fixture in support::delfi_fixtures() {
        let detection = detect(&fixture.xml);
        assert_eq!(
            detection.declared_version.as_deref(),
            Some("ntx:1.1"),
            "{}",
            fixture.name
        );
        assert_eq!(detection.declared, Some(Release::V1), "{}", fixture.name);
        assert_ne!(
            detection.structural,
            Some(Release::V2),
            "{}: {:?}",
            fixture.name,
            detection.evidence
        );
        assert_eq!(detection.release(), Some(Release::V1), "{}", fixture.name);
    }
}

/// Only a `PublicationDelivery` root carries the schema version; a document
/// rooted at another element keeps its root untouched when rewritten.
#[test]
fn rewriting_a_fragment_document_leaves_its_root_alone() {
    let fragment = Fragment::read(
        "<Locale xmlns=\"http://www.netex.org.uk/netex\"><TimeZone>Europe/Oslo</TimeZone></Locale>",
    )
    .expect("a fragment reads");
    let written = fragment.write_for(Release::V2);
    assert!(written.report.is_lossless());
    assert!(
        written
            .xml
            .contains("<Locale xmlns=\"http://www.netex.org.uk/netex\">"),
        "{}",
        written.xml
    );
    let publication = netex_rs::read("<PublicationDelivery xmlns=\"http://www.netex.org.uk/netex\"><ParticipantRef>X</ParticipantRef></PublicationDelivery>")
        .expect("a publication reads");
    assert!(publication
        .write_for(Release::V2)
        .xml
        .contains("version=\"2.0\""));
    assert!(publication
        .write_for(Release::V1)
        .xml
        .contains("version=\"1.1\""));
}

#[test]
fn should_fail_examples_are_rejected_by_the_validator() {
    if !support::official_fixtures(Release::V2) {
        return;
    }
    assert!(support::validator_available(), "{VALIDATOR_MISSING}");
    let fixtures = support::should_fail_fixtures();
    let candidates: Vec<Candidate> = fixtures
        .iter()
        .map(|f| Candidate {
            name: f.name.clone(),
            xml: f.xml.clone(),
        })
        .collect();
    let complaints = validate_all(Release::V2, &candidates);
    let accepted: Vec<String> = fixtures
        .iter()
        .filter(|f| !complaints.contains_key(&f.name))
        .map(|f| f.name.clone())
        .collect();
    assert!(
        accepted.is_empty(),
        "{}",
        report("should-fail documents were accepted", &accepted)
    );
    for (name, complaint) in &complaints {
        assert!(
            complaint.contains("Duplicate key-sequence"),
            "{name}: rejected for another reason than the duplicate key:\n{complaint}"
        );
    }
}

/// Each `should-fail` example repeats an identifier at one version inside one
/// frame: a key the schema enforces, and a check the crate makes on its own.
#[test]
fn should_fail_examples_are_found_by_the_library() {
    if !support::official_fixtures(Release::V2) {
        return;
    }
    let mut failures = Vec::new();
    for fixture in support::should_fail_fixtures() {
        let document = match netex_rs::read(&fixture.xml) {
            Ok(document) => document,
            Err(error) => {
                failures.push(format!("{}: cannot read: {error}", fixture.name));
                continue;
            }
        };
        let findings = netex_rs::check::duplicate_ids(&document);
        if findings.is_empty() {
            failures.push(format!("{}: no duplicate identifier found", fixture.name));
        } else {
            println!("{}: {}", fixture.name, findings[0]);
        }
    }
    assert!(
        failures.is_empty(),
        "{}",
        report(
            "should-fail documents passed the library's check",
            &failures
        )
    );
}
