//! The crate against an independent Python extractor, over the whole delivery.
//!
//! `extract_netex.py`, a script outside this repository, lifts three things out
//! of the DELFI NeTEx: measured walks, accessibility facts, and the level of
//! each quay. Its rules are ported in `support::delfi`; this runs them over
//! every line file through the typed model and compares the result, row for
//! row, to the TSVs the Python extractor wrote from the same delivery. Both
//! inputs come from outside the repository, named by `NETEX_DELFI_ZIP` and
//! `NETEX_PY_EXTRACT_DIR`; the test is skipped — visibly — when either is not
//! set.

mod support;

use std::path::{Path, PathBuf};
use std::time::Instant;

use netex_rs::dataset::Source;
use netex_rs::Limits;
use rayon::prelude::*;
use support::delfi::{classify_notice, extract, Extract, Fact};

/// The delivery and the Python extract, if both are there — and if this is a
/// release build, for the same reason as the corpus test.
fn inputs() -> Option<(Source, PathBuf)> {
    if cfg!(debug_assertions) && std::env::var_os("NETEX_CORPUS_DEBUG").is_none() {
        println!("SKIPPED: debug build; run with --release (or set NETEX_CORPUS_DEBUG)");
        return None;
    }
    let (Some(zip), Some(dir)) = (
        std::env::var_os("NETEX_DELFI_ZIP"),
        std::env::var_os("NETEX_PY_EXTRACT_DIR"),
    ) else {
        println!(
            "SKIPPED: NETEX_DELFI_ZIP and NETEX_PY_EXTRACT_DIR do not both name the delivery and its Python extract"
        );
        return None;
    };
    let dir = PathBuf::from(dir);
    assert!(
        Path::new(&zip).exists(),
        "NETEX_DELFI_ZIP names {}, which is not there",
        Path::new(&zip).display()
    );
    assert!(
        dir.join("footpaths.tsv").exists(),
        "NETEX_PY_EXTRACT_DIR names {}, which holds no footpaths.tsv",
        dir.display()
    );
    Some((Source::open(&zip).expect("the delivery opens"), dir))
}

fn rows_of(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
        .lines()
        .skip(1)
        .map(str::to_owned)
        .collect()
}

fn differences(what: &str, ours: &[String], theirs: &[String]) -> Vec<String> {
    let ours_set: std::collections::BTreeSet<&String> = ours.iter().collect();
    let theirs_set: std::collections::BTreeSet<&String> = theirs.iter().collect();
    let mut out = Vec::new();
    for row in theirs_set.difference(&ours_set).take(20) {
        out.push(format!("{what}: only Python: {row}"));
    }
    for row in ours_set.difference(&theirs_set).take(20) {
        out.push(format!("{what}: only netex-rs: {row}"));
    }
    if ours.len() != theirs.len() {
        out.push(format!(
            "{what}: {} rows from netex-rs, {} from Python",
            ours.len(),
            theirs.len()
        ));
    }
    out
}

#[test]
fn footpaths_and_accessibility_match_the_python_extractor_row_for_row() {
    let Some((source, dir)) = inputs() else {
        return;
    };
    let limits = Limits::default();
    let start = Instant::now();
    let extract = source
        .documents()
        .expect("the delivery lists its documents")
        .map(|loaded| match loaded.read(&limits) {
            Ok(document) => extract(&document),
            Err(error) => panic!("{}: {error}", loaded.entry.name),
        })
        .reduce(Extract::default, Extract::merge);
    println!(
        "extracted in {:.1} s: {} footpaths, {} accessibility facts, {} quay levels; unresolved {}, self-loops {}, unassigned notice points {}",
        start.elapsed().as_secs_f64(),
        extract.footpaths.len(),
        extract.accessibility.len(),
        extract.quay_levels.len(),
        extract.footpaths_unresolved,
        extract.footpaths_self,
        extract.notice_points_unassigned
    );

    let footpaths: Vec<String> = extract
        .footpaths
        .iter()
        .map(|(a, b, seconds, both)| {
            format!("{a}\t{b}\t{seconds}\t{}", if *both { "1" } else { "0" })
        })
        .collect();
    let accessibility: Vec<String> = extract
        .accessibility
        .iter()
        .map(|(dhid, axis, cmp, value, unit, code, text)| {
            format!("{dhid}\t{axis}\t{cmp}\t{value}\t{unit}\t{code}\t{text}")
        })
        .collect();
    let mut failures = differences(
        "footpaths.tsv",
        &footpaths,
        &rows_of(&dir.join("footpaths.tsv")),
    );
    failures.extend(differences(
        "stop_accessibility.tsv",
        &accessibility,
        &rows_of(&dir.join("stop_accessibility.tsv")),
    ));
    // The register's level column is the only NeTEx-derived column of the third TSV.
    let register = dir.join("zhv_stop_register.tsv");
    if register.exists() {
        let mut levelled = 0usize;
        let mut mismatched = Vec::new();
        for line in std::fs::read_to_string(&register)
            .expect("the register reads")
            .lines()
            .skip(1)
        {
            let fields: Vec<&str> = line.split('\t').collect();
            let (dhid, level) = (fields[0], fields.get(6).copied().unwrap_or(""));
            if !level.is_empty() {
                levelled += 1;
            }
            let ours = extract.quay_levels.get(dhid).map_or("", String::as_str);
            if ours != level && mismatched.len() < 20 {
                mismatched.push(format!("zhv_stop_register.tsv: {dhid}: level {level:?} from Python, {ours:?} from netex-rs"));
            }
        }
        println!(
            "{levelled} register rows carry a level; {} quay levels from netex-rs",
            extract.quay_levels.len()
        );
        failures.extend(mismatched);
    }
    let summary = std::fs::read_to_string(dir.join("extract.json")).unwrap_or_default();
    println!("Python summary: {}", summary.trim());
    assert!(
        failures.is_empty(),
        "{}",
        support::report("differences from the Python extractor", &failures)
    );
}

/// The ported notice classifier on the wordings the Python tests use.
#[test]
fn notice_classification_matches_the_python_rules() {
    let cases: &[(&str, Option<Fact>)] = &[
        (
            "Türbreite des Fahrzeugs in mm : 1200",
            Some(Fact {
                axis: "door_width",
                comparator: "eq",
                value: "1200".into(),
                unit: "mm".into(),
            }),
        ),
        (
            "Höhe des Fahrzeugbodens: 99 cm",
            Some(Fact {
                axis: "floor_height",
                comparator: "eq",
                value: "99".into(),
                unit: "cm".into(),
            }),
        ),
        ("Türbreite 59-62 cm", None),
        ("Rampe vorhanden", None),
        (
            "keine Stufen am Fahrzeug",
            Some(Fact {
                axis: "step_count",
                comparator: "eq",
                value: "0".into(),
                unit: String::new(),
            }),
        ),
        (
            "2 Stufen im Fahrzeug",
            Some(Fact {
                axis: "step_count",
                comparator: "eq",
                value: "2".into(),
                unit: String::new(),
            }),
        ),
        ("Hublift mit mindestens 300 kg Tragfähigkeit", None),
        (
            "Rampe: Tragfähigkeit der Rampe mindestens 300 kg",
            Some(Fact {
                axis: "ramp_capacity",
                comparator: "ge",
                value: "300".into(),
                unit: "kg".into(),
            }),
        ),
        (
            "Rampenneigung maximal 12 %",
            Some(Fact {
                axis: "ramp_slope",
                comparator: "le",
                value: "12".into(),
                unit: "%".into(),
            }),
        ),
        (
            "Spalt zwischen Bahnsteig und Fahrzeug ca. 5,5 cm",
            Some(Fact {
                axis: "gap_width",
                comparator: "approx",
                value: "5.5".into(),
                unit: "cm".into(),
            }),
        ),
        ("Zahl der Stehplätze : 0", None),
    ];
    for (text, expected) in cases {
        assert_eq!(&classify_notice(text), expected, "{text:?}");
    }
}
