//! A member's name is the publisher's text, not a path: a document is
//! listed under it as written, and loads only where the name is a path of
//! plain parts within its source — what a caller writing documents under
//! their names may join to a directory of its own. Of members the archive
//! gives one name, the first is read and the name reported.

mod support;

use netex_rs::dataset::Source;
use netex_rs::error::Kind;
use support::archive::{stored, Member};

const TEXT: &[u8] = b"<PublicationDelivery/>";

/// Each document of the archive of `members`: its name, and whether it loads
/// or is refused for its name.
fn loaded(members: &[Member<'_>]) -> Vec<(String, Result<(), String>)> {
    let directory = tempfile::tempdir().expect("a directory");
    let path = directory.path().join("delivery.zip");
    std::fs::write(&path, stored(members)).expect("the archive is written");
    let source = Source::open(&path).expect("the archive opens");
    source
        .entries()
        .expect("the archive lists its documents")
        .iter()
        .map(|entry| {
            let outcome = match source.load(entry).xml {
                Ok(_) => Ok(()),
                Err(error) => match error.kind {
                    Kind::DocumentName { name, .. } => Err(name),
                    other => panic!("{}: refused for another reason: {other}", entry.name),
                },
            };
            (entry.name.clone(), outcome)
        })
        .collect()
}

#[test]
fn a_name_climbing_out_of_the_archive_or_rooted_elsewhere_is_refused_when_loaded() {
    let refused = [
        "../up.xml",
        "a/../../up.xml",
        "a/../b.xml",
        "/absolute.xml",
        "./here.xml",
        "a//b.xml",
        "a\\..\\..\\up.xml",
    ];
    let mut members = vec![Member::new("plain/line.xml", TEXT)];
    members.extend(refused.iter().map(|name| Member::new(name, TEXT)));
    let outcomes = loaded(&members);
    assert_eq!(outcomes[0], ("plain/line.xml".to_owned(), Ok(())));
    for (name, outcome) in &outcomes[1..] {
        assert_eq!(outcome, &Err(name.clone()), "{name}");
    }
    assert_eq!(outcomes.len(), refused.len() + 1, "every member is listed");
}

#[test]
fn a_name_with_a_control_character_is_refused_when_loaded() {
    let outcomes = loaded(&[
        Member::new("line\n1.xml", TEXT),
        Member::new("line\u{7f}2.xml", TEXT),
        Member::new("line:3.xml", TEXT),
    ]);
    assert_eq!(
        outcomes,
        [
            ("line\n1.xml".to_owned(), Err("line\n1.xml".to_owned())),
            (
                "line\u{7f}2.xml".to_owned(),
                Err("line\u{7f}2.xml".to_owned())
            ),
            // Belgium's and Norway's deliveries name their lines so.
            ("line:3.xml".to_owned(), Ok(())),
        ]
    );
}

/// The text each document of the archive of `members` loads, by name, with
/// the names the archive gives more than one member.
fn texts(members: &[Member<'_>]) -> (Vec<(String, String)>, Vec<String>) {
    let directory = tempfile::tempdir().expect("a directory");
    let path = directory.path().join("delivery.zip");
    std::fs::write(&path, stored(members)).expect("the archive is written");
    let source = Source::open(&path).expect("the archive opens");
    let texts = source
        .entries()
        .expect("the archive lists its documents")
        .iter()
        .map(|entry| {
            let text = source.load(entry).xml.expect("the document loads");
            assert_eq!(entry.size, text.len() as u64, "{}", entry.name);
            (entry.name.clone(), text)
        })
        .collect();
    (texts, source.names_given_twice())
}

const SECOND: &[u8] = b"<PublicationDelivery><!-- the second --></PublicationDelivery>";
const THIRD: &[u8] =
    b"<PublicationDelivery><!-- the third, longer than the others --></PublicationDelivery>";

#[test]
fn of_two_members_of_one_name_the_first_is_read_and_the_name_counted() {
    let (texts, twice) = texts(&[
        Member::new("same.xml", TEXT),
        Member::new("other.xml", SECOND),
        Member::new("same.xml", SECOND),
        Member::new("same.xml", THIRD),
    ]);
    assert_eq!(
        texts,
        [
            ("same.xml".to_owned(), "<PublicationDelivery/>".to_owned()),
            (
                "other.xml".to_owned(),
                String::from_utf8(SECOND.to_vec()).expect("text")
            ),
        ]
    );
    assert_eq!(twice, ["same.xml"]);
}

#[test]
fn names_alike_once_read_are_one_name_given_twice() {
    // `ä` written in UTF-8 and flagged so, and in code page 437, where it is
    // the byte 0x84: two names in bytes, one once read.
    let (texts, twice) = texts(&[
        Member::new("ä.xml", TEXT).in_utf8(),
        Member::named_by_bytes(b"\x84.xml", SECOND),
    ]);
    assert_eq!(
        texts,
        [("ä.xml".to_owned(), "<PublicationDelivery/>".to_owned())]
    );
    assert_eq!(twice, ["ä.xml"]);
}

#[test]
fn an_archive_giving_every_member_a_name_of_its_own_names_none_twice() {
    let (texts, twice) = texts(&[Member::new("a.xml", TEXT), Member::new("b.xml", SECOND)]);
    assert_eq!(texts.len(), 2);
    assert!(twice.is_empty(), "{twice:?}");
}

#[test]
fn a_name_given_twice_in_a_source_read_along_is_named_under_that_source() {
    let directory = tempfile::tempdir().expect("a directory");
    let delivery = directory.path().join("delivery");
    std::fs::create_dir(&delivery).expect("a directory");
    std::fs::write(delivery.join("line.xml"), TEXT).expect("written");
    let register = directory.path().join("register.zip");
    std::fs::write(
        &register,
        stored(&[
            Member::new("stops.xml", TEXT),
            Member::new("stops.xml", SECOND),
        ]),
    )
    .expect("the archive is written");
    let source = Source::open(&delivery)
        .and_then(|source| source.with(Source::open(&register)?))
        .expect("the sources open");
    assert_eq!(source.names_given_twice(), ["register.zip/stops.xml"]);
    let stops = source
        .entries()
        .expect("the sources list their documents")
        .into_iter()
        .find(|entry| entry.name == "register.zip/stops.xml")
        .expect("listed once");
    assert_eq!(
        source.load(&stops).xml.expect("the document loads"),
        "<PublicationDelivery/>"
    );
}
