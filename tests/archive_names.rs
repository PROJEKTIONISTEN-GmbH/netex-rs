//! A member's name is the publisher's text, not a path: a document is
//! listed under it as written, and loads only where the name is a path of
//! plain parts within its source — what a caller writing documents under
//! their names may join to a directory of its own.

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
