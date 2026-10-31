//! Robustness against foreign input: limits, entities, encodings, and errors
//! that say where they come from.

use netex_rs::error::{Kind, Limit};
use netex_rs::model::{StopType, VehicleMode};
use netex_rs::{Fragment, Limits};

const HEAD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<PublicationDelivery xmlns="http://www.netex.org.uk/netex" version="ntx:1.1">
  <PublicationTimestamp>2026-09-19T15:04:00</PublicationTimestamp>
  <ParticipantRef>DE</ParticipantRef>
  <dataObjects>
"#;
const TAIL: &str = "  </dataObjects>\n</PublicationDelivery>\n";

fn document(body: &str) -> String {
    format!("{HEAD}{body}{TAIL}")
}

#[test]
fn nesting_deeper_than_the_limit_is_refused_with_its_depth() {
    let mut body = String::new();
    for _ in 0..300 {
        body.push_str("<GeneralFrame>");
    }
    for _ in 0..300 {
        body.push_str("</GeneralFrame>");
    }
    let xml = document(&body);
    let error = netex_rs::read(&xml).expect_err("300 levels exceed the default limit of 128");
    assert_eq!(error.kind, Kind::Limit(Limit::Depth(128)));
    let location = error.location.expect("the error has a place");
    assert_eq!(location.line, 6);
    assert!(location.path.starts_with("PublicationDelivery/dataObjects"));

    let generous = Limits {
        max_depth: 1000,
        ..Limits::default()
    };
    netex_rs::read_with(&xml, &generous).expect("a higher limit lets it through");
}

#[test]
fn the_depth_limit_also_bounds_typed_elements() {
    let mut body = String::from("<SiteFrame><stopPlaces>");
    for _ in 0..60 {
        body.push_str("<StopPlace><quays><Quay><entrances><StopPlaceEntrance>");
    }
    body.push_str("<Name>deep</Name>");
    for _ in 0..60 {
        body.push_str("</StopPlaceEntrance></entrances></Quay></quays></StopPlace>");
    }
    body.push_str("</stopPlaces></SiteFrame>");
    let xml = document(&body);
    let error = netex_rs::read(&xml).expect_err("typed nesting is bounded too");
    assert_eq!(error.kind, Kind::Limit(Limit::Depth(128)));
}

#[test]
fn a_document_declaring_a_dtd_is_refused() {
    let xml = r#"<?xml version="1.0"?>
<!DOCTYPE PublicationDelivery [ <!ENTITY lol "lol"> <!ENTITY lol2 "&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;&lol;"> ]>
<PublicationDelivery xmlns="http://www.netex.org.uk/netex"><ParticipantRef>&lol2;</ParticipantRef></PublicationDelivery>"#;
    let error = netex_rs::read(xml).expect_err("a DTD is refused before any entity is seen");
    assert!(matches!(error.kind, Kind::Entity { .. }), "{error}");
    assert_eq!(error.location.expect("placed").line, 2);
}

#[test]
fn an_undefined_entity_is_refused_and_never_expanded() {
    let xml = document("<ResourceFrame><Name>&bomb;</Name></ResourceFrame>");
    let error = netex_rs::read(&xml).expect_err("an undefined entity is an error");
    assert_eq!(
        error.kind,
        Kind::Entity {
            name: "bomb".to_owned()
        }
    );
    // Inside a carried-through fragment as well.
    let xml = document("<FareFrame><fareZones><Thing>&bomb;</Thing></fareZones></FareFrame>");
    let error =
        netex_rs::read(&xml).expect_err("an undefined entity in a fragment is an error too");
    assert_eq!(
        error.kind,
        Kind::Entity {
            name: "bomb".to_owned()
        }
    );
    let location = error.location.expect("placed");
    assert_eq!(location.path, "PublicationDelivery/dataObjects");
}

#[test]
fn predefined_and_numeric_references_are_the_characters_they_name() {
    let xml = document("<ResourceFrame><Name>A &amp; B &lt; C &#x41; &#66;</Name></ResourceFrame>");
    let parsed = netex_rs::read(&xml).expect("references resolve");
    let netex_rs::model::Frame::Resource(frame) =
        &parsed.delivery.data_objects.as_ref().unwrap().items[0]
    else {
        panic!("a resource frame");
    };
    assert_eq!(frame.name.as_ref().unwrap().as_str(), "A & B < C A B");
    assert!(parsed.write().contains("<Name>A &amp; B &lt; C A B</Name>"));
}

#[test]
fn a_declared_encoding_other_than_utf8_is_refused() {
    let xml = r#"<?xml version="1.0" encoding="ISO-8859-1"?><PublicationDelivery xmlns="http://www.netex.org.uk/netex"/>"#;
    let error = netex_rs::read(xml).expect_err("only UTF-8 is read");
    assert_eq!(
        error.kind,
        Kind::Encoding {
            declared: "ISO-8859-1".to_owned()
        }
    );
}

#[test]
fn more_attributes_than_the_limit_are_refused() {
    let mut attributes = String::new();
    for i in 0..300 {
        attributes.push_str(&format!(" a{i}=\"{i}\""));
    }
    let xml = document(&format!("<ResourceFrame{attributes}/>"));
    let error = netex_rs::read(&xml).expect_err("300 attributes exceed the default limit of 256");
    assert_eq!(error.kind, Kind::Limit(Limit::Attributes(256)));
}

#[test]
fn character_data_longer_than_the_limit_is_refused() {
    let limits = Limits {
        max_text: 100,
        ..Limits::default()
    };
    let xml = document(&format!(
        "<ResourceFrame><Name>{}</Name></ResourceFrame>",
        "x".repeat(101)
    ));
    let error = netex_rs::read_with(&xml, &limits).expect_err("101 bytes exceed a limit of 100");
    assert_eq!(error.kind, Kind::Limit(Limit::Text(100)));
    let xml = document(&format!(
        "<ResourceFrame><Name>{}</Name></ResourceFrame>",
        "x".repeat(100)
    ));
    netex_rs::read_with(&xml, &limits).expect("100 bytes are within it");
}

#[test]
fn a_fragment_larger_than_the_limit_is_refused() {
    let limits = Limits {
        max_fragment: 200,
        ..Limits::default()
    };
    let xml = document(&format!(
        "<FareFrame><Name>{}</Name></FareFrame>",
        "x".repeat(300)
    ));
    let error =
        netex_rs::read_with(&xml, &limits).expect_err("a 300-byte fragment exceeds a limit of 200");
    assert_eq!(error.kind, Kind::Limit(Limit::Fragment(200)));
    assert_eq!(
        error.location.expect("placed").path,
        "PublicationDelivery/dataObjects"
    );
}

#[test]
fn errors_name_the_line_and_the_element_path() {
    let xml = document("<SiteFrame><stopPlaces><StopPlace><Name>ok</Name><Name>twice</Name></StopPlace></stopPlaces></SiteFrame>");
    let error = netex_rs::read(&xml).expect_err("a second Name is a duplicate");
    assert_eq!(
        error.kind,
        Kind::Duplicate {
            name: "Name".to_owned()
        }
    );
    assert!(error.to_string().contains("line 6"));
    assert!(error.to_string().contains("<Name> occurs more than once"));
    let location = error.location.expect("placed");
    assert_eq!(location.line, 6);
    assert_eq!(
        location.path,
        "PublicationDelivery/dataObjects/SiteFrame/stopPlaces/StopPlace"
    );

    let xml = document("<SiteFrame><stopPlaces><StopPlace><Covered>yes</Covered><BorderCrossing>maybe</BorderCrossing></StopPlace></stopPlaces></SiteFrame>");
    let error = netex_rs::read(&xml).expect_err("`maybe` is not a boolean");
    assert_eq!(
        error.kind,
        Kind::InvalidValue {
            datatype: "xsd:boolean",
            value: "maybe".to_owned()
        }
    );
    assert_eq!(
        error.location.expect("placed").path,
        "PublicationDelivery/dataObjects/SiteFrame/stopPlaces/StopPlace"
    );
}

#[test]
fn not_well_formed_xml_is_an_error_with_a_place() {
    let xml = document(
        "<SiteFrame><stopPlaces><StopPlace><Name>unclosed</StopPlace></stopPlaces></SiteFrame>",
    );
    let error = netex_rs::read(&xml).expect_err("mismatched end tag");
    assert!(matches!(error.kind, Kind::Xml(_)), "{error}");
    assert_eq!(error.location.expect("placed").line, 6);

    let error = netex_rs::read("").expect_err("an empty document");
    assert!(matches!(error.kind, Kind::NotNetex { .. }), "{error}");
    let error = netex_rs::read("<Siri xmlns=\"http://www.siri.org.uk/siri\"/>")
        .expect_err("a SIRI document");
    assert_eq!(
        error.kind,
        Kind::NotNetex {
            root: "Siri".to_owned()
        }
    );
    let error = Fragment::read("<Thing/>").expect_err("a fragment outside the namespace");
    assert_eq!(
        error.kind,
        Kind::NotNetex {
            root: "Thing".to_owned()
        }
    );
}

#[test]
fn unknown_enumeration_tokens_are_kept_and_written_back_unchanged() {
    let xml = document("<SiteFrame><stopPlaces><StopPlace><TransportMode>hyperloop</TransportMode><StopPlaceType>onstreetBus</StopPlaceType></StopPlace></stopPlaces></SiteFrame>");
    let parsed = netex_rs::read(&xml).expect("an unknown token is not an error");
    let netex_rs::model::Frame::Site(frame) =
        &parsed.delivery.data_objects.as_ref().unwrap().items[0]
    else {
        panic!("a site frame");
    };
    let netex_rs::model::StopPlaceItem::StopPlace(place) =
        &frame.stop_places.as_ref().unwrap().items[0]
    else {
        panic!("a stop place");
    };
    assert_eq!(
        place.transport_mode,
        Some(VehicleMode::Unrecognised("hyperloop".into()))
    );
    assert_eq!(place.stop_place_type, Some(StopType::OnstreetBus));
    assert!(parsed
        .write()
        .contains("<TransportMode>hyperloop</TransportMode>"));
    // The trailing space the German delivery writes in `secondClass ` survives too.
    let xml = document("<ResourceFrame><vehicleTypes><VehicleType><capacities><PassengerCapacity><FareClass>secondClass </FareClass></PassengerCapacity></capacities></VehicleType></vehicleTypes></ResourceFrame>");
    let parsed = netex_rs::read(&xml).expect("reads");
    assert!(parsed
        .write()
        .contains("<FareClass>secondClass </FareClass>"));
}

#[test]
fn a_document_in_a_prefixed_namespace_is_read_the_same() {
    let xml = r#"<?xml version="1.0"?>
<netex:PublicationDelivery xmlns:netex="http://www.netex.org.uk/netex" version="1.1">
  <netex:ParticipantRef>X</netex:ParticipantRef>
  <netex:dataObjects>
    <netex:SiteFrame id="f" version="1"><netex:stopPlaces><netex:StopPlace id="s" version="1"><netex:Name>Prefixed</netex:Name></netex:StopPlace></netex:stopPlaces></netex:SiteFrame>
  </netex:dataObjects>
</netex:PublicationDelivery>"#;
    let parsed = netex_rs::read(xml).expect("a prefixed document reads");
    assert_eq!(parsed.delivery.participant_ref.as_deref(), Some("X"));
    let netex_rs::model::Frame::Site(frame) =
        &parsed.delivery.data_objects.as_ref().unwrap().items[0]
    else {
        panic!("a site frame, typed although prefixed");
    };
    assert_eq!(frame.id.as_ref().map(|i| i.as_str()), Some("f"));
}

#[test]
fn an_attribute_stated_twice_is_refused_by_the_model_and_the_stream() {
    let xml = document(r#"<ResourceFrame id="r:1" version="1" id="r:2"/>"#);
    let error = netex_rs::read(&xml).expect_err("an attribute stated twice is not well-formed");
    assert!(
        matches!(&error.kind, Kind::Xml(message) if message.contains("duplicated attribute")),
        "{error}"
    );
    let options = netex_rs::stream::ReadOptions::default();
    let streamed = netex_rs::stream::read(&xml, &options, |_, _| Ok(()))
        .expect_err("the stream refuses it too");
    assert!(
        matches!(&streamed.kind, Kind::Xml(message) if message.contains("duplicated attribute")),
        "{streamed}"
    );
}

#[test]
fn an_attribute_value_is_normalised_where_a_reference_or_a_line_break_asks_for_it() {
    let xml = document(
        "<ResourceFrame id=\"r:a&amp;b\tc\nd\" version=\"1\"><Name>n</Name></ResourceFrame>",
    );
    let parsed = netex_rs::read(&xml).expect("the document reads");
    let netex_rs::model::Frame::Resource(frame) =
        &parsed.delivery.data_objects.as_ref().unwrap().items[0]
    else {
        panic!("a resource frame");
    };
    assert_eq!(frame.id.as_ref().map(|id| id.as_str()), Some("r:a&b c d"));
    assert_eq!(
        frame.version.as_ref().map(|version| version.as_str()),
        Some("1")
    );
}
