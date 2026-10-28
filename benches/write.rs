//! What it costs to write a document: as it is, and rewritten for the other
//! release with its loss report.

#![allow(missing_docs)]

mod support;

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use netex_rs::Release;

fn write(c: &mut Criterion) {
    let mut group = c.benchmark_group("write");
    for case in support::cases() {
        let document = netex_rs::read(&case.xml).expect("the fixture reads");
        group.throughput(case.bytes());
        group.bench_with_input(
            BenchmarkId::from_parameter(case.name),
            &document,
            |b, document| {
                b.iter(|| black_box(document).write());
            },
        );
    }
    group.finish();

    let mut group = c.benchmark_group("write_for_2.0");
    for case in support::cases() {
        let document = netex_rs::read(&case.xml).expect("the fixture reads");
        group.throughput(case.bytes());
        group.bench_with_input(
            BenchmarkId::from_parameter(case.name),
            &document,
            |b, document| {
                b.iter(|| black_box(document).write_for(Release::V2));
            },
        );
    }
    group.finish();
}

criterion_group!(benches, write);
criterion_main!(benches);
