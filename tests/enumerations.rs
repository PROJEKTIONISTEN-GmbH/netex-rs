//! The transcribed enumerations against the schemas of both releases.
//!
//! Each enumeration names the `simpleType` it transcribes; its tokens must be
//! exactly that type's tokens, in order, in the 2.0 schemas — and in the 1.x
//! schemas too, except for the tokens the release difference table lists. The
//! schemas are read where they are (`support::official_fixtures`).

mod support;

use std::collections::BTreeMap;
use std::path::Path;

use netex_rs::model::{
    AccessMode, AccessSpaceType, DirectionType, EntranceType, FareClass, LimitationStatus,
    QuayType, StopType, StopUse, TimingPointStatus, VehicleMode,
};
use netex_rs::version::table;
use netex_rs::Release;
use quick_xml::events::Event;
use quick_xml::Reader;
use support::ReleaseFixtures;

/// The transcribed enumerations: schema type name and the crate's tokens.
fn transcribed() -> Vec<(&'static str, &'static [&'static str])> {
    vec![
        (VehicleMode::XSD_TYPE, VehicleMode::TOKENS),
        (StopType::XSD_TYPE, StopType::TOKENS),
        (QuayType::XSD_TYPE, QuayType::TOKENS),
        (FareClass::XSD_TYPE, FareClass::TOKENS),
        (DirectionType::XSD_TYPE, DirectionType::TOKENS),
        (LimitationStatus::XSD_TYPE, LimitationStatus::TOKENS),
        (EntranceType::XSD_TYPE, EntranceType::TOKENS),
        (AccessSpaceType::XSD_TYPE, AccessSpaceType::TOKENS),
        (AccessMode::XSD_TYPE, AccessMode::TOKENS),
        (StopUse::XSD_TYPE, StopUse::TOKENS),
        (TimingPointStatus::XSD_TYPE, TimingPointStatus::TOKENS),
    ]
}

/// Tokens an enumeration holds beyond the type it transcribes: those of the
/// types the elements carrying it have where that type is another.
fn beyond_the_type(xsd_type: &str) -> &'static [&'static str] {
    if xsd_type == VehicleMode::XSD_TYPE {
        // `AllPublicTransportModesEnumeration` (2.0) and
        // `AllVehicleModesOfTransportEnumeration` (1.x) add `other`.
        &["other"]
    } else {
        &[]
    }
}

/// The tokens of `xsd_type` as the schema lists them, with those the crate
/// holds beyond it.
fn expected_tokens(schema: &[String], xsd_type: &str) -> Vec<String> {
    let mut tokens = schema.to_vec();
    tokens.extend(
        beyond_the_type(xsd_type)
            .iter()
            .map(|token| (*token).to_owned()),
    );
    tokens
}

/// Every named enumerated `simpleType` of a schema set: name → tokens in order.
fn schema_enumerations(release: Release) -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::new();
    collect(&release.dir().join("xsd"), &mut out);
    out
}

fn collect(dir: &Path, out: &mut BTreeMap<String, Vec<String>>) {
    for entry in std::fs::read_dir(dir).expect("schema directory is readable") {
        let path = entry.expect("readable entry").path();
        if path.is_dir() {
            collect(&path, out);
        } else if path.extension().is_some_and(|e| e == "xsd") {
            let xml = std::fs::read_to_string(&path).expect("schema is readable");
            read_enumerations(&xml, out);
        }
    }
}

fn read_enumerations(xml: &str, out: &mut BTreeMap<String, Vec<String>>) {
    let mut reader = Reader::from_str(xml);
    let mut current: Option<(String, Vec<String>)> = None;
    let mut depth = 0usize;
    let mut simple_type_depth = None;
    loop {
        let event = reader.read_event().expect("schema is well-formed");
        match event {
            Event::Start(ref start) | Event::Empty(ref start) => {
                let local = start.local_name().as_ref().to_owned();
                if matches!(event, Event::Start(_)) {
                    depth += 1;
                }
                if local == "simpleType" && simple_type_depth.is_none() {
                    if let Some(name) = start
                        .attributes()
                        .flatten()
                        .find(|a| a.key.as_ref() == "name")
                    {
                        current = Some((name.value.into_owned(), Vec::new()));
                        simple_type_depth = Some(depth);
                    }
                }
                // A documented token is a start tag with an annotation inside;
                // an undocumented one is an empty tag. Both count.
                if local == "enumeration" {
                    if let (Some((_, tokens)), Some(value)) = (
                        &mut current,
                        start
                            .attributes()
                            .flatten()
                            .find(|a| a.key.as_ref() == "value"),
                    ) {
                        tokens.push(value.value.into_owned());
                    }
                }
            }
            Event::End(_) => {
                if simple_type_depth == Some(depth) {
                    if let Some((name, tokens)) = current.take() {
                        if !tokens.is_empty() {
                            out.insert(name, tokens);
                        }
                    }
                    simple_type_depth = None;
                }
                depth -= 1;
            }
            Event::Eof => break,
            _ => {}
        }
    }
}

#[test]
fn every_transcribed_enumeration_matches_the_20_schemas_token_for_token() {
    if !support::official_fixtures(Release::V2) {
        return;
    }
    let schema = schema_enumerations(Release::V2);
    let mut failures = Vec::new();
    for (xsd_type, tokens) in transcribed() {
        let Some(expected) = schema.get(xsd_type) else {
            failures.push(format!(
                "{xsd_type}: no such enumerated type in the 2.0 schemas"
            ));
            continue;
        };
        let expected = expected_tokens(expected, xsd_type);
        if expected != tokens {
            failures.push(format!(
                "{xsd_type}:\n  schema: {expected:?}\n  crate:  {tokens:?}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "{}",
        support::report("enumerations differ from the 2.0 schemas", &failures)
    );
}

/// The 1.x schemas list the same tokens, minus those the difference table
/// names as 2.0-only for the elements carrying the type.
#[test]
fn every_transcribed_enumeration_matches_the_1x_schemas_up_to_the_table() {
    if !support::official_fixtures(Release::V1) {
        return;
    }
    let schema = schema_enumerations(Release::V1);
    let only_20: Vec<&str> = table::TOKENS_ONLY_IN_20
        .iter()
        .map(|(_, token)| *token)
        .collect();
    let mut failures = Vec::new();
    for (xsd_type, tokens) in transcribed() {
        let Some(expected) = schema.get(xsd_type) else {
            failures.push(format!(
                "{xsd_type}: no such enumerated type in the 1.x schemas"
            ));
            continue;
        };
        let expected = expected_tokens(expected, xsd_type);
        let without_20: Vec<&str> = tokens
            .iter()
            .copied()
            .filter(|t| !only_20.contains(t) || expected.iter().any(|e| e == t))
            .collect();
        if expected.iter().map(String::as_str).collect::<Vec<_>>() != without_20 {
            failures.push(format!("{xsd_type}:\n  1.x schema: {expected:?}\n  crate minus 2.0-only tokens: {without_20:?}"));
        }
    }
    assert!(
        failures.is_empty(),
        "{}",
        support::report("enumerations differ from the 1.x schemas", &failures)
    );
}

/// The types the schemas give the elements of `name`, each with its tokens.
fn types_of_element(
    release: Release,
    name: &str,
    enumerations: &BTreeMap<String, Vec<String>>,
) -> BTreeMap<String, Vec<String>> {
    fn collect_types(dir: &Path, name: &str, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).expect("schema directory is readable") {
            let path = entry.expect("readable entry").path();
            if path.is_dir() {
                collect_types(&path, name, out);
            } else if path.extension().is_some_and(|e| e == "xsd") {
                let xml = std::fs::read_to_string(&path).expect("schema is readable");
                let declaration = format!(r#"<xsd:element name="{name}" type=""#);
                for (at, _) in xml.match_indices(&declaration) {
                    let rest = &xml[at + declaration.len()..];
                    out.push(rest[..rest.find('"').expect("a closing quote")].to_owned());
                }
            }
        }
    }
    let mut types = Vec::new();
    collect_types(&release.dir().join("xsd"), name, &mut types);
    types
        .into_iter()
        .filter_map(|xsd_type| {
            let tokens = enumerations.get(&xsd_type)?.clone();
            Some((xsd_type, tokens))
        })
        .collect()
}

/// `TransportMode` and `VehicleMode` are typed with one enumeration of modes
/// or another, element by element; the crate reads them all as
/// [`VehicleMode`], which must hold every token of each.
#[test]
fn every_mode_an_element_of_a_mode_may_hold_is_a_token_of_vehicle_mode() {
    let mut failures = Vec::new();
    for release in [Release::V2, Release::V1] {
        if !support::official_fixtures(release) {
            continue;
        }
        let enumerations = schema_enumerations(release);
        for element in ["TransportMode", "VehicleMode"] {
            let types = types_of_element(release, element, &enumerations);
            assert!(!types.is_empty(), "{release}: no type of <{element}>");
            for (xsd_type, tokens) in types {
                for token in tokens {
                    if !VehicleMode::TOKENS.contains(&token.as_str()) {
                        failures.push(format!("{release} <{element}> {xsd_type}: `{token}`"));
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{}",
        support::report("modes the crate reads as unrecognised", &failures)
    );
    assert!(
        !matches!(
            VehicleMode::from_token("other"),
            VehicleMode::Unrecognised(_)
        ),
        "`other` is a token of the schemas"
    );
}
