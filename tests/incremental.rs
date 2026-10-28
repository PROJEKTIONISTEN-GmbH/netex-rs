//! Reading only what changed: a fingerprint per entry from the archive's
//! central directory, the difference between two deliveries without inflating
//! a single unchanged file, and the changed entries read on their own.

mod support;

use std::collections::BTreeSet;
use std::sync::Mutex;

use netex_rs::dataset::{Fingerprint, Manifest, Source};
use netex_rs::index::{Index, IndexBuilder};
use netex_rs::stream::{ReadOptions, Selection};

#[test]
fn the_manifest_fingerprints_every_entry_from_the_central_directory() {
    let fixtures = support::delfi_fixtures();
    let dir = support::scratch_dir("incremental-manifest");
    let zip = support::zip_of(&dir, "delivery.zip", &fixtures);
    let manifest = Source::open(&zip)
        .expect("the archive opens")
        .manifest()
        .expect("the archive lists its entries");
    assert_eq!(manifest.entries.len(), fixtures.len());
    for (fingerprint, fixture) in manifest.entries.iter().zip(&fixtures) {
        assert_eq!(fingerprint.name, fixture.name);
        assert_eq!(fingerprint.size, fixture.xml.len() as u64);
        assert_eq!(
            fingerprint.crc32,
            crc32fast::hash(fixture.xml.as_bytes()),
            "{}",
            fixture.name
        );
    }
    // A directory has no central directory: the fingerprint is computed from
    // the file, and is the same.
    let from_dir = Source::open(support::fixtures_dir().join("delfi"))
        .expect("the directory opens")
        .manifest()
        .expect("the directory lists its files");
    assert_eq!(from_dir, manifest);
}

#[test]
fn the_manifest_of_an_archive_is_read_without_inflating_its_entries() {
    let fixtures = support::delfi_fixtures();
    let dir = support::scratch_dir("incremental-no-inflate");
    let zip = support::zip_of(&dir, "delivery.zip", &fixtures);
    // Corrupt one byte of the first entry's stored data — the central
    // directory at the end of the archive keeps the entry's fingerprint.
    let mut bytes = std::fs::read(&zip).expect("the archive is readable");
    let marker = b"<PublicationTimestamp>";
    let at = bytes
        .windows(marker.len())
        .position(|window| window == marker)
        .expect("the first entry holds a publication timestamp");
    bytes[at + 1] ^= 0x20;
    std::fs::write(&zip, bytes).expect("the archive is rewritten");

    let source = Source::open(&zip).expect("the archive opens");
    let manifest = source.manifest().expect("the manifest is read");
    assert_eq!(
        manifest.entries[0].crc32,
        crc32fast::hash(fixtures[0].xml.as_bytes()),
        "the fingerprint is the central directory's, untouched"
    );
    let failures = Mutex::new(Vec::new());
    source
        .read_each(|entry, text| {
            if text.is_err() {
                failures
                    .lock()
                    .expect("not poisoned")
                    .push(entry.name.clone());
            }
        })
        .expect("the archive lists its entries");
    assert_eq!(
        failures.into_inner().expect("not poisoned"),
        vec![fixtures[0].name.clone()],
        "inflating the corrupted entry fails on its checksum"
    );
}

#[test]
fn two_deliveries_differ_by_what_is_new_changed_and_gone() {
    let fixtures = support::delfi_fixtures();
    let dir = support::scratch_dir("incremental-diff");
    let last_week = support::zip_of(&dir, "last-week.zip", &fixtures[..4]);
    let mut this_week_fixtures = fixtures[1..].to_vec();
    this_week_fixtures[1].xml = this_week_fixtures[1]
        .xml
        .replacen("<Name>", "<Name>Neu: ", 1);
    let this_week = support::zip_of(&dir, "this-week.zip", &this_week_fixtures);

    let previous = Source::open(&last_week)
        .expect("last week's archive opens")
        .manifest()
        .expect("last week's manifest");
    let source = Source::open(&this_week).expect("this week's archive opens");
    let current = source.manifest().expect("this week's manifest");
    let changes = current.diff(&previous);
    let names = |list: &[&netex_rs::dataset::Fingerprint]| -> Vec<String> {
        list.iter().map(|f| f.name.clone()).collect()
    };
    assert_eq!(names(&changes.added), vec![fixtures[4].name.clone()]);
    assert_eq!(names(&changes.changed), vec![fixtures[2].name.clone()]);
    assert_eq!(names(&changes.removed), vec![fixtures[0].name.clone()]);
    assert_eq!(
        names(&changes.unchanged),
        vec![fixtures[1].name.clone(), fixtures[3].name.clone()]
    );
    let to_read: BTreeSet<&str> = changes.to_read().collect();
    assert_eq!(
        to_read,
        [fixtures[2].name.as_str(), fixtures[4].name.as_str()]
            .into_iter()
            .collect()
    );

    // Only the changed and new entries are read.
    let entries: Vec<_> = source
        .entries()
        .expect("this week's entries")
        .into_iter()
        .filter(|entry| to_read.contains(entry.name.as_str()))
        .collect();
    assert_eq!(entries.len(), 2);
    let read = Mutex::new(Vec::new());
    source
        .read_entries_with(
            entries.clone(),
            || (),
            |(), entry, text| {
                let text = text.expect("the entry loads");
                assert_eq!(text.len() as u64, entry.size);
                read.lock().expect("not poisoned").push(entry.name.clone());
            },
        )
        .expect("the changed entries read");
    let mut read = read.into_inner().expect("not poisoned");
    read.sort();
    assert_eq!(
        read,
        to_read.iter().map(|n| n.to_string()).collect::<Vec<_>>()
    );

    // The index of just those entries is the index one builder builds from them.
    let options = ReadOptions::new(Selection::routing());
    let partial = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .expect("a pool of one thread")
        .install(|| Index::build_from(&source, entries, &options).expect("the entries index"));
    let mut expected = IndexBuilder::new();
    let mut chosen: Vec<&support::Fixture> = this_week_fixtures
        .iter()
        .filter(|f| to_read.contains(f.name.as_str()))
        .collect();
    chosen.sort_by_key(|f| std::cmp::Reverse(f.xml.len()));
    for fixture in chosen {
        expected
            .add_streamed(&fixture.xml, &options, &fixture.name)
            .expect("the fixture streams");
    }
    assert!(partial == expected.finish());
}

/// The German delivery names every file after its publication date —
/// `…-4111_20260912.xml` one week, `…-4111_20260919.xml` the next — so
/// compared by name, nothing is in both weeks. Compared by an identity the
/// caller derives from the name, the same files are the same documents.
#[test]
fn two_deliveries_are_compared_by_the_identity_the_caller_derives_from_the_name() {
    let fixtures = support::delfi_fixtures();
    let dir = support::scratch_dir("incremental-diff-by");
    let dated = |fixtures: &[support::Fixture], date: &str| -> Vec<support::Fixture> {
        fixtures
            .iter()
            .map(|f| support::Fixture {
                name: f.name.replace(".xml", &format!("_{date}.xml")),
                ..f.clone()
            })
            .collect()
    };
    let last_week = support::zip_of(&dir, "last-week.zip", &dated(&fixtures[..4], "20260912"));
    let mut this_week_fixtures = dated(&fixtures[1..], "20260919");
    this_week_fixtures[1].xml = this_week_fixtures[1]
        .xml
        .replacen("<Name>", "<Name>Neu: ", 1);
    let this_week = support::zip_of(&dir, "this-week.zip", &this_week_fixtures);
    let previous = Source::open(&last_week)
        .expect("last week's archive opens")
        .manifest()
        .expect("last week's manifest");
    let current = Source::open(&this_week)
        .expect("this week's archive opens")
        .manifest()
        .expect("this week's manifest");

    // By name, every file is new and every old one gone.
    let by_name = current.diff(&previous);
    assert_eq!(by_name.added.len(), 4);
    assert_eq!(by_name.removed.len(), 4);
    assert!(by_name.changed.is_empty() && by_name.unchanged.is_empty());

    // By the name without its date, the documents are the same documents.
    let without_date = |name: &str| -> String {
        let (stem, _) = name.rsplit_once('_').expect("a dated name");
        stem.to_owned()
    };
    let changes = current.diff_by(&previous, without_date);
    let names =
        |list: &[&Fingerprint]| -> Vec<String> { list.iter().map(|f| f.name.clone()).collect() };
    assert_eq!(
        names(&changes.added),
        vec![this_week_fixtures[3].name.clone()]
    );
    assert_eq!(
        names(&changes.changed),
        vec![this_week_fixtures[1].name.clone()]
    );
    assert_eq!(
        names(&changes.removed),
        vec![fixtures[0].name.replace(".xml", "_20260912.xml")],
        "the removed document keeps last week's name"
    );
    assert_eq!(
        names(&changes.unchanged),
        vec![
            this_week_fixtures[0].name.clone(),
            this_week_fixtures[2].name.clone()
        ],
        "an unchanged document is reported under this week's name"
    );
}

#[test]
fn a_manifest_round_trips_through_its_text_form() {
    let fixtures = support::delfi_fixtures();
    let dir = support::scratch_dir("incremental-text");
    let zip = support::zip_of(&dir, "delivery.zip", &fixtures);
    let manifest = Source::open(&zip)
        .expect("the archive opens")
        .manifest()
        .expect("the manifest is read");
    let text = manifest.to_text();
    assert_eq!(text.lines().count(), fixtures.len());
    assert!(
        text.lines().all(|line| line.split('\t').count() == 3),
        "one line per entry: crc, size, name\n{text}"
    );
    assert_eq!(Manifest::parse(&text).expect("the text parses"), manifest);
    assert!(Manifest::parse("not a manifest").is_err());
    assert!(Manifest::parse("zz\t1\tname.xml").is_err());
}
