//! Gathering the index on several threads: a builder per worker, merged at
//! the end, builds the index one builder would have built; and the dataset
//! hands its documents out largest first, one buffer per worker.

mod support;

use std::sync::Mutex;

use rayon::prelude::*;

use netex_rs::dataset::Source;
use netex_rs::index::IndexBuilder;

fn serial_index(fixtures: &[support::Fixture]) -> netex_rs::index::Index {
    let mut builder = IndexBuilder::new();
    for fixture in fixtures {
        let document = netex_rs::read(&fixture.xml).expect("the fixture reads");
        builder.add(&document, &fixture.name);
    }
    builder.finish()
}

#[test]
fn builders_fed_one_after_the_other_merge_to_the_index_one_builder_builds() {
    let fixtures = support::delfi_fixtures();
    let (first, second) = fixtures.split_at(3);
    let mut a = IndexBuilder::new();
    for fixture in first {
        a.add(
            &netex_rs::read(&fixture.xml).expect("the fixture reads"),
            &fixture.name,
        );
    }
    let mut b = IndexBuilder::new();
    for fixture in second {
        b.add(
            &netex_rs::read(&fixture.xml).expect("the fixture reads"),
            &fixture.name,
        );
    }
    a.merge(b);
    let merged = a.finish();
    let serial = serial_index(&fixtures);
    assert_eq!(merged.document_count(), 5);
    assert!(
        merged == serial,
        "the merged index differs from the serially built one: {} vs {} stops, {} vs {} journeys, {} vs {} strings, {} vs {} unresolved",
        merged.stop_count(),
        serial.stop_count(),
        merged.journey_count(),
        serial.journey_count(),
        merged.string_count(),
        serial.string_count(),
        merged.unresolved().len(),
        serial.unresolved().len()
    );
}

#[test]
fn builders_fed_in_turns_merge_to_the_same_entities_and_resolutions() {
    let fixtures = support::delfi_fixtures();
    let mut a = IndexBuilder::new();
    let mut b = IndexBuilder::new();
    for (i, fixture) in fixtures.iter().enumerate() {
        let builder = if i % 2 == 0 { &mut a } else { &mut b };
        builder.add(
            &netex_rs::read(&fixture.xml).expect("the fixture reads"),
            &fixture.name,
        );
    }
    b.merge(a);
    let merged = b.finish();
    let serial = serial_index(&fixtures);
    assert_eq!(merged.stop_count(), serial.stop_count());
    assert_eq!(merged.redefinition_count(), serial.redefinition_count());
    assert_eq!(merged.stop_point_count(), serial.stop_point_count());
    assert_eq!(merged.line_count(), serial.line_count());
    assert_eq!(merged.journey_count(), serial.journey_count());
    assert_eq!(merged.passing_time_count(), serial.passing_time_count());
    assert_eq!(merged.calendar_count(), serial.calendar_count());
    assert_eq!(merged.transfer_count(), serial.transfer_count());
    assert_eq!(merged.interchange_count(), serial.interchange_count());
    assert_eq!(merged.string_count(), serial.string_count());
    assert_eq!(merged.unresolved().len(), serial.unresolved().len());
    // The interchanges of the RE19 file point at journeys of the TER file; they
    // resolve across the merge exactly as they do in one builder.
    for stop in serial.stops() {
        let stop = serial.stop(stop);
        let Some(dhid) = stop.dhid else { continue };
        let merged_stop = merged.stop(
            merged
                .stop_by_dhid(dhid)
                .unwrap_or_else(|| panic!("{dhid} is in the merged index")),
        );
        assert_eq!(merged_stop.name, stop.name, "{dhid}");
        assert_eq!(merged_stop.kind, stop.kind, "{dhid}");
        assert_eq!(merged_stop.level, stop.level, "{dhid}");
        assert_eq!(
            merged
                .transfers_from(merged.stop_by_dhid(dhid).expect("found above"))
                .len(),
            serial
                .transfers_from(serial.stop_by_dhid(dhid).expect("indexed"))
                .len(),
            "walks from {dhid}"
        );
    }
    for line in 0..serial.line_count() {
        let line = netex_rs::index::LineId(line as u32);
        let id = serial.line(line).id;
        let merged_line = merged.line_by_id(id).expect("the line is in both");
        assert_eq!(
            merged.journeys_of_line(merged_line).len(),
            serial.journeys_of_line(line).len(),
            "journeys of {id}"
        );
    }
}

#[test]
fn every_document_is_handed_out_once_with_its_text_largest_first() {
    let dir = support::scratch_dir("gather-largest-first");
    let zip = support::zip_of_delfi_fixtures(&dir);
    for source in [
        Source::open(&zip).expect("the archive opens"),
        Source::open(support::fixtures_dir().join("delfi")).expect("the directory opens"),
    ] {
        let seen = Mutex::new(Vec::new());
        // One thread, so that the order the documents come in is the order
        // they are handed out.
        rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .expect("a pool of one thread")
            .install(|| {
                source
                    .read_each(|entry, text| {
                        let text = text.expect("the document loads");
                        assert!(text.starts_with("<?xml"), "{}", entry.name);
                        assert_eq!(text.len() as u64, entry.size, "{}", entry.name);
                        seen.lock()
                            .expect("not poisoned")
                            .push((entry.name.clone(), entry.size));
                    })
                    .expect("the source lists its documents");
            });
        let seen = seen.into_inner().expect("not poisoned");
        assert_eq!(seen.len(), 5, "{}", source.path().display());
        let sizes: Vec<u64> = seen.iter().map(|(_, size)| *size).collect();
        let mut largest_first = sizes.clone();
        largest_first.sort_unstable_by(|a, b| b.cmp(a));
        assert_eq!(sizes, largest_first, "{}", source.path().display());
        assert!(seen[0].0.contains("TER"), "the 1.1 MB file comes first");
    }
}

#[test]
fn the_parallel_iterator_hands_out_every_document_once_over_one_parsed_archive() {
    let dir = support::scratch_dir("gather-documents");
    let zip = support::zip_of_delfi_fixtures(&dir);
    let source = Source::open(&zip).expect("the archive opens");
    let seen = Mutex::new(Vec::new());
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .expect("a pool of four threads")
        .install(|| {
            source
                .documents()
                .expect("the archive lists its documents")
                .for_each(|loaded| {
                    let xml = loaded.xml.expect("the document loads");
                    assert_eq!(xml.len() as u64, loaded.entry.size, "{}", loaded.entry.name);
                    assert!(
                        netex_rs::read(&xml).is_ok(),
                        "{} reads through the shared archive",
                        loaded.entry.name
                    );
                    seen.lock().expect("not poisoned").push(loaded.entry.name);
                });
        });
    let mut seen = seen.into_inner().expect("not poisoned");
    seen.sort();
    let mut expected: Vec<String> = support::delfi_fixtures()
        .into_iter()
        .map(|f| f.name)
        .collect();
    expected.sort();
    assert_eq!(seen, expected);
}

/// The WEB fixture, and the same document with one of its stop place's two
/// quays taken out: two definitions of one stop place that differ in their
/// quays, as the line files of the German delivery write them when a stop
/// place is served at different quays by different lines.
fn one_stop_place_defined_with_different_quays() -> [(String, &'static str); 2] {
    let full = support::delfi_fixtures()
        .into_iter()
        .find(|f| f.name.contains("WEB-WEB_HB"))
        .expect("the WEB fixture")
        .xml;
    let start = full
        .find("<Quay id=\"DE::Quay:176188_MSTMASTER::\"")
        .expect("the stop place's second quay");
    let end = start + full[start..].find("</Quay>").expect("the quay closes") + "</Quay>".len();
    let without = format!("{}{}", &full[..start], &full[end..]);
    [(without, "without.xml"), (full, "full.xml")]
}

#[test]
fn a_stop_place_defined_twice_keeps_every_quay_whichever_definition_comes_first() {
    const PLACE: &str = "DE::StopPlace:9072118_MASTER::";
    const QUAY: &str = "DE::Quay:176188_MSTMASTER::";
    let [first, second] = one_stop_place_defined_with_different_quays();
    let read = |(xml, name): &(String, &str)| {
        let mut builder = IndexBuilder::new();
        builder.add(&netex_rs::read(xml).expect("the document reads"), name);
        builder
    };
    let one_builder = |order: [&(String, &str); 2]| {
        let mut builder = IndexBuilder::new();
        for (xml, name) in order {
            builder.add(&netex_rs::read(xml).expect("the document reads"), name);
        }
        builder.finish()
    };
    let merged = |order: [&(String, &str); 2]| {
        let mut builder = read(order[0]);
        builder.merge(read(order[1]));
        builder.finish()
    };
    let indexes = [
        ("one builder, without first", one_builder([&first, &second])),
        ("one builder, full first", one_builder([&second, &first])),
        ("merged, without first", merged([&first, &second])),
        ("merged, full first", merged([&second, &first])),
    ];
    let reference = &indexes[1].1;
    for (label, index) in &indexes {
        let quay = index
            .stop_by_id(QUAY)
            .unwrap_or_else(|| panic!("{label}: the quay only one definition has is indexed"));
        assert_eq!(
            index.stop(quay).parent,
            index.stop_by_id(PLACE),
            "{label}: the quay belongs to its stop place"
        );
        assert_eq!(index.stop_count(), reference.stop_count(), "{label}");
        assert_eq!(
            index.redefinition_count(),
            reference.redefinition_count(),
            "{label}"
        );
        assert!(
            index.unresolved().is_empty(),
            "{label}: {:?}",
            index.unresolved().first()
        );
    }
}

/// The WEB fixture, and a second document that contradicts it on the stops
/// both define: the stop place under another name, its second quay under the
/// other stop place, and the stop point of that quay assigned to the first
/// quay instead — the three kinds of contradiction the German delivery holds
/// between its line files.
fn two_documents_contradicting_each_other_on_shared_stops() -> (String, String) {
    let original = support::delfi_fixtures()
        .into_iter()
        .find(|f| f.name.contains("WEB-WEB_HB"))
        .expect("the WEB fixture")
        .xml;
    let quay_start = original
        .find("<Quay id=\"DE::Quay:176188_MSTMASTER::\"")
        .expect("the second stop place's second quay");
    let quay_end = quay_start
        + original[quay_start..]
            .find("</Quay>")
            .expect("the quay closes")
        + "</Quay>".len();
    let quay = &original[quay_start..quay_end];
    let without_quay = format!("{}{}", &original[..quay_start], &original[quay_end..]);
    let first_place = without_quay
        .find("<StopPlace id=\"DE::StopPlace:9016189_MASTER::\"")
        .expect("the first stop place");
    let first_quays_end = first_place
        + without_quay[first_place..]
            .find("</quays>")
            .expect("the first stop place has quays");
    let moved = format!(
        "{}{quay}{}",
        &without_quay[..first_quays_end],
        &without_quay[first_quays_end..]
    );
    let assignment = moved
        .find("<PassengerStopAssignment id=\"DE::PassengerStopAssignment:850276::\"")
        .expect("the assignment of the second quay's stop point");
    let quay_ref = assignment
        + moved[assignment..]
            .find("<QuayRef ref=\"DE::Quay:176188_MSTMASTER::\"")
            .expect("the assignment names the second quay");
    let assigned_elsewhere = format!(
        "{}{}",
        &moved[..quay_ref],
        moved[quay_ref..].replacen("176188", "116188", 1)
    );
    let place = assigned_elsewhere
        .find("<StopPlace id=\"DE::StopPlace:9072118_MASTER::\"")
        .expect("the second stop place");
    let contradicting = format!(
        "{}{}",
        &assigned_elsewhere[..place],
        assigned_elsewhere[place..].replacen(
            "<Name>Osterholz-Scharmbeck-Heilshorn Grundschule</Name>",
            "<Name>Heilshorn Schule</Name>",
            1,
        )
    );
    assert!(
        contradicting != assigned_elsewhere,
        "the stop place is renamed"
    );
    (original, contradicting)
}

#[test]
fn where_documents_contradict_each_other_the_document_named_first_decides_in_any_order() {
    const PLACE: &str = "DE::StopPlace:9072118_MASTER::";
    const OTHER_PLACE: &str = "DE::StopPlace:9016189_MASTER::";
    const QUAY: &str = "DE::Quay:176188_MSTMASTER::";
    const FIRST_QUAY: &str = "DE::Quay:116188_MSTMASTER::";
    const POINT: &str = "DE::ScheduledStopPoint:1618802_WEB_::";
    let (original, contradicting) = two_documents_contradicting_each_other_on_shared_stops();
    let builder_of = |xml: &str, name: &str| {
        let mut builder = IndexBuilder::new();
        builder.add(&netex_rs::read(xml).expect("the document reads"), name);
        builder
    };
    // Which document is named first decides; the order they are read in,
    // one builder or merged, does not.
    for (original_name, contradicting_name, original_decides) in
        [("a.xml", "b.xml", true), ("b.xml", "a.xml", false)]
    {
        let documents = [
            (original.as_str(), original_name),
            (contradicting.as_str(), contradicting_name),
        ];
        let mut indexes = Vec::new();
        for [first, second] in [[documents[0], documents[1]], [documents[1], documents[0]]] {
            let mut one = builder_of(first.0, first.1);
            one.add(
                &netex_rs::read(second.0).expect("the document reads"),
                second.1,
            );
            indexes.push((format!("one builder, {} first", first.1), one.finish()));
            // A builder that saw both, merged on: a quay whose record the
            // second document replaced may belong to a stop place that
            // builder numbered after it.
            let mut both = builder_of(first.0, first.1);
            both.add(
                &netex_rs::read(second.0).expect("the document reads"),
                second.1,
            );
            let mut onward = IndexBuilder::new();
            onward.merge(both);
            indexes.push((
                format!("one builder, {} first, merged on", first.1),
                onward.finish(),
            ));
            let mut merged = builder_of(first.0, first.1);
            merged.merge(builder_of(second.0, second.1));
            indexes.push((format!("merged, {} first", first.1), merged.finish()));
        }
        let (name, parent, assigned) = if original_decides {
            ("Osterholz-Scharmbeck-Heilshorn Grundschule", PLACE, QUAY)
        } else {
            ("Heilshorn Schule", OTHER_PLACE, FIRST_QUAY)
        };
        for (label, index) in &indexes {
            let label = format!("{label}, the original named {original_name}");
            let place = index.stop_by_id(PLACE).expect("the stop place is indexed");
            assert_eq!(index.stop(place).name, name, "{label}");
            let quay = index.stop_by_id(QUAY).expect("the quay is indexed");
            assert_eq!(index.stop(quay).parent, index.stop_by_id(parent), "{label}");
            let point = index
                .stop_point_by_id(POINT)
                .expect("the stop point is indexed");
            assert_eq!(
                index.stop_point(point).stop,
                index.stop_by_id(assigned),
                "{label}"
            );
            assert_eq!(index.stop_count(), indexes[0].1.stop_count(), "{label}");
        }
    }
}
