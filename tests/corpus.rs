//! The corpus test: every file of the German national NeTEx delivery reads.
//!
//! The delivery is 28 176 line files in one zip archive, 78 GB inflated, and
//! is not in the repository. The test reads it from the path in
//! `NETEX_DELFI_ZIP` and is skipped — visibly — when that is not set. Every file that fails to read is a finding with its path,
//! line and element. One file in a hundred is also written back and compared
//! to the original element by element.

mod support;

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use netex_rs::dataset::Source;
use netex_rs::Limits;
use rayon::prelude::*;
use support::{compare, parse};

/// The delivery, if it is there — and if this is a release build: reading
/// 78 GB through an unoptimised build takes hours, so a debug build skips the
/// corpus unless `NETEX_CORPUS_DEBUG` is set.
fn corpus() -> Option<Source> {
    if cfg!(debug_assertions) && std::env::var_os("NETEX_CORPUS_DEBUG").is_none() {
        println!("SKIPPED: debug build; run with --release (or set NETEX_CORPUS_DEBUG)");
        return None;
    }
    let Some(path) = std::env::var_os("NETEX_DELFI_ZIP") else {
        println!("SKIPPED: NETEX_DELFI_ZIP does not name a DELFI delivery");
        return None;
    };
    assert!(
        Path::new(&path).exists(),
        "NETEX_DELFI_ZIP names {}, which is not there",
        Path::new(&path).display()
    );
    Some(Source::open(&path).expect("the delivery opens"))
}

/// Peak resident set size of this process, in kilobytes, from `/proc`.
fn peak_rss_kb() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    status
        .lines()
        .find(|line| line.starts_with("VmHWM:"))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}

#[derive(Default)]
struct Tally {
    files: usize,
    bytes: u64,
    failures: Vec<String>,
    round_trips: usize,
}

impl Tally {
    fn merge(mut self, other: Tally) -> Tally {
        self.files += other.files;
        self.bytes += other.bytes;
        self.failures.extend(other.failures);
        self.round_trips += other.round_trips;
        self
    }
}

#[test]
fn every_file_of_the_delfi_delivery_reads() {
    let Some(source) = corpus() else { return };
    let limits = Limits::default();
    let counter = AtomicUsize::new(0);
    let start = Instant::now();
    let tally = source
        .documents()
        .expect("the delivery lists its documents")
        .map(|loaded| {
            let mut tally = Tally {
                files: 1,
                bytes: loaded.entry.size,
                ..Tally::default()
            };
            let document = match loaded.read(&limits) {
                Ok(document) => document,
                Err(error) => {
                    tally
                        .failures
                        .push(format!("{}: {error}", loaded.entry.name));
                    return tally;
                }
            };
            if counter.fetch_add(1, Ordering::Relaxed) % 100 == 0 {
                tally.round_trips = 1;
                let xml = loaded
                    .xml
                    .as_ref()
                    .expect("the document was read from its text");
                if let Err(difference) = compare(&parse(xml), &parse(&document.write())) {
                    tally
                        .failures
                        .push(format!("{}: round trip: {difference}", loaded.entry.name));
                }
            }
            tally
        })
        .reduce(Tally::default, Tally::merge);
    let elapsed = start.elapsed();
    println!(
        "{} files, {:.1} GB inflated, read in {:.1} s on {} threads ({:.0} MB/s); {} round-tripped; peak RSS {} MB",
        tally.files,
        tally.bytes as f64 / 1e9,
        elapsed.as_secs_f64(),
        rayon::current_num_threads(),
        tally.bytes as f64 / 1e6 / elapsed.as_secs_f64(),
        tally.round_trips,
        peak_rss_kb().map_or(0, |kb| kb / 1024)
    );
    assert!(tally.files > 0, "the delivery holds no documents");
    assert!(
        tally.failures.is_empty(),
        "{} of {} files failed:\n{}",
        tally.failures.len(),
        tally.files,
        tally.failures.join("\n")
    );
}
