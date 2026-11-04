//! The codes a managed object is known by in other systems — its key-value
//! pairs (`keyList`) and, in NeTEx 2.0, its typed codes (`privateCodes`) —,
//! typed on the managed objects the schema gives them to.

use netex_rs::model::{
    AllowedLineDirection, DataSource, Element, HeadwayJourneyGroup, KeyList, PassengerCapacity,
    PrivateCodes, ResponsibilitySet,
};
use netex_rs::Limits;

/// A lone `name`, its codes before `rest`, its first child the schema puts
/// after them.
fn element(name: &str, rest: &str) -> String {
    format!(
        concat!(
            r#"<{name} xmlns="http://www.netex.org.uk/netex" id="ex:{name}:1" version="1">"#,
            "<keyList><KeyValue><Key>legacy</Key><Value>4711</Value></KeyValue></keyList>",
            r#"<privateCodes><PrivateCode type="operator">A-17</PrivateCode></privateCodes>"#,
            "{rest}</{name}>"
        ),
        name = name,
        rest = rest
    )
}

/// Reads `xml` as a lone `name`, checks that it writes back as it was read,
/// and returns it.
fn read_back<T: Element + PartialEq + std::fmt::Debug>(xml: &str, name: &str) -> T {
    let read: T = netex_rs::read_element(xml, name, &Limits::default()).expect("it reads");
    let written = netex_rs::write_element(&read, name);
    let again: T =
        netex_rs::read_element(&written, name, &Limits::default()).expect("it reads again");
    assert_eq!(again, read, "{name} written back as it was read");
    read
}

fn assert_codes(name: &str, key_list: Option<&KeyList>, private_codes: Option<&PrivateCodes>) {
    assert_eq!(
        key_list.and_then(|k| k.get("legacy")),
        Some("4711"),
        "{name}"
    );
    assert_eq!(
        private_codes.and_then(|c| c.get("operator")),
        Some("A-17"),
        "{name}"
    );
}

#[test]
fn the_codes_of_the_managed_objects_are_typed() {
    let source: DataSource = read_back(&element("DataSource", "<Name>Quelle</Name>"), "DataSource");
    assert_codes(
        "DataSource",
        source.key_list.as_ref(),
        source.private_codes.as_ref(),
    );
    assert!(source.unprocessed.is_empty());

    let set: ResponsibilitySet = read_back(
        &element("ResponsibilitySet", "<Name>Zuständig</Name>"),
        "ResponsibilitySet",
    );
    assert_codes(
        "ResponsibilitySet",
        set.key_list.as_ref(),
        set.private_codes.as_ref(),
    );
    assert!(set.unprocessed.is_empty());

    let capacity: PassengerCapacity = read_back(
        &element("PassengerCapacity", "<TotalCapacity>80</TotalCapacity>"),
        "PassengerCapacity",
    );
    assert_codes(
        "PassengerCapacity",
        capacity.key_list.as_ref(),
        capacity.private_codes.as_ref(),
    );
    assert!(capacity.unprocessed.is_empty());

    let direction: AllowedLineDirection = read_back(
        &element("AllowedLineDirection", r#"<LineRef ref="ex:Line:1"/>"#),
        "AllowedLineDirection",
    );
    assert_codes(
        "AllowedLineDirection",
        direction.key_list.as_ref(),
        direction.private_codes.as_ref(),
    );
    assert!(direction.unprocessed.is_empty());

    let group: HeadwayJourneyGroup = read_back(
        &element(
            "HeadwayJourneyGroup",
            "<FirstDepartureTime>06:00:00</FirstDepartureTime>",
        ),
        "HeadwayJourneyGroup",
    );
    assert_codes(
        "HeadwayJourneyGroup",
        group.key_list.as_ref(),
        group.private_codes.as_ref(),
    );
    assert!(group.unprocessed.is_empty());
}
