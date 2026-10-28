//! What a reader that borrows from the document rather than typing it builds on: a copy of the
//! cursor to go on from where an element does not read, the text of a leaf read as a typed leaf is
//! read, and the seconds of a time of day read from the text it is written as.

use netex_rs::model::Time;
use netex_rs::xml::Cursor;
use netex_rs::{Kind, Limits};

const NETEX: &str = "http://www.netex.org.uk/netex";

#[test]
fn a_copy_of_the_cursor_goes_on_from_where_it_was_taken() {
    let xml = format!(
        r#"<PublicationDelivery xmlns="{NETEX}"><calls><Call><Time>soon</Time></Call></calls><LineRef ref="l:1"/></PublicationDelivery>"#
    );
    let mut cursor = Cursor::new(&xml, Limits::default());
    let root = cursor.root().unwrap();
    cursor.enter(&root).unwrap();
    let calls = cursor.next_child().unwrap().unwrap();
    let at_the_calls = cursor.clone();

    // Read into the calls and give up inside them, as a reader does on a value it refuses.
    cursor.enter(&calls).unwrap();
    let call = cursor.next_child().unwrap().unwrap();
    cursor.enter(&call).unwrap();
    let time = cursor.next_child().unwrap().unwrap();
    assert_eq!(cursor.text(&time).unwrap(), "soon");

    // From the copy, the calls are skipped and the next sibling is read.
    let mut cursor = at_the_calls;
    cursor.skip_over(&calls).unwrap();
    let next = cursor.next_child().unwrap().unwrap();
    assert_eq!(next.local, "LineRef");
    cursor.skip_over(&next).unwrap();
    assert!(cursor.next_child().unwrap().is_none());
    cursor.leave();
    cursor.finish().unwrap();
}

#[test]
fn the_text_of_a_leaf_refuses_an_attribute_but_a_namespace_declaration() {
    let xml = format!(
        r#"<PublicationDelivery xmlns="{NETEX}"><Time xmlns:x="urn:x">08:00:00</Time><Time zone="CET">09:00:00</Time></PublicationDelivery>"#
    );
    let mut cursor = Cursor::new(&xml, Limits::default());
    let root = cursor.root().unwrap();
    cursor.enter(&root).unwrap();
    let declared = cursor.next_child().unwrap().unwrap();
    assert_eq!(cursor.leaf_text(&declared).unwrap(), "08:00:00");
    let attributed = cursor.next_child().unwrap().unwrap();
    let error = cursor.leaf_text(&attributed).unwrap_err();
    assert_eq!(
        error.kind,
        Kind::UnexpectedAttribute {
            name: "zone".to_owned()
        }
    );
}

#[test]
fn the_seconds_of_a_time_written_are_those_of_the_time_it_is() {
    for written in [
        "08:00:00",
        "24:00:00",
        "8:5:7",
        "08:00:00.5",
        "08:00:00Z",
        "08:00:00+01:00",
        "08:00:00-05:00",
        "",
        "25:00:00",
        "08:60:00",
        " 08:00:00",
        "08:00",
    ] {
        assert_eq!(
            Time::seconds_of(written),
            Time::from(written).seconds_since_midnight(),
            "{written:?}"
        );
    }
    assert_eq!(Time::seconds_of("23:59:30"), Some(86_370));
    assert_eq!(
        Time::seconds_of("08:00:00-05:00"),
        Some(8 * 3600),
        "a zone offset is ignored"
    );
}
