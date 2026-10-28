//! What it costs to index documents: gathering entities and resolving
//! references over the DELFI fixtures, and the five fixtures indexed as a
//! dataset in the full and in the lean mode — one thread each.

#![allow(missing_docs)]

mod support;

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use netex_rs::index::IndexBuilder;
use netex_rs::stream::{ReadOptions, Selection};

fn index(c: &mut Criterion) {
    let mut group = c.benchmark_group("index");
    for case in support::cases() {
        let document = netex_rs::read(&case.xml).expect("the fixture reads");
        group.throughput(case.bytes());
        group.bench_with_input(
            BenchmarkId::from_parameter(case.name),
            &document,
            |b, document| {
                b.iter(|| {
                    let mut builder = IndexBuilder::new();
                    builder.add(black_box(document), case.name);
                    builder.finish()
                });
            },
        );
    }
    group.finish();
}

/// Every DELFI line file among the fixtures, by name, in name order.
fn delfi_fixtures() -> Vec<(String, String)> {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/delfi");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("the fixture directory is readable")
        .map(|entry| {
            entry
                .expect("a readable entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .filter(|name| name.ends_with(".xml"))
        .collect();
    names.sort();
    names
        .into_iter()
        .map(|name| {
            let xml = std::fs::read_to_string(dir.join(&name)).expect("the fixture is readable");
            (name, xml)
        })
        .collect()
}

/// The five DELFI fixtures, read and indexed one after the other: through the
/// full model, and lean through the routing profile.
fn dataset(c: &mut Criterion) {
    let fixtures = delfi_fixtures();
    let bytes: usize = fixtures.iter().map(|(_, xml)| xml.len()).sum();
    let mut group = c.benchmark_group("dataset");
    group.throughput(Throughput::Bytes(bytes as u64));
    group.bench_function("full", |b| {
        b.iter(|| {
            let mut builder = IndexBuilder::new();
            for (name, xml) in &fixtures {
                let document = netex_rs::read(black_box(xml)).expect("the fixture reads");
                builder.add(&document, name);
            }
            builder.finish()
        });
    });
    let options = ReadOptions::new(Selection::routing());
    group.bench_function("lean", |b| {
        b.iter(|| {
            let mut builder = IndexBuilder::new();
            for (name, xml) in &fixtures {
                builder
                    .add_streamed(black_box(xml), &options, name)
                    .expect("the fixture streams");
            }
            builder.finish()
        });
    });
    group.finish();
}

criterion_group!(benches, index, dataset);
criterion_main!(benches);
