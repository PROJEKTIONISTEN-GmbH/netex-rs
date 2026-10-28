//! Reads a whole dataset, builds the index, and reports what it cost: the
//! measurement over the German national delivery.
//!
//! ```sh
//! cargo run --release --example index_delivery -- /path/to/netex.zip [--lean] [--referrers] [--since /path/to/previous.zip]
//! ```
//!
//! Documents are read on every thread of the rayon pool (`RAYON_NUM_THREADS`),
//! largest first, each thread gathering into a builder of its own; the
//! builders are merged once every document is in. Without `--lean` every
//! document is read into the full typed model first; with it, the index is
//! built from the lean stream of the routing profile, no document model in
//! memory. With `--since`, the delivery is compared to the previous one by
//! fingerprint — a document being the same document when its name is the
//! same once the date the German delivery puts in every name is taken off —
//! and only its new and changed documents are read, lean. With `--referrers`,
//! the lean index keeps who refers to each object as well
//! ([`Index::referrers`]), and the report counts the objects and references.
//! Peak memory is the process's high-water mark from `/proc/self/status`.
//!
//! The report ends with a fingerprint of the index's content, taken over
//! identifiers rather than the index's own numbering: two runs that read the
//! documents in different orders — on a different number of threads — build
//! the same index exactly when they print the same fingerprint.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Mutex;
use std::time::Instant;

use netex_rs::dataset::{Entry, Source};
use netex_rs::index::{Concern, Index, IndexBuilder};
use netex_rs::model::Date;
use netex_rs::stream::{ReadOptions, Selection};
use netex_rs::Limits;

fn peak_rss_mb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|status| {
            status
                .lines()
                .find(|line| line.starts_with("VmHWM:"))?
                .split_whitespace()
                .nth(1)?
                .parse::<u64>()
                .ok()
        })
        .map_or(0, |kb| kb / 1024)
}

/// What one worker gathered: its builder and its tallies.
struct Worker {
    builder: IndexBuilder,
    files: usize,
    bytes: u64,
}

/// The full path: every document into the typed model, then into the worker's
/// builder.
fn full(source: &Source) -> (Index, Vec<String>) {
    let limits = Limits::default();
    let failures = Mutex::new(Vec::new());
    let start = Instant::now();
    let workers = source
        .read_each_with(
            || Worker {
                builder: IndexBuilder::new(),
                files: 0,
                bytes: 0,
            },
            |worker, entry, text| {
                worker.files += 1;
                worker.bytes += entry.size;
                match text.and_then(|xml| netex_rs::read_with(xml, &limits)) {
                    Ok(document) => worker.builder.add(&document, &entry.name),
                    Err(error) => failures
                        .lock()
                        .expect("not poisoned")
                        .push(format!("{}: {error}", entry.name)),
                }
            },
        )
        .expect("the source lists its documents");
    let read_seconds = start.elapsed().as_secs_f64();
    let read_peak = peak_rss_mb();
    let (files, bytes) = workers
        .iter()
        .fold((0, 0), |(f, b), w| (f + w.files, b + w.bytes));
    println!("files: {files}");
    println!("bytes inflated: {bytes}");
    println!(
        "read + gather: {read_seconds:.1} s on {} threads ({:.0} MB/s), peak RSS {read_peak} MB",
        workers.len(),
        bytes as f64 / 1e6 / read_seconds
    );
    println!(
        "interned strings across the workers before the merge: {}",
        workers
            .iter()
            .map(|w| w.builder.string_count())
            .sum::<usize>()
    );
    let merging = Instant::now();
    let builder = IndexBuilder::merge_all(workers.into_iter().map(|w| w.builder).collect());
    println!("merge: {:.1} s", merging.elapsed().as_secs_f64());
    let resolving = Instant::now();
    let index = builder.finish();
    println!(
        "resolve + index: {:.1} s, peak RSS {} MB",
        resolving.elapsed().as_secs_f64(),
        peak_rss_mb()
    );
    println!("total: {:.1} s", start.elapsed().as_secs_f64());
    (index, failures.into_inner().expect("not poisoned"))
}

/// The lean path: the routing profile streamed into the workers' builders,
/// over `entries` of the source, keeping who refers to each object where
/// `referrers` says so.
fn lean(source: &Source, entries: Vec<Entry>, referrers: bool) -> Index {
    let bytes: u64 = entries.iter().map(|e| e.size).sum();
    println!("files: {}", entries.len());
    println!("bytes inflated: {bytes}");
    let start = Instant::now();
    let options = ReadOptions::new(Selection::routing());
    let index = if referrers {
        let workers = source
            .read_entries_with(
                entries,
                || IndexBuilder::new().with_referrers(),
                |builder, entry, text| {
                    text.and_then(|xml| builder.add_streamed(xml, &options, &entry.name))
                        .expect("the document reads");
                },
            )
            .expect("the source lists its documents");
        IndexBuilder::merge_all(workers).finish()
    } else {
        Index::build_from(source, entries, &options).expect("the delivery indexes")
    };
    let seconds = start.elapsed().as_secs_f64();
    println!(
        "lean read + gather + merge + resolve: {seconds:.1} s on {} threads ({:.0} MB/s), peak RSS {} MB",
        rayon::current_num_threads(),
        bytes as f64 / 1e6 / seconds,
        peak_rss_mb()
    );
    println!("total: {seconds:.1} s");
    index
}

/// A document's name without the publication date the German delivery puts
/// before the extension: `…-4111_20260919.xml` is the document `…-4111`
/// whichever week it was published.
fn without_date(name: &str) -> &str {
    name.strip_suffix(".xml")
        .and_then(|stem| stem.rsplit_once('_'))
        .filter(|(_, date)| date.len() == 8 && date.bytes().all(|b| b.is_ascii_digit()))
        .map_or(name, |(document, _)| document)
}

/// The entries of `source` that are new or changed since `previous`.
fn changed_since(source: &Source, previous: &str) -> Vec<Entry> {
    let start = Instant::now();
    let before = Source::open(previous)
        .expect("the previous delivery opens")
        .manifest()
        .expect("the previous delivery lists its documents");
    let now = source.manifest().expect("the delivery lists its documents");
    let changes = now.diff_by(&before, without_date);
    println!(
        "since {previous}, by name without its date: {} added, {} changed, {} removed, {} unchanged of {} ({:.1} s to compare, nothing inflated)",
        changes.added.len(),
        changes.changed.len(),
        changes.removed.len(),
        changes.unchanged.len(),
        now.entries.len(),
        start.elapsed().as_secs_f64()
    );
    let to_read: std::collections::HashSet<&str> = changes.to_read().collect();
    source
        .entries()
        .expect("the delivery lists its documents")
        .into_iter()
        .filter(|entry| to_read.contains(entry.name.as_str()))
        .collect()
}

/// The hash of one item of the index.
fn hash_of(item: impl Hash) -> u64 {
    let mut hasher = DefaultHasher::new();
    item.hash(&mut hasher);
    hasher.finish()
}

/// A fingerprint of everything the index holds, independent of the order
/// its entities were numbered in: every stop, stop point, line, journey, walk,
/// interchange and finding is hashed with the identifiers it refers to, and the
/// hashes are summed. Coordinates are hashed by their bits; of a journey, the
/// document defining it and every day it runs on; of a stop's DHID, whether the
/// lookup by DHID finds this stop.
fn content_fingerprint(index: &Index) -> u64 {
    use netex_rs::index::{JourneyId, LineId, StopId, StopPointId};
    let stop_id = |stop: StopId| index.stop(stop).id;
    let stop_point_id = |point: StopPointId| index.stop_point(point).id;
    let journey_id = |journey: JourneyId| index.journey(journey).id;
    let coordinate = |value: Option<f64>| value.map(f64::to_bits);
    let mut sum = 0u64;
    let mut add = |item: u64| sum = sum.wrapping_add(item);
    for stop in index.stops() {
        let s = index.stop(stop);
        add(hash_of((
            "stop",
            s.id,
            s.dhid,
            s.name,
            s.kind == netex_rs::index::StopKind::Quay,
            coordinate(s.latitude),
            coordinate(s.longitude),
            s.parent.map(stop_id),
            s.level,
            s.dhid.map(|dhid| index.stop_by_dhid(dhid) == Some(stop)),
        )));
        for walk in index.transfers_from(stop) {
            add(hash_of((
                "walk",
                stop_id(walk.from),
                stop_id(walk.to),
                walk.seconds,
                walk.both_ways,
            )));
        }
    }
    for point in (0..index.stop_point_count()).map(|i| StopPointId(i as u32)) {
        let p = index.stop_point(point);
        add(hash_of((
            "stop point",
            p.id,
            p.name,
            coordinate(p.latitude),
            coordinate(p.longitude),
            p.stop.map(stop_id),
        )));
        for interchange in index.interchanges_at(point) {
            add(hash_of((
                "interchange",
                journey_id(interchange.from_journey),
                journey_id(interchange.to_journey),
                stop_point_id(interchange.from_stop_point),
                stop_point_id(interchange.to_stop_point),
                interchange.stay_seated,
                interchange.guaranteed,
                interchange.maximum_wait_seconds,
            )));
        }
    }
    for line in (0..index.line_count()).map(|i| LineId(i as u32)) {
        let l = index.line(line);
        add(hash_of(("line", l.id, l.name, l.public_code, l.mode)));
    }
    for journey in (0..index.journey_count()).map(|i| JourneyId(i as u32)) {
        let j = index.journey(journey);
        let calls: Vec<(&str, Option<u32>, Option<u32>)> = j
            .stop_points
            .iter()
            .zip(j.passing_times)
            .map(|(point, time)| (stop_point_id(*point), time.arrival, time.departure))
            .collect();
        add(hash_of((
            "journey",
            j.id,
            j.document,
            j.line.map(|line| index.line(line).id),
            j.pattern,
            calls,
            index.days_of(journey).collect::<Vec<i64>>(),
        )));
    }
    for finding in index.findings() {
        add(hash_of(("finding", finding)));
    }
    for unresolved in index.unresolved() {
        add(hash_of(("unresolved", unresolved.to_string())));
    }
    sum
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let path = args
        .iter()
        .find(|a| !a.starts_with("--"))
        .expect("usage: index_delivery <zip or directory> [--lean] [--since <previous>]");
    let source = Source::open(path).expect("the source opens");
    let previous = args
        .iter()
        .position(|a| a == "--since")
        .and_then(|at| args.get(at + 1));
    let referrers = args.iter().any(|a| a == "--referrers");
    let index = if let Some(previous) = previous {
        let entries = changed_since(&source, previous);
        lean(&source, entries, referrers)
    } else if referrers || args.iter().any(|a| a == "--lean") {
        let entries = source.entries().expect("the source lists its documents");
        lean(&source, entries, referrers)
    } else {
        let (index, failures) = full(&source);
        println!("documents that failed to read: {}", failures.len());
        for failure in failures.iter().take(10) {
            println!("  {failure}");
        }
        index
    };
    println!(
        "stops: {} (redefinitions left out: {})",
        index.stop_count(),
        index.redefinition_count()
    );
    println!("stop points: {}", index.stop_point_count());
    println!("lines: {}", index.line_count());
    println!("journeys: {}", index.journey_count());
    println!("passing times: {}", index.passing_time_count());
    println!("calendars: {}", index.calendar_count());
    println!("walks (a two-way walk twice): {}", index.transfer_count());
    println!("interchanges: {}", index.interchange_count());
    println!("interned strings: {}", index.string_count());
    if let Some(referrers) = index.referrers() {
        println!(
            "objects referred to or referring: {}, references between them: {}, unreferenced: {}",
            referrers.object_count(),
            referrers.reference_count(),
            referrers.unreferenced().count()
        );
    }
    println!("unresolved references: {}", index.unresolved().len());
    let mut by_element = std::collections::BTreeMap::new();
    for unresolved in index.unresolved() {
        *by_element
            .entry(unresolved.element.as_str())
            .or_insert(0u64) += 1;
    }
    for (element, count) in by_element {
        println!("  {element}: {count}");
    }
    for unresolved in index.unresolved().iter().take(5) {
        println!("  e.g. {unresolved}");
    }
    let stops_with_dhid = index
        .stops()
        .filter(|stop| index.stop(*stop).dhid.is_some())
        .count();
    println!("stops with a DHID: {stops_with_dhid}");
    let day = Date::from("2026-10-01");
    let running = (0..index.line_count())
        .map(|line| {
            index
                .journeys_of_line_on(netex_rs::index::LineId(line as u32), &day)
                .count()
        })
        .sum::<usize>();
    println!("journeys running on {day}: {running}");
    println!("findings:");
    for &concern in Concern::ALL {
        println!("  {concern:?}: {}", index.finding_count(concern));
    }
    println!("content fingerprint: {:016x}", content_fingerprint(&index));
}
