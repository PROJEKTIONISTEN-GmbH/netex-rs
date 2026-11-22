//! The identity constraints of the schemas, checked on the crate's own: an
//! identifier stated twice at one version by elements of one type in the
//! NeTEx namespace, as the schemas' keys select them.

use netex_rs::check::duplicate_ids_in;
use netex_rs::error::Kind;

/// A delivery holding `members` in a frame of its own.
fn delivery(members: &str) -> String {
    format!(
        r#"<PublicationDelivery xmlns="http://www.netex.org.uk/netex" xmlns:gml="http://www.opengis.net/gml/3.2" version="2.0"><dataObjects><SiteFrame id="f" version="1"><stopPlaces>{members}</stopPlaces></SiteFrame></dataObjects></PublicationDelivery>"#
    )
}

/// The identifier and the element of each finding.
fn found(xml: &str) -> Vec<(String, String)> {
    duplicate_ids_in(xml)
        .expect("the document is well-formed")
        .into_iter()
        .map(|duplicate| (duplicate.id, duplicate.element))
        .collect()
}

#[test]
fn an_identifier_stated_twice_by_elements_of_one_type_is_found() {
    let xml = delivery(
        r#"<StopPlace id="X" version="1"/>
<StopPlace id="X" version="1"/>"#,
    );
    let findings = duplicate_ids_in(&xml).expect("the document is well-formed");
    assert_eq!(findings.len(), 1, "{findings:?}");
    assert_eq!(
        (
            findings[0].element.as_str(),
            findings[0].first_line,
            findings[0].line
        ),
        ("StopPlace", 1, 2)
    );
}

#[test]
fn elements_of_two_types_may_share_an_identifier_and_version() {
    // The schemas key each type apart (`StopPlace_UniqueBy_Id_Version`,
    // `Quay_UniqueBy_Id_Version`, …).
    let xml = delivery(r#"<StopPlace id="X" version="1"/><Quay id="X" version="1"/>"#);
    assert_eq!(found(&xml), []);
}

#[test]
fn an_element_without_a_version_is_in_no_key() {
    // `xsd:unique` leaves out an element lacking one of its fields.
    let xml = delivery(r#"<StopPlace id="X"/><StopPlace id="X"/>"#);
    assert_eq!(found(&xml), []);
}

#[test]
fn only_elements_of_the_netex_namespace_are_keyed() {
    let xml = delivery(
        r#"<StopPlace id="X" version="1"><gml:Point gml:id="p" id="g" version="1"/></StopPlace><StopPlace id="Y" version="1"><gml:Point gml:id="p" id="g" version="1"/></StopPlace>"#,
    );
    assert_eq!(found(&xml), []);
}

#[test]
fn identifiers_are_compared_as_read_not_as_escaped() {
    let xml =
        delivery(r#"<StopPlace id="a&amp;b" version="1"/><StopPlace id="a&#38;b" version="1"/>"#);
    assert_eq!(found(&xml), [("a&b".to_owned(), "StopPlace".to_owned())]);
}

#[test]
fn a_document_that_is_not_well_formed_is_an_error_not_a_partial_answer() {
    let xml =
        delivery(r#"<StopPlace id="X" version="1"/><Quay></Wrong><StopPlace id="X" version="1"/>"#);
    let error = duplicate_ids_in(&xml).expect_err("not well-formed");
    assert!(matches!(error.kind, Kind::Xml(_)), "{error}");
    let location = error.location.expect("where it stopped");
    assert_eq!(location.line, 1);
    assert!(location.path.ends_with("stopPlaces/Quay"), "{location}");
}
