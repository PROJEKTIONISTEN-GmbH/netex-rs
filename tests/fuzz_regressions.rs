//! Inputs the fuzzer found: each one panicked something once, and must not again.

use netex_rs::version::{detect, rewrite};
use netex_rs::Release;

/// A byte-order mark followed by a semicolon — and two marks followed by two
/// NULs: the tokenizer skips one mark, so its offsets and the text's bytes no
/// longer agree unless every leading mark is taken off first.
#[test]
fn byte_order_marks_do_not_derail_the_scanners() {
    for xml in [
        "\u{feff};",
        "\u{feff}\u{feff}\0\0",
        "\u{feff}\u{feff}\u{feff}<a/>",
    ] {
        let _ = detect(xml);
        for release in [Release::V1, Release::V2] {
            let _ = rewrite(xml, release);
        }
        assert!(netex_rs::read(xml).is_err());
        assert!(netex_rs::Fragment::read(xml).is_err());
        let _ = netex_rs::check::duplicate_ids_in(xml);
    }
}

/// A byte-order mark in front of a real document is read past.
#[test]
fn a_byte_order_mark_in_front_of_a_document_is_read_past() {
    let xml = "\u{feff}<PublicationDelivery xmlns=\"http://www.netex.org.uk/netex\" version=\"1.1\"><ParticipantRef>X</ParticipantRef></PublicationDelivery>";
    let document = netex_rs::read(xml).expect("the document reads");
    assert_eq!(document.delivery.participant_ref.as_deref(), Some("X"));
    let written = rewrite(xml, Release::V2);
    assert!(written.report.is_lossless());
    assert!(written.xml.contains("version=\"2.0\""));
    assert_eq!(detect(xml).declared, Some(Release::V1));
}
