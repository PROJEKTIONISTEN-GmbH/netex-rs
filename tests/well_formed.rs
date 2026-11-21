//! What is read is well-formed XML 1.0, at any depth, and what is written is
//! too: a document the reader takes, it can be written and read again, and
//! what it refuses no other parser would take either.

use netex_rs::error::Kind;
use netex_rs::model::{Frame, StopPlace, StopPlaceItem};
use netex_rs::stream::{self, ReadOptions, Selection};
use netex_rs::version::{detect, rewrite};
use netex_rs::{Attribute, Release};

const HEAD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<PublicationDelivery xmlns="http://www.netex.org.uk/netex" version="1.1">
  <PublicationTimestamp>2026-09-19T15:04:00</PublicationTimestamp>
  <ParticipantRef>DE</ParticipantRef>
  <dataObjects>
"#;
const TAIL: &str = "  </dataObjects>\n</PublicationDelivery>\n";

fn document(body: &str) -> String {
    format!("{HEAD}{body}{TAIL}")
}

/// A document with `inner` inside a stop place, an element the model carries
/// through untyped.
fn with_untyped(inner: &str) -> String {
    document(&format!(
        r#"<SiteFrame id="s" version="1"><stopPlaces><StopPlace id="p" version="1"><Foo>{inner}</Foo></StopPlace></stopPlaces></SiteFrame>"#
    ))
}

/// Whether the lean reader, which skips what it does not yield, takes `xml`.
fn streams(xml: &str) -> bool {
    stream::read(xml, &ReadOptions::new(Selection::routing()), |_, _| Ok(())).is_ok()
}

#[test]
fn an_element_carried_through_is_well_formed_to_its_innermost_attribute() {
    for inner in [
        r#"<Bar a="1" a="2"/>"#,
        "<Bar a=1/>",
        r#"<Bar a="x"b="y"/>"#,
        r#"<Bar a="&undefined;"/>"#,
        r#"<Bar a="&#1;"/>"#,
        r#"<Bar><Baz a="1" a="2"></Baz></Bar>"#,
    ] {
        let xml = with_untyped(inner);
        assert!(netex_rs::read(&xml).is_err(), "read: {inner}");
        assert!(!streams(&xml), "streamed: {inner}");
    }
    netex_rs::read(&with_untyped(r#"<Bar a="1" b='&amp;&#x41;'/>"#))
        .expect("well-formed attributes are read");
}

#[test]
fn a_character_xml_forbids_is_refused_as_a_reference_or_as_itself() {
    for text in ["a&#1;b", "a\u{1}b", "a&#xFFFE;b", "a\u{fffe}b"] {
        let xml = document(&format!(
            r#"<SiteFrame id="s" version="1"><stopPlaces><StopPlace id="p" version="1"><Name>{text}</Name></StopPlace></stopPlaces></SiteFrame>"#
        ));
        let error = netex_rs::read(&xml).expect_err(text);
        assert!(matches!(error.kind, Kind::Xml(_)), "{text}: {error}");
        assert!(!streams(&xml), "streamed: {text}");
        assert!(
            netex_rs::read(&with_untyped(text)).is_err(),
            "carried: {text}"
        );
    }
    for allowed in ["a&#9;b", "a\tb", "a&#x10FFFF;b", "a\u{fffd}b"] {
        netex_rs::read(&with_untyped(allowed)).expect(allowed);
    }
}

#[test]
fn character_data_or_a_declaration_out_of_place_is_refused() {
    let cdata_first = format!("<![CDATA[x]]>{}", &HEAD[HEAD.find('\n').unwrap() + 1..]) + TAIL;
    assert!(
        netex_rs::read(&cdata_first).is_err(),
        "CDATA before the root"
    );
    let declared_inside = HEAD.replace(
        "<ParticipantRef>",
        r#"<?xml version="1.0"?><ParticipantRef>"#,
    ) + TAIL;
    assert!(
        netex_rs::read(&declared_inside).is_err(),
        "a declaration inside"
    );
    assert!(
        netex_rs::read(&with_untyped(r#"<Bar><?xml version="1.0"?></Bar>"#)).is_err(),
        "a declaration inside an element carried through"
    );
    let declared_twice = format!(r#"<?xml version="1.0"?>{HEAD}{TAIL}"#);
    assert!(netex_rs::read(&declared_twice).is_err(), "two declarations");
}

#[test]
fn a_value_set_with_a_tab_or_a_line_break_reads_back_as_it_was_set() {
    let mut read = netex_rs::read(&document("")).expect("the document reads");
    let value = "one\ttwo\nthree\r\nfour";
    read.delivery.extra_attributes.push(Attribute {
        name: "note".into(),
        value: value.into(),
    });
    read.delivery.participant_ref = Some("line\rbreak".into());
    let again = netex_rs::read(&read.write()).expect("the written document reads");
    let note = again
        .delivery
        .extra_attributes
        .iter()
        .find(|attribute| &*attribute.name == "note")
        .expect("the attribute is written");
    assert_eq!(&*note.value, value);
    assert_eq!(
        again.delivery.participant_ref.as_deref(),
        Some("line\rbreak")
    );
}

#[test]
fn a_character_xml_forbids_set_by_hand_is_written_as_the_replacement_character() {
    let mut read = netex_rs::read(&document("")).expect("the document reads");
    read.delivery.participant_ref = Some("a\u{1}b".into());
    read.delivery.extra_attributes.push(Attribute {
        name: "note".into(),
        value: "c\u{fffe}d".into(),
    });
    let written = read.write();
    let again = netex_rs::read(&written).expect("the written document reads");
    assert_eq!(
        again.delivery.participant_ref.as_deref(),
        Some("a\u{fffd}b")
    );
    assert!(written.contains(r#"note="c�d""#), "{written}");
}

#[test]
fn an_element_written_as_a_document_of_its_own_reads_again() {
    let xml = document(
        r#"<SiteFrame id="s" version="1"><stopPlaces><StopPlace id="sp" version="1"><Name>Torget</Name></StopPlace></stopPlaces></SiteFrame>"#,
    );
    let read = netex_rs::read(&xml).expect("the document reads");
    let frames = read.delivery.data_objects.as_ref().expect("the frames");
    let Some(Frame::Site(site)) = frames.items.first() else {
        panic!("the site frame is read");
    };
    let places = site.stop_places.as_ref().expect("the stop places");
    let Some(StopPlaceItem::StopPlace(place)) = places.items.first() else {
        panic!("the stop place is read");
    };
    let place = StopPlace::clone(place);
    let written = netex_rs::write_element(&place, "StopPlace");
    let mut again: StopPlace =
        netex_rs::read_element(&written, "StopPlace", &netex_rs::Limits::default())
            .expect("the element written reads again");
    // Read as a document of its own, the element declares its namespace.
    let declared = again.extra_attributes.remove(0);
    assert_eq!(&*declared.name, "xmlns");
    assert_eq!(again, place);
}

#[test]
fn an_element_of_another_name_or_namespace_is_refused_naming_the_one_expected() {
    let error = netex_rs::read_element::<StopPlace>(
        r#"<Quay xmlns="http://www.netex.org.uk/netex" id="q" version="1"/>"#,
        "StopPlace",
        &netex_rs::Limits::default(),
    )
    .expect_err("a quay is no stop place");
    assert!(error.to_string().contains("<StopPlace>"), "{error}");
    let error = netex_rs::read_element::<StopPlace>(
        r#"<StopPlace id="sp" version="1"/>"#,
        "StopPlace",
        &netex_rs::Limits::default(),
    )
    .expect_err("no namespace");
    assert!(error.to_string().contains("namespace"), "{error}");
}

#[test]
fn a_rewrite_of_a_document_that_is_not_well_formed_says_so_and_keeps_the_rest() {
    let broken = with_untyped("<Bar></Baz>");
    let written = rewrite(&broken, Release::V2);
    assert!(!written.report.is_lossless());
    let malformed = written
        .report
        .malformed()
        .expect("where it is not well-formed");
    assert_eq!(malformed.line, 6);
    assert!(
        written
            .xml
            .ends_with(&broken[broken.find("</Baz>").unwrap()..]),
        "the rest is kept as it was: {}",
        written.xml
    );
    assert!(detect(&broken).malformed.is_some());
    assert!(detect(&with_untyped("<Bar/>")).malformed.is_none());
}

#[test]
fn content_of_another_namespace_below_the_root_is_neither_evidence_nor_left_out() {
    // `PassingTime` is an element of 1.x alone; under a default namespace of
    // another vocabulary it is that vocabulary's.
    let xml = document(
        r#"<GeneralFrame id="g" version="1"><Extensions><Other xmlns="urn:other"><PassingTime>kept</PassingTime></Other></Extensions></GeneralFrame>"#,
    );
    let detection = detect(&xml);
    assert_eq!(detection.structural, None, "{:?}", detection.evidence);
    let written = rewrite(&xml, Release::V2);
    assert!(written.report.is_lossless(), "{:?}", written.report);
    assert!(written.xml.contains("<PassingTime>kept</PassingTime>"));
    // A prefix bound to NeTEx below the root names NeTEx elements.
    let prefixed = document(
        r#"<GeneralFrame id="g" version="1"><Extensions><n:Wrapper xmlns:n="http://www.netex.org.uk/netex"><n:PassingTime/></n:Wrapper></Extensions></GeneralFrame>"#,
    );
    assert_eq!(detect(&prefixed).structural, Some(Release::V1));
}

#[test]
#[should_panic(expected = "no element is open")]
fn ending_an_element_never_started_is_a_panic_saying_so() {
    let mut emitter = netex_rs::xml::Emitter::new();
    emitter.end("StopPlace");
}
