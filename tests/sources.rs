//! Several sources read as one: a delivery with the stop register it names,
//! published apart — a directory and a zip archive here —, their documents
//! listed, loaded and read together, those of the register named under the
//! archive's name.

mod support;

use std::collections::BTreeMap;
use std::sync::Mutex;

use netex_rs::dataset::Source;

/// The Nordic delivery's directory, with its stop register as a zip archive
/// written into `dir`.
fn delivery_and_register(dir: &std::path::Path) -> Source {
    let register = support::zip_of(dir, "register.zip", &support::own_fixtures("nordic-stops"));
    Source::open(support::fixtures_dir().join("own").join("nordic"))
        .and_then(|delivery| delivery.with(Source::open(&register)?))
        .expect("the delivery and its register open")
}

#[test]
fn a_source_read_along_names_its_documents_under_its_own_name() {
    let source = delivery_and_register(&support::scratch_dir("sources-names"));
    let names: Vec<String> = source
        .entries()
        .expect("the sources list their documents")
        .into_iter()
        .map(|entry| entry.name)
        .collect();
    assert_eq!(
        names,
        [
            "EX_EX-Line-1_1_Torget-Eksempelby-bussterminal.xml",
            "EX_EX-Line-R1_R1_Eksempelby-Fjellstua.xml",
            "EX_FlexibleLine-hent_Hent-meg-Eksempelby.xml",
            "_EX_shared_data.xml",
            "register.zip/_stops.xml",
        ]
    );
    let manifest = source.manifest().expect("the sources list their documents");
    assert_eq!(
        manifest.entries.last().map(|e| e.name.as_str()),
        Some("register.zip/_stops.xml")
    );
    assert!(source.path().ends_with("nordic"), "the first source's path");
}

#[test]
fn every_document_of_both_sources_is_read_as_written() {
    let source = delivery_and_register(&support::scratch_dir("sources-read"));
    let mut expected: BTreeMap<String, String> = support::own_fixtures("nordic")
        .into_iter()
        .map(|fixture| (fixture.name, fixture.xml))
        .collect();
    for fixture in support::own_fixtures("nordic-stops") {
        expected.insert(format!("register.zip/{}", fixture.name), fixture.xml);
    }
    let read = Mutex::new(BTreeMap::new());
    source
        .read_each(|entry, text| {
            let text = text.expect("the document loads").to_owned();
            read.lock()
                .expect("not poisoned")
                .insert(entry.name.clone(), text);
        })
        .expect("the sources list their documents");
    assert_eq!(read.into_inner().expect("not poisoned"), expected);
    for entry in source.entries().expect("the sources list their documents") {
        let loaded = source.load(&entry);
        assert_eq!(
            loaded.xml.as_deref().ok(),
            expected.get(&entry.name).map(String::as_str),
            "{}",
            entry.name
        );
    }
}

#[test]
fn a_source_named_as_one_read_along_already_is_refused() {
    let dir = support::scratch_dir("sources-refused");
    let source = delivery_and_register(&dir.join("first"));
    let again = support::zip_of(
        &dir.join("second"),
        "register.zip",
        &support::own_fixtures("nordic-stops"),
    );
    let error = source
        .with(Source::open(&again).expect("the archive opens"))
        .expect_err("two sources read along under one name");
    assert!(
        error
            .to_string()
            .contains("another source read along is named register.zip too"),
        "{error}"
    );
}

/// The names of the documents of a source, and whether each loads.
#[cfg(unix)]
fn names_loading(source: &Source) -> Vec<(String, bool)> {
    source
        .entries()
        .expect("the source lists its documents")
        .iter()
        .map(|entry| (entry.name.clone(), source.load(entry).xml.is_ok()))
        .collect()
}

#[cfg(unix)]
#[test]
fn a_directory_linked_into_itself_is_read_once() {
    let dir = support::scratch_dir("sources-linked-into-itself");
    std::fs::write(dir.join("x.xml"), "<PublicationDelivery/>").expect("written");
    std::os::unix::fs::symlink(".", dir.join("a")).expect("a link to the directory");
    std::os::unix::fs::symlink(".", dir.join("b")).expect("another link to it");
    let source = Source::open(&dir).expect("the directory opens");
    assert_eq!(names_loading(&source), [("x.xml".to_owned(), true)]);
}

#[cfg(unix)]
#[test]
fn a_directory_reached_by_a_link_beside_its_own_name_is_read_once_under_the_first() {
    let dir = support::scratch_dir("sources-linked-beside");
    std::fs::create_dir(dir.join("2026-11-20")).expect("a directory");
    std::fs::write(dir.join("2026-11-20/x.xml"), "<PublicationDelivery/>").expect("written");
    std::os::unix::fs::symlink("2026-11-20", dir.join("current")).expect("a link");
    let source = Source::open(&dir).expect("the directory opens");
    assert_eq!(
        names_loading(&source),
        [("2026-11-20/x.xml".to_owned(), true)]
    );
}

#[cfg(target_os = "linux")]
#[test]
fn a_file_named_other_than_in_utf_8_is_listed_and_refused_for_its_name() {
    use std::os::unix::ffi::OsStrExt;
    let dir = support::scratch_dir("sources-latin-1");
    let name = std::ffi::OsStr::from_bytes(b"caf\xe9.xml");
    std::fs::write(dir.join(name), "<PublicationDelivery/>").expect("written");
    std::fs::write(dir.join("plain.xml"), "<PublicationDelivery/>").expect("written");
    let source = Source::open(&dir).expect("the directory opens");
    let entries = source.entries().expect("the directory lists its documents");
    assert_eq!(entries.len(), 2);
    let error = source
        .load(&entries[0])
        .xml
        .expect_err("a name no text can hold");
    let netex_rs::error::Kind::DocumentName { name, reason } = error.kind else {
        panic!("refused for another reason: {error}");
    };
    assert_eq!(name, "caf\u{fffd}.xml");
    assert!(reason.contains("UTF-8"), "{reason}");
    assert!(source.load(&entries[1]).xml.is_ok());
}

#[test]
fn an_entry_of_another_source_is_refused_rather_than_read_as_another_document() {
    let dir = support::scratch_dir("sources-foreign-entry");
    let fixtures = support::own_fixtures("nordic-stops");
    let one = Source::open(support::zip_of(&dir, "one.zip", &fixtures)).expect("opens");
    let mut renamed: Vec<support::Fixture> = support::own_fixtures("nordic-stops");
    for fixture in &mut renamed {
        fixture.name = format!("other-{}", fixture.name);
    }
    let other = Source::open(support::zip_of(&dir, "other.zip", &renamed)).expect("opens");
    let entry = &one.entries().expect("the archive lists its documents")[0];
    let error = other
        .load(entry)
        .xml
        .expect_err("the other archive holds no document of that name");
    assert!(
        error.to_string().contains("not a document of this source"),
        "{error}"
    );
    let combined = delivery_and_register(&dir.join("combined"));
    let register = combined
        .entries()
        .expect("the sources list their documents")
        .pop()
        .expect("the register's document");
    assert!(
        one.load(&register).xml.is_err(),
        "an entry of a part this source does not have"
    );
}
