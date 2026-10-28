//! What it costs to read a document: the typed model, the lean routing
//! profile, release detection, and the two side checks — one thread each.

// `criterion_group!` expands to a function that has nowhere to carry documentation,
// so the crate's `missing_docs` lint is lifted for the benchmarks.
#![allow(missing_docs)]

mod support;

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use netex_rs::stream::{self, ReadOptions, Selection};

fn read(c: &mut Criterion) {
    let mut group = c.benchmark_group("read");
    for case in support::cases() {
        group.throughput(case.bytes());
        group.bench_with_input(
            BenchmarkId::from_parameter(case.name),
            &case.xml,
            |b, xml| {
                b.iter(|| netex_rs::read(black_box(xml)).expect("the fixture reads"));
            },
        );
    }
    group.finish();

    let mut group = c.benchmark_group("read_lean");
    let options = ReadOptions::new(Selection::routing());
    for case in support::cases() {
        group.throughput(case.bytes());
        group.bench_with_input(
            BenchmarkId::from_parameter(case.name),
            &case.xml,
            |b, xml| {
                b.iter(|| {
                    stream::read(black_box(xml), &options, |entity, _| {
                        black_box(entity);
                        Ok(())
                    })
                    .expect("the fixture streams")
                });
            },
        );
    }
    group.finish();

    let mut group = c.benchmark_group("detect");
    for case in support::cases() {
        group.throughput(case.bytes());
        group.bench_with_input(
            BenchmarkId::from_parameter(case.name),
            &case.xml,
            |b, xml| {
                b.iter(|| netex_rs::version::detect(black_box(xml)));
            },
        );
    }
    group.finish();

    let mut group = c.benchmark_group("duplicate_ids");
    for case in support::cases() {
        group.throughput(case.bytes());
        group.bench_with_input(
            BenchmarkId::from_parameter(case.name),
            &case.xml,
            |b, xml| {
                b.iter(|| netex_rs::check::duplicate_ids_in(black_box(xml)));
            },
        );
    }
    group.finish();
}

criterion_group!(benches, read);
criterion_main!(benches);
