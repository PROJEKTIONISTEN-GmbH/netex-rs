//! The size an archive states for a member is the archive's claim, not a
//! promise: a member claiming more than any machine holds is that member's
//! error, and the process reading the delivery lives on. And however large a
//! member is, claimed or inflated, a document is read only up to the limit
//! its source sets.
//!
//! In a file of its own, since what it guards against ends the process
//! running the tests.

mod support;

use std::sync::atomic::{AtomicUsize, Ordering};

use netex_rs::dataset::{Source, DEFAULT_MAX_DOCUMENT_BYTES};
use netex_rs::error::{Kind, Limit};
use support::archive::{stored, Member};

/// A source over the archive of `members`, written into a directory of its own.
fn archive_of(members: &[Member<'_>]) -> (tempfile::TempDir, Source) {
    let directory = tempfile::tempdir().expect("a directory");
    let path = directory.path().join("delivery.zip");
    std::fs::write(&path, stored(members)).expect("the archive is written");
    let source = Source::open(&path).expect("the archive opens");
    (directory, source)
}

/// What became of each document of `source`, read document by document.
fn kinds(source: &Source) -> Vec<Option<Kind>> {
    source
        .entries()
        .expect("the archive lists its documents")
        .iter()
        .map(|entry| source.load(entry).xml.err().map(|error| error.kind))
        .collect()
}

#[test]
fn a_member_claiming_a_petabyte_is_its_own_error_and_no_abort() {
    let (_directory, source) =
        archive_of(&[Member::new("a.xml", b"<PublicationDelivery/>").claiming(1 << 50)]);
    let failed = AtomicUsize::new(0);
    let read = source.read_each(|_, text| {
        if text.is_err() {
            failed.fetch_add(1, Ordering::Relaxed);
        }
    });
    assert!(
        read.is_err() || failed.load(Ordering::Relaxed) == 1,
        "the member claiming a petabyte is read as if it held one"
    );
}

#[test]
fn a_member_claiming_more_than_the_limit_is_refused_unread_beside_the_others() {
    let text = b"<PublicationDelivery/>";
    let (_directory, source) = archive_of(&[
        Member::new("claiming.xml", text).claiming(DEFAULT_MAX_DOCUMENT_BYTES + 1),
        Member::new("plain.xml", text),
    ]);
    assert_eq!(
        kinds(&source),
        [
            Some(Kind::Limit(Limit::Document(DEFAULT_MAX_DOCUMENT_BYTES))),
            None
        ]
    );
}

#[test]
fn the_default_limit_lies_above_the_largest_document_of_a_national_delivery() {
    // Belgium's railway delivery states its network in one document of
    // 1 084 707 209 bytes, the largest member of the deliveries read so far.
    let largest_seen: u64 = 1_084_707_209;
    assert!(DEFAULT_MAX_DOCUMENT_BYTES > 2 * largest_seen);
}

#[test]
fn a_document_larger_than_the_limit_its_source_sets_is_refused() {
    let large = vec![b' '; 4_000];
    let (_directory, source) = archive_of(&[
        Member::new("large.xml", &large),
        Member::new("small.xml", b"<PublicationDelivery/>"),
    ]);
    let source = source.with_max_document_bytes(1_000);
    assert_eq!(
        kinds(&source),
        [Some(Kind::Limit(Limit::Document(1_000))), None]
    );
}

#[test]
fn a_member_holding_more_than_it_claims_is_read_no_further_than_the_limit() {
    // The central directory says a hundred bytes; the member holds four
    // thousand. The limit holds against what inflates, not against the claim.
    let large = vec![b' '; 4_000];
    let (_directory, source) = archive_of(&[Member::new("lying.xml", &large).claiming(100)]);
    let source = source.with_max_document_bytes(1_000);
    let failed = AtomicUsize::new(0);
    source
        .read_each(|_, text| {
            if let Err(error) = text {
                assert_eq!(error.kind, Kind::Limit(Limit::Document(1_000)));
                failed.fetch_add(1, Ordering::Relaxed);
            }
        })
        .expect("the archive lists its documents");
    assert_eq!(failed.load(Ordering::Relaxed), 1);
}

#[test]
fn a_file_of_a_directory_larger_than_the_limit_is_refused() {
    let directory = tempfile::tempdir().expect("a directory");
    std::fs::write(directory.path().join("large.xml"), vec![b' '; 4_000]).expect("written");
    let source = Source::open(directory.path())
        .expect("the directory opens")
        .with_max_document_bytes(1_000);
    assert_eq!(kinds(&source), [Some(Kind::Limit(Limit::Document(1_000)))]);
}

#[test]
fn a_limit_set_on_sources_read_as_one_holds_for_every_part() {
    let large = vec![b' '; 4_000];
    let (_first, first) = archive_of(&[Member::new("first.xml", b"<PublicationDelivery/>")]);
    let (_second, second) = archive_of(&[Member::new("second.xml", &large)]);
    let source = first
        .with(second.with_max_document_bytes(u64::MAX))
        .expect("the sources combine")
        .with_max_document_bytes(1_000);
    assert_eq!(
        kinds(&source),
        [None, Some(Kind::Limit(Limit::Document(1_000)))]
    );
}
