//! The service links a journey pattern names in sequence, against the stop
//! points it names: a link from or to a stop point that is not the pattern's
//! is a finding — the defect of 2 149 patterns of the Finnish delivery —,
//! the link found in the pattern's own document first.

mod support;

use netex_rs::index::{Concern, Index, IndexBuilder};
use netex_rs::model::{LinkInSequenceItem, ServiceJourneyPattern};
use netex_rs::stream::{Kind, ReadOptions, Selection};
use netex_rs::Limits;

/// The shared links and the line document, in the order of their names.
fn documents() -> Vec<(String, String)> {
    ["pattern-links-line.xml", "pattern-links-shared.xml"]
        .iter()
        .map(|&name| (name.to_owned(), support::fragments::document(name)))
        .collect()
}

fn findings(index: &Index) -> Vec<String> {
    index
        .findings()
        .filter(|finding| finding.concern == Concern::LinkBetweenOtherPoints)
        .map(|finding| finding.to_string())
        .collect()
}

/// The index of `documents`, each read lean with `options` into a builder of
/// its own, the builders merged.
fn merged(documents: &[(String, String)], options: &ReadOptions) -> Index {
    let mut merged = IndexBuilder::new();
    for (name, xml) in documents {
        let mut builder = IndexBuilder::new();
        builder
            .add_streamed(xml, options, name)
            .expect("the document reads");
        merged.merge(builder);
    }
    merged.finish()
}

#[test]
fn a_pattern_naming_a_link_between_other_stop_points_than_its_own_is_a_finding() {
    let documents = documents();
    let links = ReadOptions::new(Selection::routing().with(Kind::ServiceLink));
    let reversed: Vec<_> = documents.iter().rev().cloned().collect();
    for index in [
        support::timetable::serially(&documents),
        merged(&documents, &links),
        merged(&reversed, &links),
    ] {
        assert_eq!(
            findings(&index),
            ["service link a pattern names from or to a stop point not its own: ServiceJourneyPattern ex:ServiceJourneyPattern:other in pattern-links-line.xml (ex:ServiceLink:Braunlage-Elend)"],
            "a link per hop, one for the whole way, one its own document defines \
             between its points, and one no document defines are none"
        );
        assert_eq!(index.finding_count(Concern::LinkBetweenOtherPoints), 1);
    }
}

#[test]
fn without_the_service_links_read_no_pattern_link_is_judged() {
    let index = merged(&documents(), &ReadOptions::new(Selection::routing()));
    assert!(findings(&index).is_empty());
}

#[test]
fn the_links_of_a_pattern_are_typed_in_their_order() {
    let xml = concat!(
        r#"<ServiceJourneyPattern xmlns="http://www.netex.org.uk/netex" id="P" version="1">"#,
        r#"<linksInSequence>"#,
        r#"<ServiceLinkInJourneyPattern id="P-2" version="1" order="2"><ServiceLinkRef ref="B-C"/></ServiceLinkInJourneyPattern>"#,
        r#"<ServiceLinkInJourneyPattern id="P-1" version="1" order="1"><ServiceLinkRef ref="A-B"/></ServiceLinkInJourneyPattern>"#,
        r#"</linksInSequence></ServiceJourneyPattern>"#
    );
    let pattern: ServiceJourneyPattern =
        netex_rs::read_element(xml, "ServiceJourneyPattern", &Limits::default())
            .expect("the pattern reads");
    let links: Vec<(Option<u32>, Option<&str>)> = pattern
        .links_in_sequence
        .iter()
        .flat_map(|links| &links.items)
        .map(|item| match item {
            LinkInSequenceItem::ServiceLinkInJourneyPattern(link) => (
                link.order.as_ref().and_then(|order| order.value()),
                link.service_link_ref
                    .as_ref()
                    .and_then(|r| r.r#ref.as_ref())
                    .map(|id| id.as_str()),
            ),
            LinkInSequenceItem::Unprocessed(fragment) => (None, Some(&*fragment.name)),
        })
        .collect();
    assert_eq!(links, [(Some(2), Some("B-C")), (Some(1), Some("A-B"))]);
    assert!(pattern.unprocessed.is_empty(), "nothing carried untyped");
}
