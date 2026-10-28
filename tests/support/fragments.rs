//! The crate's own fragments under `tests/fixtures/fragments/`: small sets of
//! frames, and entities each standing alone, every file pinning what the
//! reader makes of one kind of thing a delivery states (see the directory's
//! `README.md`).

use super::{fixtures_dir, fixtures_under, Fixture};

/// Every set of frames, by file name, in name order.
pub fn frames() -> Vec<Fixture> {
    fixtures_under(&fixtures_dir().join("fragments/frames"))
}

/// Every set of entities standing alone, by file name, in name order.
pub fn entities() -> Vec<Fixture> {
    fixtures_under(&fixtures_dir().join("fragments/entities"))
}

/// The set of frames of this file name, wrapped in a publication.
pub fn document(name: &str) -> String {
    let path = fixtures_dir().join("fragments/frames").join(name);
    let frames =
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    publication(&frames)
}

/// Frames wrapped in a publication of this crate's, in a composite frame.
pub fn publication(frames: &str) -> String {
    delivery(&format!(
        r#"<CompositeFrame id="netex-rs:fragments" version="1"><frames>{frames}</frames></CompositeFrame>"#
    ))
}

/// Entities wrapped in a publication of this crate's, as the members of a
/// general frame.
pub fn members(entities: &str) -> String {
    delivery(&format!(
        r#"<GeneralFrame id="netex-rs:fragments" version="1"><members>{entities}</members></GeneralFrame>"#
    ))
}

/// A delivery of one frame, declaring the namespaces of NeTEx and GML.
fn delivery(frame: &str) -> String {
    format!(
        concat!(
            r#"<PublicationDelivery xmlns="http://www.netex.org.uk/netex" "#,
            r#"xmlns:gml="http://www.opengis.net/gml/3.2" version="ntx:1.1">"#,
            "<PublicationTimestamp>2026-10-14T00:00:00</PublicationTimestamp>",
            "<ParticipantRef>netex-rs</ParticipantRef>",
            "<dataObjects>{}</dataObjects></PublicationDelivery>"
        ),
        frame
    )
}
