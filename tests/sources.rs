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
