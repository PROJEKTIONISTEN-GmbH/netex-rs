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

#[cfg(unix)]
#[test]
fn a_directory_lists_its_documents_without_reading_them() {
    use std::os::unix::fs::PermissionsExt;
    let dir = support::scratch_dir("sources-listed-unread");
    std::fs::write(dir.join("open.xml"), "<PublicationDelivery/>").expect("written");
    let closed = dir.join("closed.xml");
    std::fs::write(
        &closed,
        "<PublicationDelivery><!-- closed --></PublicationDelivery>",
    )
    .expect("written");
    std::fs::set_permissions(&closed, std::fs::Permissions::from_mode(0o000))
        .expect("the file is closed");
    if std::fs::read(&closed).is_ok() {
        // A reader that may read any file cannot show what is not read.
        return;
    }
    let source = Source::open(&dir).expect("the directory opens");
    let entries = source
        .entries()
        .expect("listing a directory reads none of its files");
    let sizes: Vec<(&str, u64)> = entries
        .iter()
        .map(|entry| (entry.name.as_str(), entry.size))
        .collect();
    assert_eq!(sizes, [("closed.xml", 58), ("open.xml", 22)]);
    let read = Mutex::new(BTreeMap::new());
    source
        .read_each(|entry, text| {
            read.lock()
                .expect("not poisoned")
                .insert(entry.name.clone(), text.is_ok());
        })
        .expect("the directory lists its documents");
    assert_eq!(
        read.into_inner().expect("not poisoned"),
        BTreeMap::from([
            ("closed.xml".to_owned(), false),
            ("open.xml".to_owned(), true)
        ]),
        "the file that cannot be read is that document's error, the others are read"
    );
    let error = source
        .manifest()
        .expect_err("a fingerprint reads the file it is of");
    assert!(error.to_string().contains("closed.xml"), "{error}");
}

/// A directory of `count` documents, each larger than the next: `0.xml` the
/// largest, read first.
fn documents_largest_first(test: &str, count: usize) -> Source {
    let dir = support::scratch_dir(test);
    for number in 0..count {
        let padding = " ".repeat(10 * (count - number));
        std::fs::write(
            dir.join(format!("{number}.xml")),
            format!("<PublicationDelivery/>{padding}"),
        )
        .expect("written");
    }
    Source::open(&dir).expect("the directory opens")
}

#[test]
fn a_failing_document_stops_every_reader_from_reading_another() {
    let source = documents_largest_first("sources-stopped", 40);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .expect("a pool");
    let handed = std::sync::atomic::AtomicUsize::new(0);
    let outcome = pool.install(|| {
        source.try_read_each_with(
            || (),
            |(), entry, text| {
                handed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                text?;
                if entry.name == "0.xml" {
                    return Err(netex_rs::Error::new(netex_rs::error::Kind::Io(format!(
                        "{} fails",
                        entry.name
                    ))));
                }
                Ok(())
            },
        )
    });
    let error = outcome.expect_err("the first document fails the read");
    assert_eq!(
        error.kind,
        netex_rs::error::Kind::Io("0.xml fails".to_owned())
    );
    assert_eq!(
        handed.into_inner(),
        1,
        "no document is loaded or handed over after the one that failed"
    );
}

#[test]
fn of_several_failing_documents_the_one_read_first_is_the_error_on_any_number_of_threads() {
    let source = documents_largest_first("sources-stopped-first", 40);
    let entries = source.entries().expect("the directory lists its documents");
    for threads in [1, 2, 8] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .expect("a pool");
        for _ in 0..20 {
            let outcome: Result<Vec<()>, String> = pool.install(|| {
                source.try_read_entries_with(
                    entries.clone(),
                    || (),
                    |(), entry, text| {
                        text.map_err(|error| error.to_string())?;
                        // Every document but the largest fails.
                        if entry.name == "0.xml" {
                            Ok(())
                        } else {
                            Err(entry.name.clone())
                        }
                    },
                )
            });
            assert_eq!(
                outcome,
                Err("1.xml".to_owned()),
                "{threads} threads: the second largest document is the first to fail"
            );
        }
    }
}

#[test]
fn readers_that_do_not_fail_return_their_states_and_every_document() {
    let source = documents_largest_first("sources-not-stopped", 12);
    let states = source
        .try_read_each_with(Vec::new, |names: &mut Vec<String>, entry, text| {
            text?;
            names.push(entry.name.clone());
            Ok::<(), netex_rs::Error>(())
        })
        .expect("nothing fails");
    let mut names: Vec<String> = states.into_iter().flatten().collect();
    names.sort_by_key(|name| {
        name.trim_end_matches(".xml")
            .parse::<u32>()
            .expect("a number")
    });
    let expected: Vec<String> = (0..12).map(|number| format!("{number}.xml")).collect();
    assert_eq!(names, expected);
}
