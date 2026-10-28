//! The regression guards: reading must not get noticeably slower, nor
//! allocate noticeably more, than it does today.
//!
//! Memory is guarded by counting: every allocation of a read is counted, so
//! the figures are exact and the thresholds tight.
//! Speed is guarded by the clock, the best of several runs against a floor
//! well below what this machine does, so that noise never trips it; the
//! speed guards run in release builds only, since a debug build is ten times
//! slower by construction.

mod support;

use std::time::Instant;

use allocation_counter::AllocationInfo;
use netex_rs::stream::{self, ReadOptions, Selection};

/// The 1.1 MB DELFI line file.
fn large() -> String {
    support::delfi_fixtures()
        .into_iter()
        .find(|f| f.name.contains("DBDB-87____-TER"))
        .expect("the large fixture is there")
        .xml
}

fn ratio(bytes: u64, input: &str) -> f64 {
    bytes as f64 / input.len() as f64
}

/// Counts what `run` allocates on this thread.
fn counted<T>(run: impl FnOnce() -> T) -> (T, AllocationInfo) {
    let mut result = None;
    let allocations = allocation_counter::measure(|| result = Some(run()));
    (result.expect("the run returned"), allocations)
}

/// Measured on the large fixture (`--nocapture` prints the figures): the full
/// model allocates 4.77 bytes in 0.024 allocations per input byte and holds
/// 3.54 bytes per input byte at its peak; the routing profile 3.29 bytes in
/// 0.022 allocations, holding 0.13 at its peak. Counting is exact, so the
/// thresholds leave a fifth of headroom and no more: a reader that copies
/// one more string per element trips them.
#[test]
fn the_full_model_does_not_allocate_more_than_it_does_today() {
    let xml = large();
    let (document, allocations) = counted(|| netex_rs::read(&xml).expect("the fixture reads"));
    drop(document);
    println!(
        "full model: {:.3} allocations, {:.2} bytes allocated, {:.2} bytes peak live per input byte ({} allocations)",
        ratio(allocations.count_total, &xml),
        ratio(allocations.bytes_total, &xml),
        ratio(allocations.bytes_max, &xml),
        allocations.count_total
    );
    assert!(
        ratio(allocations.bytes_total, &xml) < 5.7,
        "the full model allocates {:.2} bytes per input byte, more than today's 4.77",
        ratio(allocations.bytes_total, &xml)
    );
    assert!(
        ratio(allocations.count_total, &xml) < 0.029,
        "the full model makes {:.3} allocations per input byte, more than today's 0.024",
        ratio(allocations.count_total, &xml)
    );
    assert!(
        ratio(allocations.bytes_max, &xml) < 4.3,
        "the full model holds {:.2} bytes per input byte at its peak, more than today's 3.54",
        ratio(allocations.bytes_max, &xml)
    );
}

#[test]
fn the_lean_mode_holds_no_more_than_one_entity_at_a_time() {
    let xml = large();
    let options = ReadOptions::new(Selection::routing());
    let ((), allocations) = counted(|| {
        stream::read(&xml, &options, |entity, _| {
            drop(entity);
            Ok(())
        })
        .expect("the fixture streams")
    });
    println!(
        "routing profile: {:.3} allocations, {:.2} bytes allocated, {:.2} bytes peak live per input byte",
        ratio(allocations.count_total, &xml),
        ratio(allocations.bytes_total, &xml),
        ratio(allocations.bytes_max, &xml)
    );
    assert!(
        ratio(allocations.bytes_total, &xml) < 4.0,
        "the routing profile allocates {:.2} bytes per input byte, more than today's 3.29",
        ratio(allocations.bytes_total, &xml)
    );
    assert!(
        ratio(allocations.count_total, &xml) < 0.027,
        "the routing profile makes {:.3} allocations per input byte, more than today's 0.022",
        ratio(allocations.count_total, &xml)
    );
    assert!(
        ratio(allocations.bytes_max, &xml) < 0.2,
        "the routing profile holds {:.2} bytes per input byte at its peak, more than today's 0.13: more than one entity at a time",
        ratio(allocations.bytes_max, &xml)
    );
}

#[test]
fn skipping_everything_copies_nothing() {
    let xml = large();
    let options = ReadOptions::new(Selection::none());
    let ((), allocations) =
        counted(|| stream::read(&xml, &options, |_, _| Ok(())).expect("the fixture streams"));
    println!(
        "nothing selected: {} allocations, {} bytes, over {} bytes of input",
        allocations.count_total,
        allocations.bytes_total,
        xml.len()
    );
    assert!(
        allocations.count_total < 64 && allocations.bytes_total < 64 * 1024,
        "skipping the whole document made {} allocations of {} bytes: something was copied",
        allocations.count_total,
        allocations.bytes_total
    );
}

/// Best of `runs` runs of `read` over `xml`, in megabytes per second.
fn best_throughput(xml: &str, runs: usize, mut read: impl FnMut(&str)) -> f64 {
    (0..runs)
        .map(|_| {
            let start = Instant::now();
            read(xml);
            xml.len() as f64 / 1e6 / start.elapsed().as_secs_f64()
        })
        .fold(0.0, f64::max)
}

/// Whether the speed guards run: they need an optimised build.
fn optimised() -> bool {
    if cfg!(debug_assertions) {
        println!("SKIPPED: debug build; the speed guards run with --release");
        return false;
    }
    true
}

/// Measured on the large fixture in this sandbox, one thread, warm, best of
/// seven: the full model reads it at 144 MB/s, the routing profile at
/// 152 MB/s. The floors are a third of that — the best of seven runs on a
/// slower or a busy machine stays well above them, a reader half as fast as
/// today does not.
#[test]
fn the_full_model_reads_no_slower_than_a_third_of_today() {
    if !optimised() {
        return;
    }
    let xml = large();
    let best = best_throughput(&xml, 7, |xml| {
        std::hint::black_box(netex_rs::read(xml).expect("the fixture reads"));
    });
    println!("full model: best of 7 runs {best:.0} MB/s");
    assert!(
        best > 45.0,
        "the full model reads the large fixture at {best:.0} MB/s, below the 45 MB/s floor"
    );
}

#[test]
fn the_lean_mode_reads_no_slower_than_a_third_of_today() {
    if !optimised() {
        return;
    }
    let xml = large();
    let options = ReadOptions::new(Selection::routing());
    let best = best_throughput(&xml, 7, |xml| {
        stream::read(xml, &options, |entity, _| {
            std::hint::black_box(entity);
            Ok(())
        })
        .expect("the fixture streams");
    });
    println!("routing profile: best of 7 runs {best:.0} MB/s");
    assert!(
        best > 50.0,
        "the routing profile reads the large fixture at {best:.0} MB/s, below the 50 MB/s floor"
    );
}
