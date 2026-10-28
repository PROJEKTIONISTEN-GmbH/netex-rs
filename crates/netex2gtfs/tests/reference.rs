//! The feed against a GTFS feed of the same stand published beside the NeTEx
//! delivery — DELFI publishes both, so does the Swiss platform —, day by day:
//! trips, stations, times and days, each difference counted with its cause.
//!
//! The comparison itself is tested on a feed against an altered copy of it.
//! Against a national reference it runs only when told where the feeds are:
//!
//! ```sh
//! NETEX2GTFS_OURS=/path/to/our/feed NETEX2GTFS_REFERENCE=/path/to/reference \
//! NETEX2GTFS_DAYS=20261013,20261017,20261018 \
//!   cargo test --release -p netex2gtfs --test reference -- --ignored --nocapture
//! ```
//!
//! Both feeds as directories (unpack a zip archive first). `NETEX2GTFS_RESULT`
//! names a file to write the comparison to as JSON.

#[path = "support/compare.rs"]
mod compare;

use std::path::{Path, PathBuf};

use compare::{Cause, Feed};
use netex2gtfs::{convert, output, Delivery, Options, Reading};
use netex_rs::dataset::Source;

/// The Seeweg delivery's feed, in a directory of its own.
fn seeweg_feed() -> tempfile::TempDir {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/seeweg");
    let delivery =
        Delivery::read(&Source::open(path).expect("opens"), Reading::default()).expect("reads");
    let feed = tempfile::tempdir().expect("a directory");
    convert(
        &delivery,
        &Options::default(),
        output::at(feed.path()).expect("an output"),
    )
    .expect("converts");
    feed
}

fn edit(path: &Path, file: &str, change: impl Fn(&str) -> String) {
    let path = path.join(file);
    let text = std::fs::read_to_string(&path).expect("the file");
    std::fs::write(&path, change(&text)).expect("written");
}

#[test]
fn a_station_is_known_by_its_dhid_its_sloid_or_its_number() {
    assert_eq!(
        compare::dhid_station("de:06412:1502:32:3").as_deref(),
        Some("de:06412:1502")
    );
    assert_eq!(
        compare::dhid_station("at:46:3040").as_deref(),
        Some("at:46:3040")
    );
    assert_eq!(compare::dhid_station("ch:2:StopPlace:8014228"), None);
    assert_eq!(
        compare::dhid_station("de:09671:53113#2").as_deref(),
        Some("de:09671:53113")
    );
    assert_eq!(
        compare::station_number("000005603743").as_deref(),
        Some("5603743")
    );
    assert_eq!(
        compare::station_number("DE::ScheduledStopPoint:5603743_DBDB_::").as_deref(),
        Some("5603743")
    );
    assert_eq!(
        compare::station_number("8503000:0:3").as_deref(),
        Some("8503000")
    );
    assert_eq!(
        compare::station_number("Parent8503000").as_deref(),
        Some("8503000")
    );
    assert_eq!(compare::station_number("ch:1:sloid:3000:3:5"), None);
    // A Swiss SLOID, a quay's or its parent's as the Swiss GTFS writes it.
    for sloid in [
        "ch:1:sloid:3000:3:5",
        "ch:1:sloid:3000",
        "Parentch:1:sloid:3000",
    ] {
        assert_eq!(
            compare::sloid_station(sloid).as_deref(),
            Some("ch:1:sloid:3000"),
            "{sloid}"
        );
    }
    assert_eq!(compare::sloid_station("ch:2:Quay:10:10000"), None);
}

#[test]
fn a_feed_is_alike_to_itself_and_each_change_has_its_cause() {
    let ours = seeweg_feed();
    let days = ["20261013", "20261017"];
    let same = compare::compare(
        &Feed::read(ours.path(), &days),
        &Feed::read(ours.path(), &days),
    );
    let tuesday = &same["20261013"];
    // Three journeys, one of them a template with a headway group in the
    // morning and one in the evening: each group counts once, its runs not
    // being stated.
    assert_eq!(
        (tuesday.ours, tuesday.reference, tuesday.matched),
        (4, 4, 4)
    );
    assert!(tuesday.only_ours.is_empty() && tuesday.only_reference.is_empty());
    assert_eq!(same["20261017"].ours, 0, "a Saturday");

    // The reference: the night trip half a minute later at its last stop,
    // the inbound trip without its middle call, the template's service on
    // Saturdays instead.
    let reference = tempfile::tempdir().expect("a directory");
    for entry in std::fs::read_dir(ours.path()).expect("the feed") {
        let entry = entry.expect("a file");
        std::fs::copy(entry.path(), reference.path().join(entry.file_name())).expect("copied");
    }
    edit(reference.path(), "stop_times.txt", |text| {
        text.replace(
            "sw:ServiceJourney:7-2310,24:10:00,24:10:00",
            "sw:ServiceJourney:7-2310,24:10:30,24:10:30",
        )
        .lines()
        .filter(|line| !line.starts_with("sw:ServiceJourney:7-1200,12:05:00"))
        .map(|line| format!("{line}\n"))
        .collect()
    });
    edit(reference.path(), "trips.txt", |text| {
        text.replace(
            "sw:Line:7,c0,sw:ServiceJourney:7-takt",
            "sw:Line:7,saturdays,sw:ServiceJourney:7-takt",
        )
    });
    edit(reference.path(), "calendar.txt", |text| {
        format!("{text}saturdays,0,0,0,0,0,1,0,20261012,20261023\n")
    });
    let compared = compare::compare(
        &Feed::read(ours.path(), &days),
        &Feed::read(reference.path(), &days),
    );
    let tuesday = &compared["20261013"];
    assert_eq!(
        (tuesday.ours, tuesday.reference, tuesday.matched),
        (4, 2, 0)
    );
    assert_eq!(
        tuesday
            .only_ours
            .iter()
            .map(|(&cause, &count)| (cause, count))
            .collect::<Vec<_>>(),
        [
            (Cause::TimesWithinAMinute, 1),
            (Cause::CallsLeftOut, 1),
            (Cause::OtherDays, 2)
        ]
    );
    let saturday = &compared["20261017"];
    assert_eq!((saturday.ours, saturday.reference), (0, 2));
    assert_eq!(saturday.only_reference.get(&Cause::OtherDays), Some(&2));
    // Weide is served only by the call the reference leaves out.
    assert_eq!(tuesday.stations, (4, 3, 3));
}

#[test]
fn a_place_named_otherwise_in_the_reference_is_still_the_same_place() {
    // The reference names Bergdorf by an internal number, as the German
    // export names some thirty thousand stops, rather than by its identifier.
    let ours = seeweg_feed();
    let reference = tempfile::tempdir().expect("a directory");
    for entry in std::fs::read_dir(ours.path()).expect("the feed") {
        let entry = entry.expect("a file");
        std::fs::copy(entry.path(), reference.path().join(entry.file_name())).expect("copied");
    }
    let internal = |text: &str| {
        text.replace(
            "sw:StopPlace:bergdorf,Bergdorf,47.0600000,8.3200000,0,sw:StopPlace:bergdorf-ort",
            "000004711,Bergdorf,47.0600000,8.3200000,0,",
        )
        .replace(",sw:StopPlace:bergdorf,", ",000004711,")
    };
    edit(reference.path(), "stops.txt", internal);
    edit(reference.path(), "stop_times.txt", internal);
    let days = ["20261013"];
    let compared = compare::compare(
        &Feed::read(ours.path(), &days),
        &Feed::read(reference.path(), &days),
    );
    let tuesday = &compared["20261013"];
    // The inbound trip does not call at Bergdorf; the night trip and the
    // two headway groups of the template do.
    assert_eq!(tuesday.matched, 1);
    assert_eq!(
        tuesday
            .only_ours
            .iter()
            .map(|(&c, &n)| (c, n))
            .collect::<Vec<_>>(),
        [(Cause::StationsNamedOtherwise, 3)]
    );
    assert_eq!(
        tuesday
            .only_reference
            .iter()
            .map(|(&c, &n)| (c, n))
            .collect::<Vec<_>>(),
        [(Cause::StationsNamedOtherwise, 3)]
    );
}

fn environment(name: &str) -> Option<PathBuf> {
    std::env::var_os(name).map(PathBuf::from)
}

#[test]
#[ignore = "needs a national feed and its reference: NETEX2GTFS_OURS, NETEX2GTFS_REFERENCE, NETEX2GTFS_DAYS"]
fn the_feed_against_the_reference_of_the_same_stand() {
    let (Some(ours), Some(reference)) = (
        environment("NETEX2GTFS_OURS"),
        environment("NETEX2GTFS_REFERENCE"),
    ) else {
        eprintln!("SKIPPED: NETEX2GTFS_OURS and NETEX2GTFS_REFERENCE name no feeds");
        return;
    };
    let days =
        std::env::var("NETEX2GTFS_DAYS").expect("NETEX2GTFS_DAYS names the days, YYYYMMDD,…");
    let days: Vec<&str> = days.split(',').collect();
    let ours = Feed::read(&ours, &days);
    let reference = Feed::read(&reference, &days);
    let compared = compare::compare(&ours, &reference);
    for (day, result) in &compared {
        // Every trip of either side is matched or has its cause.
        let accounted =
            |causes: &std::collections::BTreeMap<Cause, u64>| causes.values().sum::<u64>();
        assert_eq!(
            result.matched + accounted(&result.only_ours),
            result.ours,
            "{day}"
        );
        assert_eq!(
            result.matched + accounted(&result.only_reference),
            result.reference,
            "{day}"
        );
        println!(
            "{day}: ours {} reference {} matched {} ({:.1} % of the reference); only ours {:?}; only the reference {:?}; stations {:?}",
            result.ours,
            result.reference,
            result.matched,
            100.0 * result.matched as f64 / result.reference.max(1) as f64,
            result.only_ours,
            result.only_reference,
            result.stations
        );
        for (kind, example) in &result.examples {
            println!("    {kind}: {example:?}");
        }
    }
    if let Some(path) = environment("NETEX2GTFS_RESULT") {
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&compared).expect("JSON"),
        )
        .expect("written");
    }
}
