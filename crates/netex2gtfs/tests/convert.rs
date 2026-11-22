//! The feed of a delivery, file by file, read back as a consumer reads it.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use netex2gtfs::{
    convert, output, Delivery, Options, Profile, Publisher, Reading, Report, StopIds,
};
use netex_rs::dataset::Source;

/// A row of a CSV file, by column name.
type Row = BTreeMap<String, String>;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

/// The rows of `file` in the feed at `feed`, in the order written.
fn rows(feed: &Path, file: &str) -> Vec<Row> {
    let mut reader =
        csv::Reader::from_path(feed.join(file)).unwrap_or_else(|error| panic!("{file}: {error}"));
    let header = reader.headers().expect("a header").clone();
    reader
        .records()
        .map(|record| {
            let record = record.expect("a row");
            header
                .iter()
                .zip(record.iter())
                .map(|(name, value)| (name.to_owned(), value.to_owned()))
                .collect()
        })
        .collect()
}

/// The values of `columns` of every row, each row one tuple joined by `|`.
fn table(feed: &Path, file: &str, columns: &[&str]) -> Vec<String> {
    rows(feed, file)
        .iter()
        .map(|row| {
            columns
                .iter()
                .map(|column| row.get(*column).map_or("<missing>", String::as_str))
                .collect::<Vec<_>>()
                .join("|")
        })
        .collect()
}

/// Converts the delivery at `path` into a directory, returning it with the report.
fn convert_to_directory(path: &Path, options: &Options) -> (tempfile::TempDir, Report) {
    let source = Source::open(path).expect("the delivery opens");
    let delivery = Delivery::read(
        &source,
        Reading {
            geometry: options.shapes,
            ..Reading::default()
        },
    )
    .expect("the delivery reads");
    let feed = tempfile::tempdir().expect("a directory");
    let report = convert(
        &delivery,
        options,
        output::at(feed.path()).expect("an output"),
    )
    .expect("converts");
    (feed, report)
}

/// The options of a conversion writing shapes.
fn shapes() -> Options {
    let mut options = Options::default();
    options.shapes = true;
    options
}

fn seeweg() -> (tempfile::TempDir, Report) {
    convert_to_directory(&fixture("seeweg"), &shapes())
}

#[test]
fn the_agency_is_the_operator_of_the_line_with_its_address_and_time_zone() {
    let (feed, _) = seeweg();
    assert_eq!(
        table(feed.path(), "agency.txt", &["agency_id", "agency_name", "agency_url", "agency_timezone", "agency_phone"]),
        ["sw:Operator:SWB|Seeweg Bahnen|https://bahnen.seeweg.example|Europe/Zurich|+41 41 000 00 00"]
    );
    assert_eq!(
        table(
            feed.path(),
            "routes.txt",
            &[
                "route_id",
                "agency_id",
                "route_short_name",
                "route_long_name",
                "route_type",
                "route_color",
                "route_text_color"
            ]
        ),
        ["sw:Line:7|sw:Operator:SWB|7|Seeweg|3|1F6FB2|FFFFFF"]
    );
}

#[test]
fn quays_stand_under_their_stop_place_and_every_call_has_a_placed_stop() {
    let (feed, report) = seeweg();
    assert_eq!(
        table(
            feed.path(),
            "stops.txt",
            &["stop_id", "stop_name", "stop_lat", "stop_lon", "location_type", "parent_station", "wheelchair_boarding", "platform_code", "level_id"]
        ),
        // The stations first, then the stops, each in the order of their
        // identifiers.
        [
            // The parent site of Bergdorf, which is called at itself.
            "sw:StopPlace:bergdorf-ort|Bergdorf Ort|47.0601000|8.3202000|1||0||",
            "sw:StopPlace:seehof|Seehof|47.0500000|8.3000000|1||0||",
            "sw:StopPlace:talwil|Talwil|47.0700000|8.3400000|1||0||",
            "sw:Quay:seehof-1|Seehof|47.0501000|8.3001000|0|sw:StopPlace:seehof|1|1|sw:Level:seehof-0",
            // Without a position of its own: at its stop place.
            "sw:Quay:seehof-2|Seehof|47.0500000|8.3000000|0|sw:StopPlace:seehof|0|2|",
            "sw:Quay:talwil-1|Talwil|47.0701000|8.3401000|0|sw:StopPlace:talwil|0||",
            // Called at itself, without quays: a stop, under its parent site.
            "sw:StopPlace:bergdorf|Bergdorf|47.0600000|8.3200000|0|sw:StopPlace:bergdorf-ort|2||",
            // Assigned to no stop, but with a position of its own.
            "sw:ScheduledStopPoint:weide|Weide|47.0650000|8.3300000|0||0||",
        ]
    );
    assert_eq!(
        table(
            feed.path(),
            "levels.txt",
            &["level_id", "level_index", "level_name"]
        ),
        ["sw:Level:seehof-0|0|Strassenebene"]
    );
    let decision = |code: &str| {
        report
            .decisions
            .iter()
            .find(|decision| decision.code == code)
            .map(|decision| (decision.count, decision.example.clone().unwrap_or_default()))
    };
    assert_eq!(
        decision("call-without-place"),
        Some((
            1,
            "sw:ScheduledStopPoint:nirgends in sw:ServiceJourney:7-1200".to_owned()
        ))
    );
    assert_eq!(
        decision("quay-at-its-stop-place"),
        Some((1, "sw:Quay:seehof-2".to_owned()))
    );
    assert_eq!(
        decision("stop-point-as-stop"),
        Some((1, "sw:ScheduledStopPoint:weide".to_owned()))
    );
}

#[test]
fn trips_show_where_they_go_and_call_with_their_times_past_midnight() {
    let (feed, _) = seeweg();
    assert_eq!(
        table(
            feed.path(),
            "trips.txt",
            &[
                "trip_id",
                "route_id",
                "service_id",
                "trip_headsign",
                "trip_short_name",
                "direction_id",
                "shape_id"
            ]
        ),
        [
            "sw:ServiceJourney:7-2310|sw:Line:7|c0|Bergdorf|2310|0|s0",
            "sw:ServiceJourney:7-1200|sw:Line:7|c0|||1|",
            "sw:ServiceJourney:7-takt|sw:Line:7|c0|Bergdorf||0|s0",
        ]
    );
    assert_eq!(
        table(
            feed.path(),
            "stop_times.txt",
            &[
                "trip_id",
                "stop_sequence",
                "stop_id",
                "arrival_time",
                "departure_time",
                "pickup_type",
                "drop_off_type",
                "stop_headsign"
            ]
        ),
        [
            "sw:ServiceJourney:7-2310|0|sw:Quay:seehof-1|23:50:00|23:50:00|0|1|",
            // The display changes here: shown from here on.
            "sw:ServiceJourney:7-2310|1|sw:StopPlace:bergdorf|23:58:00|24:01:00|0|0|Talwil Mitte",
            "sw:ServiceJourney:7-2310|2|sw:Quay:talwil-1|24:10:00|24:10:00|1|0|Talwil Mitte",
            "sw:ServiceJourney:7-1200|0|sw:Quay:talwil-1|12:00:00|12:00:00|0|0|",
            "sw:ServiceJourney:7-1200|1|sw:ScheduledStopPoint:weide|12:05:00|12:05:00|0|0|",
            "sw:ServiceJourney:7-1200|2|sw:Quay:seehof-2|12:15:00|12:15:00|0|0|",
            "sw:ServiceJourney:7-takt|0|sw:Quay:seehof-1|06:00:00|06:00:00|0|1|",
            // A call without a time between two with one.
            "sw:ServiceJourney:7-takt|1|sw:StopPlace:bergdorf|||0|0|Talwil Mitte",
            "sw:ServiceJourney:7-takt|2|sw:Quay:talwil-1|06:15:00|06:15:00|1|0|Talwil Mitte",
        ]
    );
}

#[test]
fn a_template_journey_runs_at_its_headway_from_its_first_to_its_last_departure() {
    // A headway group states an interval, not departures — "every 20
    // minutes" —, as GTFS frequencies without exact times do; GTFS runs a
    // frequency up to, not including, its end, which lies a second after
    // the group's last departure. The group whose last departure is not a
    // whole number of intervals after its first is written as stated too.
    let (feed, report) = seeweg();
    assert_eq!(
        table(
            feed.path(),
            "frequencies.txt",
            &[
                "trip_id",
                "start_time",
                "end_time",
                "headway_secs",
                "exact_times"
            ]
        ),
        [
            "sw:ServiceJourney:7-takt|06:00:00|07:00:01|1200|0",
            "sw:ServiceJourney:7-takt|18:00:00|18:50:01|1200|0",
        ]
    );
    assert!(!report
        .decisions
        .iter()
        .any(|decision| decision.code == "headway-written-as-runs"));
}

#[test]
fn a_headway_group_s_last_departure_is_a_departure_of_the_frequency() {
    // GTFS runs a frequency from its start time up to, not including, its end
    // time: the end lies after the last departure, by less than an interval.
    // The group runs at 06:00, 06:20, 06:40 and 07:00.
    let (feed, _) = seeweg();
    let frequencies = table(
        feed.path(),
        "frequencies.txt",
        &["trip_id", "start_time", "end_time", "headway_secs"],
    );
    let first = frequencies
        .iter()
        .find(|row| row.contains("|06:00:00|"))
        .expect("the morning group");
    let end = first.split('|').nth(2).expect("an end time");
    assert!(
        ("07:00:01"..="07:20:00").contains(&end),
        "the 07:00 departure is no departure of {first}"
    );
}

#[test]
fn the_calendar_is_the_weekdays_with_the_holiday_taken_away() {
    let (feed, report) = seeweg();
    assert_eq!(
        table(
            feed.path(),
            "calendar.txt",
            &[
                "service_id",
                "monday",
                "tuesday",
                "wednesday",
                "thursday",
                "friday",
                "saturday",
                "sunday",
                "start_date",
                "end_date"
            ]
        ),
        ["c0|1|1|1|1|1|0|0|20261012|20261023"]
    );
    assert_eq!(
        table(
            feed.path(),
            "calendar_dates.txt",
            &["service_id", "date", "exception_type"]
        ),
        ["c0|20261020|2"]
    );
    assert_eq!(
        report.period,
        Some(("20261012".to_owned(), "20261023".to_owned()))
    );
}

#[test]
fn walks_and_planned_interchanges_are_transfers() {
    let (feed, report) = seeweg();
    assert_eq!(
        table(
            feed.path(),
            "transfers.txt",
            &["from_stop_id", "to_stop_id", "from_trip_id", "to_trip_id", "transfer_type", "min_transfer_time"]
        ),
        [
            // Stated twice, one way longer: the longer, no walk is shortened.
            "sw:Quay:seehof-1|sw:Quay:talwil-1|||2|180",
            "sw:Quay:talwil-1|sw:Quay:seehof-1|||2|120",
            "sw:Quay:talwil-1|sw:Quay:talwil-1|sw:ServiceJourney:7-2310|sw:ServiceJourney:7-1200|1|",
        ]
    );
    // Stated twice, it is one transfer, the first: GTFS keys a transfer by
    // its stops and trips.
    let repeated = report
        .decisions
        .iter()
        .find(|decision| decision.code == "interchange-repeated")
        .expect("the repetition is reported");
    assert_eq!(repeated.count, 1);
    assert_eq!(
        repeated.example.as_deref(),
        Some("sw:ServiceJourney:7-2310")
    );
    let walk = report
        .decisions
        .iter()
        .find(|decision| decision.code == "walk-stated-twice")
        .expect("the second walk is reported");
    assert_eq!(walk.count, 1);
    assert_eq!(
        walk.example.as_deref(),
        Some("sw:Quay:seehof-1 to sw:Quay:talwil-1")
    );
}

#[test]
fn a_journey_whose_times_go_back_is_left_out_and_said() {
    let (feed, report) = seeweg();
    assert!(!rows(feed.path(), "trips.txt")
        .iter()
        .any(|row| row["trip_id"] == "sw:ServiceJourney:7-rueckwaerts"));
    let decision = report
        .decisions
        .iter()
        .find(|decision| decision.code == "trip-time-goes-back")
        .expect("the journey is reported");
    assert_eq!(decision.count, 1);
    assert_eq!(
        decision.example.as_deref(),
        Some("sw:ServiceJourney:7-rueckwaerts")
    );
}

#[test]
fn a_shape_follows_the_service_links_read_in_their_reference_system() {
    let (feed, report) = seeweg();
    let points: Vec<(f64, f64)> = rows(feed.path(), "shapes.txt")
        .iter()
        .map(|row| {
            assert_eq!(row["shape_id"], "s0");
            (
                row["shape_pt_lat"].parse().expect("a latitude"),
                row["shape_pt_lon"].parse().expect("a longitude"),
            )
        })
        .collect();
    // Five positions — the two links meet in one —, each where PROJ puts
    // the LV95 coordinates written: within a metre.
    let expected = [
        (47.05, 8.30),
        (47.055, 8.31),
        (47.06, 8.32),
        (47.065, 8.33),
        (47.07, 8.34),
    ];
    assert_eq!(points.len(), expected.len());
    for (point, expected) in points.iter().zip(expected) {
        let metres = netex_rs::geometry::metres_between(*point, expected);
        assert!(metres < 1.0, "{point:?} is {metres:.2} m from {expected:?}");
    }
    // The inbound journey calls where no link with a geometry leads.
    assert!(report
        .decisions
        .iter()
        .any(|decision| decision.code == "shape-incomplete"
            && decision.example.as_deref() == Some("sw:ServiceJourney:7-1200")));
}

#[test]
fn the_feed_names_its_source_and_the_report_what_it_does_not_carry() {
    let (feed, report) = seeweg();
    assert_eq!(
        table(
            feed.path(),
            "feed_info.txt",
            &["feed_publisher_name", "feed_publisher_url", "feed_lang", "feed_start_date", "feed_end_date", "feed_version"]
        ),
        ["Seeweg Verkehrsverbund|https://seeweg.example|de|20261012|20261023|2026-10-10T06:00:00"]
    );
    assert_eq!(
        table(
            feed.path(),
            "attributions.txt",
            &[
                "attribution_id",
                "organization_name",
                "is_producer",
                "attribution_url"
            ]
        ),
        [
            "a1|Seeweg Verkehrsverbund|1|https://seeweg.example",
            "a2|SEEWEG|1|"
        ]
    );
    let not_carried: Vec<String> = report
        .not_carried
        .iter()
        .map(|entry| format!("{} {} {}", entry.element, entry.count, entry.first_document))
        .collect();
    assert_eq!(
        not_carried,
        [
            "FareFrame 1 seeweg-other.xml",
            "FlexibleLine 1 seeweg-other.xml",
            "NoticeAssignment 1 seeweg-line-7.xml"
        ]
    );
    assert_eq!(report.index_findings.get("flexible-line-journey"), Some(&1));
    assert_eq!(report.profile, Profile::Other);
    assert_eq!(report.documents, 2);
    assert_eq!(report.rows["stop_times.txt"], 9);
}

#[test]
fn a_publisher_given_names_the_feed_and_stops_may_take_their_global_identifier() {
    let source = Source::open(fixture("seeweg")).expect("the delivery opens");
    let delivery = Delivery::read(&source, Reading::default()).expect("the delivery reads");
    let mut options = Options::default();
    options.stop_ids = StopIds::Global;
    options.publisher = Some(Publisher {
        name: "Example".to_owned(),
        url: "https://example.org".to_owned(),
        language: Some("en".to_owned()),
    });
    let archive = tempfile::tempdir().expect("a directory");
    let path = archive.path().join("feed.zip");
    let report =
        convert(&delivery, &options, output::at(&path).expect("an archive")).expect("converts");
    let numbered = report
        .decisions
        .iter()
        .find(|decision| decision.code == "stop-made-unique")
        .expect("the numbering is reported");
    assert_eq!(
        (numbered.count, numbered.example.as_deref()),
        (1, Some("de:99001:1:0:1"))
    );
    let unpacked = tempfile::tempdir().expect("a directory");
    zip::ZipArchive::new(std::fs::File::open(&path).expect("the archive"))
        .expect("a zip archive")
        .extract(unpacked.path())
        .expect("unpacks");
    assert_eq!(
        table(
            unpacked.path(),
            "feed_info.txt",
            &["feed_publisher_name", "feed_publisher_url", "feed_lang"]
        ),
        ["Example|https://example.org|en"]
    );
    // A stop's GlobalID is its identifier: the two quays of Seehof share one,
    // the second numbered; the quay of Talwil carries its stop place's DHID, as
    // the Hessian files write it; the stop place of Talwil carries a Swiss
    // SLOID instead; a stop without either keeps its NeTEx identifier.
    assert_eq!(
        table(unpacked.path(), "stops.txt", &["stop_id", "parent_station"]),
        [
            "de:99001:2|",
            "de:99001:1|",
            "ch:1:sloid:99003|",
            "de:99001:1:0:1|de:99001:1",
            "de:99001:1:0:1#2|de:99001:1",
            "de:99001:3|ch:1:sloid:99003",
            "sw:StopPlace:bergdorf|de:99001:2",
            "sw:ScheduledStopPoint:weide|",
        ]
    );
    assert!(!unpacked.path().join("shapes.txt").exists());
}

/// The crate's own example deliveries of netex-rs, where the repository has them.
fn own_example(profile: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/own")
        .join(profile);
    if path.is_dir() {
        Some(path)
    } else {
        eprintln!(
            "SKIPPED: netex-rs's own examples are not at {}",
            path.display()
        );
        None
    }
}

#[test]
fn a_swiss_style_train_calls_as_it_states_named_by_its_train_number() {
    let (feed, report) = convert_to_directory(&fixture("seetal"), &Options::default());
    assert_eq!(report.profile, Profile::Switzerland);
    assert_eq!(
        table(feed.path(), "agency.txt", &["agency_id", "agency_name"]),
        ["ch:st:Operator:SBB|Seetalbahnen"]
    );
    assert_eq!(
        table(
            feed.path(),
            "routes.txt",
            &[
                "route_id",
                "agency_id",
                "route_short_name",
                "route_long_name",
                "route_type"
            ]
        ),
        ["ch:st:Line:IR9|ch:st:Operator:SBB|IR9|IR 9|2"]
    );
    assert_eq!(
        table(
            feed.path(),
            "trips.txt",
            &[
                "trip_id",
                "trip_headsign",
                "trip_short_name",
                "direction_id"
            ]
        ),
        ["ch:st:ServiceJourney:IR9-2148|Grenzheim|2148|0"]
    );
    // The train passes Talwil without a call and reaches Grenzheim after midnight.
    assert_eq!(
        table(
            feed.path(),
            "stop_times.txt",
            &["stop_id", "arrival_time", "departure_time", "stop_headsign"]
        ),
        [
            "ch:st:ScheduledStopPoint:A|21:48:00|21:48:00|",
            "ch:st:ScheduledStopPoint:B|22:16:00|22:18:00|",
            "ch:st:ScheduledStopPoint:D|24:30:00|24:30:00|",
        ]
    );
    assert_eq!(
        table(
            feed.path(),
            "calendar.txt",
            &["service_id", "monday", "sunday", "start_date", "end_date"]
        ),
        ["c0|1|1|20261012|20261018"]
    );
    assert_eq!(report.index_findings.get("passed-through"), Some(&1));
}

#[test]
fn calls_at_stop_points_with_no_place_and_no_position_are_left_out_and_said() {
    // netex-rs's own Swiss-style example gives its stop points no position.
    let Some(path) = own_example("ch") else {
        return;
    };
    let (feed, report) = convert_to_directory(&path, &Options::default());
    assert!(rows(feed.path(), "trips.txt").is_empty());
    let count = |code: &str| {
        report
            .decisions
            .iter()
            .find(|decision| decision.code == code)
            .map(|decision| decision.count)
    };
    assert_eq!(count("call-without-place"), Some(5));
    assert_eq!(count("trip-without-two-calls"), Some(2));
}

#[test]
fn an_epip_line_offer_is_read_as_epip() {
    let Some(path) = own_example("epip") else {
        return;
    };
    let (feed, report) = convert_to_directory(&path, &Options::default());
    assert_eq!(report.profile, Profile::Epip);
    assert!(!rows(feed.path(), "trips.txt").is_empty());
    assert!(rows(feed.path(), "stops.txt")
        .iter()
        .any(|row| row["location_type"] == "1"));
}

#[test]
fn a_delivery_stating_offsets_and_no_time_zone_takes_its_profile_s() {
    // The Swiss documents write their offsets from UTC, not a zone.
    let delivery = tempfile::tempdir().expect("a directory");
    for name in ["shared.xml", "timetable.xml"] {
        let xml = std::fs::read_to_string(fixture("seetal").join(name)).expect("the fixture");
        let xml = xml.replace(
            "<TimeZone>Europe/Zurich</TimeZone>",
            "<TimeZoneOffset>+1</TimeZoneOffset><SummerTimeZoneOffset>+2</SummerTimeZoneOffset>",
        );
        std::fs::write(delivery.path().join(name), xml).expect("written");
    }
    let (feed, report) = convert_to_directory(delivery.path(), &Options::default());
    assert_eq!(
        table(feed.path(), "agency.txt", &["agency_timezone"]),
        ["Europe/Zurich"]
    );
    let decision = report
        .decisions
        .iter()
        .find(|decision| decision.code == "time-zone-of-identifiers")
        .expect("the decision is reported");
    assert_eq!(
        decision.example.as_deref(),
        Some("Europe/Zurich: 1 of 1 journeys identified ch:")
    );
}

/// The decision `code` of `report`: how often it was taken, and its example.
fn decision(report: &Report, code: &str) -> Option<(u64, String)> {
    report
        .decisions
        .iter()
        .find(|decision| decision.code == code)
        .map(|decision| (decision.count, decision.example.clone().unwrap_or_default()))
}

#[test]
fn an_austrian_delivery_framed_as_epip_takes_the_zone_its_identifiers_show() {
    let Some(path) = own_example("at") else {
        return;
    };
    // As the Austrian national deliveries are written: frames typed as the
    // European profile's, no time zone.
    let delivery = tempfile::tempdir().expect("a directory");
    for entry in std::fs::read_dir(&path).expect("the example") {
        let file = entry.expect("a file").path();
        let xml = std::fs::read_to_string(&file).expect("the document");
        let xml = xml
            .replace("<TimeZone>Europe/Vienna</TimeZone>", "")
            .replacen(
                "<FrameDefaults>",
                r#"<TypeOfFrameRef ref="epip:EU_PI_LINE_OFFER"/><FrameDefaults>"#,
                1,
            );
        std::fs::write(delivery.path().join(file.file_name().expect("a name")), xml)
            .expect("written");
    }
    let (feed, report) = convert_to_directory(delivery.path(), &Options::default());
    assert_eq!(report.profile, Profile::Epip);
    assert!(table(feed.path(), "agency.txt", &["agency_timezone"])
        .iter()
        .all(|zone| zone == "Europe/Vienna"));
    assert_eq!(
        decision(&report, "time-zone-of-identifiers"),
        Some((
            1,
            "Europe/Vienna: 3 of 3 journeys identified at:".to_owned()
        ))
    );
}

#[test]
fn a_belgian_delivery_takes_the_zone_of_the_country_its_stops_lie_in() {
    let Some(path) = own_example("be") else {
        return;
    };
    // Its line names no operator: the publisher's agency runs it.
    let publisher = Publisher {
        name: "Example".to_owned(),
        url: "https://example.org".to_owned(),
        language: None,
    };
    let mut options = Options::default();
    options.publisher = Some(publisher);
    let (feed, report) = convert_to_directory(&path, &options);
    assert_eq!(
        table(feed.path(), "agency.txt", &["agency_id", "agency_timezone"]),
        ["publisher|Europe/Brussels"]
    );
    assert_eq!(
        decision(&report, "time-zone-of-stops"),
        Some((
            1,
            "Europe/Brussels: 4 of 4 stops with a position in BE".to_owned()
        ))
    );
}

#[test]
fn a_delivery_stating_no_zone_whose_stops_lie_in_no_country_known_takes_none() {
    // Uferweg moved out into the Atlantic, its documents naming no zone.
    let delivery = tempfile::tempdir().expect("a directory");
    for name in ["uferweg-nord.xml", "uferweg-sued.xml"] {
        let xml = std::fs::read_to_string(fixture("uferweg").join(name)).expect("the fixture");
        let xml = xml
            .replace("<TimeZone>Europe/Zurich</TimeZone>", "")
            .replace("<Longitude>8.", "<Longitude>-30.");
        std::fs::write(delivery.path().join(name), xml).expect("written");
    }
    let source = Source::open(delivery.path()).expect("the delivery opens");
    let delivery = Delivery::read(&source, Reading::default()).expect("the delivery reads");
    let feed = tempfile::tempdir().expect("a directory");
    let converted = convert(
        &delivery,
        &Options::default(),
        output::at(feed.path()).expect("an output"),
    );
    assert!(
        matches!(converted, Err(netex2gtfs::Error::NoTimeZone)),
        "{converted:?}"
    );
    // Given one, it converts.
    let mut options = Options::default();
    options.time_zone = Some("Atlantic/Azores".to_owned());
    convert(
        &delivery,
        &options,
        output::at(feed.path()).expect("an output"),
    )
    .expect("converts with the zone given");
}

#[test]
fn a_french_delivery_converts_with_its_quays_beside_their_stop_places_and_its_three_calendars() {
    let Some(path) = own_example("fr") else {
        return;
    };
    let (feed, report) = convert_to_directory(&path, &Options::default());
    assert_eq!(
        table(feed.path(), "agency.txt", &["agency_id", "agency_timezone"]),
        ["EX:Operator:autocars:LOC|Europe/Paris"]
    );
    // The quays stand under the stop place naming them; the first states its
    // accessibility. The second quay of the Gare has no stop point and no call.
    assert_eq!(
        table(
            feed.path(),
            "stops.txt",
            &[
                "stop_id",
                "location_type",
                "parent_station",
                "wheelchair_boarding"
            ]
        ),
        [
            "FR:00001:LMO:gare:LOC|1||0",
            "FR:00001:LMO:mairie:LOC|1||0",
            "FR:00002:LMO:eglise:LOC|1||0",
            "FR:00001:ZE:gare-a:LOC|0|FR:00001:LMO:gare:LOC|1",
            "FR:00001:ZE:mairie:LOC|0|FR:00001:LMO:mairie:LOC|0",
            "FR:00002:ZE:eglise:LOC|0|FR:00002:LMO:eglise:LOC|0",
        ]
    );
    // The passing times name no point: each is at the point of its position.
    assert_eq!(
        table(
            feed.path(),
            "stop_times.txt",
            &["trip_id", "stop_id", "arrival_time", "departure_time"]
        )[6..],
        [
            "EX:ServiceJourney:10-2345:LOC|FR:00001:ZE:gare-a:LOC|23:45:00|23:45:00",
            "EX:ServiceJourney:10-2345:LOC|FR:00001:ZE:mairie:LOC|23:56:00|23:57:00",
            "EX:ServiceJourney:10-2345:LOC|FR:00002:ZE:eglise:LOC|24:10:00|24:10:00",
        ]
    );
    // School days as their dates list them; the weekend by its days of the
    // week but Sunday 18, with Friday 16; the Wednesdays of a plain period.
    assert_eq!(
        table(feed.path(), "trips.txt", &["trip_id", "service_id"]),
        [
            "EX:ServiceJourney:10-0730:LOC|c0",
            "EX:ServiceJourney:10-0930:LOC|c1",
            "EX:ServiceJourney:10-2345:LOC|c2",
        ]
    );
    assert_eq!(
        table(
            feed.path(),
            "calendar.txt",
            &[
                "service_id",
                "monday",
                "tuesday",
                "wednesday",
                "thursday",
                "friday",
                "saturday",
                "sunday",
                "start_date",
                "end_date"
            ]
        ),
        [
            "c0|1|1|1|1|0|0|0|20261012|20261020",
            "c1|0|0|0|0|1|1|1|20261010|20261025",
            "c2|0|0|1|0|0|0|0|20261014|20261021",
        ]
    );
    assert_eq!(
        table(
            feed.path(),
            "calendar_dates.txt",
            &["service_id", "date", "exception_type"]
        ),
        ["c1|20261018|2", "c1|20261023|2"]
    );
    assert_eq!(report.index_findings.get("walk-without-duration"), Some(&1));
}

#[test]
fn a_train_number_among_a_general_frame_s_members_names_its_trip() {
    let Some(path) = own_example("fr") else {
        return;
    };
    let delivery = tempfile::tempdir().expect("a directory");
    for entry in std::fs::read_dir(&path).expect("the example") {
        let entry = entry.expect("an entry");
        let mut xml = std::fs::read_to_string(entry.path()).expect("a document");
        if entry.file_name() == "line_10.xml" {
            xml = xml
                .replacen(
                    r#"<JourneyPatternRef ref="EX:ServiceJourneyPattern:10-aller:LOC" version="1"/>"#,
                    r#"<JourneyPatternRef ref="EX:ServiceJourneyPattern:10-aller:LOC" version="1"/><trainNumbers><TrainNumberRef ref="EX:TrainNumber:851:LOC" version="1"/></trainNumbers>"#,
                    1,
                )
                .replace(
                    r#"<TypeOfFrameRef ref="FR:TypeOfFrame:NETEX_HORAIRE:"/>
          <members>"#,
                    r#"<TypeOfFrameRef ref="FR:TypeOfFrame:NETEX_HORAIRE:"/>
          <members><TrainNumber id="EX:TrainNumber:851:LOC" version="1"><ForAdvertisement>851</ForAdvertisement></TrainNumber>"#,
                );
        }
        std::fs::write(delivery.path().join(entry.file_name()), xml).expect("written");
    }
    let (feed, report) = convert_to_directory(delivery.path(), &Options::default());
    assert_eq!(
        table(feed.path(), "trips.txt", &["trip_id", "trip_short_name"]),
        [
            "EX:ServiceJourney:10-0730:LOC|851",
            "EX:ServiceJourney:10-0930:LOC|",
            "EX:ServiceJourney:10-2345:LOC|",
        ]
    );
    assert!(
        report
            .not_carried
            .iter()
            .all(|n| n.element != "TrainNumber"),
        "{:?}",
        report.not_carried
    );
}

#[test]
fn a_nordic_delivery_converts_with_its_register_its_dated_trains_and_its_interchange() {
    let (Some(path), Some(register)) = (own_example("nordic"), own_example("nordic-stops")) else {
        return;
    };
    let source = Source::open(&path)
        .and_then(|delivery| delivery.with(Source::open(&register)?))
        .expect("the delivery and its register open");
    let delivery = Delivery::read(&source, Reading::default()).expect("the delivery reads");
    let feed = tempfile::tempdir().expect("a directory");
    let report = convert(
        &delivery,
        &Options::default(),
        output::at(feed.path()).expect("an output"),
    )
    .expect("converts");
    // The frames' time zone; the quays of the register under their stations,
    // the outermost parent site where a stop place is within one.
    assert_eq!(
        table(feed.path(), "agency.txt", &["agency_id", "agency_timezone"]),
        ["EX:Operator:1|Europe/Oslo"]
    );
    assert_eq!(
        table(
            feed.path(),
            "stops.txt",
            &["stop_id", "location_type", "parent_station"]
        )[3..],
        [
            "NSR:Quay:9011|0|NSR:StopPlace:900",
            "NSR:Quay:9021|0|NSR:StopPlace:900",
            "NSR:Quay:9101|0|NSR:StopPlace:910",
            "NSR:Quay:9201|0|NSR:StopPlace:920",
            "NSR:Quay:9202|0|NSR:StopPlace:920",
        ]
    );
    // The flexible line's journey is no trip; the train runs on the days of
    // its dated journeys but the cancelled and the replaced, the bus
    // replacing it on the day it is replaced.
    assert_eq!(
        table(feed.path(), "trips.txt", &["trip_id", "service_id"]),
        [
            "EX:ServiceJourney:1-0750|c0",
            "EX:ServiceJourney:1-1000|c1",
            "EX:ServiceJourney:R1-0812|c2",
            "EX:ServiceJourney:R1-0812-bus|c3",
        ]
    );
    assert_eq!(
        table(
            feed.path(),
            "calendar.txt",
            &[
                "service_id",
                "monday",
                "tuesday",
                "wednesday",
                "thursday",
                "friday",
                "saturday",
                "sunday",
                "start_date",
                "end_date"
            ]
        ),
        [
            "c0|1|1|1|1|1|0|0|20261001|20261030",
            "c1|1|1|1|1|1|1|0|20261001|20261030",
            "c2|1|1|0|0|1|0|0|20261012|20261016",
            "c3|0|0|0|1|0|0|0|20261015|20261015",
        ]
    );
    assert_eq!(
        table(
            feed.path(),
            "calendar_dates.txt",
            &["service_id", "date", "exception_type"]
        ),
        [
            "c0|20261014|2",
            "c1|20261003|2",
            "c1|20261010|2",
            "c1|20261014|2"
        ]
    );
    // The interchange of the train's line file from the bus of the other.
    assert_eq!(
        table(
            feed.path(),
            "transfers.txt",
            &[
                "from_stop_id",
                "to_stop_id",
                "from_trip_id",
                "to_trip_id",
                "transfer_type"
            ]
        ),
        ["NSR:Quay:9021|NSR:Quay:9011|EX:ServiceJourney:1-0750|EX:ServiceJourney:R1-0812|1"]
    );
    assert_eq!(report.index_findings.get("flexible-line-journey"), Some(&1));
}

#[test]
fn a_dutch_delivery_converts_with_its_register_codes_rd_positions_and_demanded_times() {
    let (Some(path), Some(register)) = (own_example("nl"), own_example("nl-register")) else {
        return;
    };
    let source = Source::open(&path)
        .and_then(|delivery| delivery.with(Source::open(&register)?))
        .expect("the delivery and its register open");
    let delivery = Delivery::read(&source, Reading::default()).expect("the delivery reads");
    let feed = tempfile::tempdir().expect("a directory");
    let mut options = Options::default();
    options.stop_ids = StopIds::Global;
    let report = convert(
        &delivery,
        &options,
        output::at(feed.path()).expect("an output"),
    )
    .expect("converts");
    assert_eq!(
        table(feed.path(), "agency.txt", &["agency_id", "agency_timezone"]),
        ["NL:EX:Operator:EX|Europe/Amsterdam"]
    );
    // Every stop the journeys call at is known by the code the register gives
    // it, as the German delivery names Dutch stops, and placed where its RD New
    // position says. The stop point the register holds no quay for is called
    // at only after the journey to it runs out of times, and is no stop.
    assert_eq!(
        table(
            feed.path(),
            "stops.txt",
            &["stop_id", "parent_station", "stop_lat", "stop_lon"]
        ),
        [
            "NL:S:99000100||52.1591269|5.3902724",
            "NL:S:99000200||52.1549922|5.3964094",
            "NL:S:99000300||52.1470814|5.4076574",
            "NL:Q:99000101|NL:S:99000100|52.1592618|5.3900532",
            "NL:Q:99000102|NL:S:99000100|52.1589921|5.3904916",
            "NL:Q:99000201|NL:S:99000200|52.1551270|5.3965556",
            "NL:Q:99000301|NL:S:99000300|52.1469915|5.4078034",
        ]
    );
    // The times the departure and the time demand type give, the timing
    // point between Station and Markt no stop; the night journey past
    // midnight.
    let trip = |trip: &str| {
        rows(feed.path(), "stop_times.txt")
            .into_iter()
            .filter(|row| row["trip_id"] == trip)
            .map(|row| {
                format!(
                    "{} {}-{}",
                    row["stop_id"], row["arrival_time"], row["departure_time"]
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(
        trip("NL:EX:ServiceJourney:1-191"),
        [
            "NL:Q:99000101 23:58:00-23:58:00",
            "NL:Q:99000201 24:02:00-24:03:00",
            "NL:Q:99000301 24:06:00-24:06:00",
        ]
    );
    // The journey running on no day — it names only a condition not
    // available — is no trip.
    let trips = table(feed.path(), "trips.txt", &["trip_id"]);
    assert_eq!(trips.len(), 6, "{trips:?}");
    assert!(!trips.contains(&"NL:EX:ServiceJourney:1-103-na".to_owned()));
    assert_eq!(report.index_findings.get("link-without-run-time"), Some(&1));
    assert_eq!(report.index_findings.get("journey-without-days"), Some(&1));
}

#[test]
fn stops_and_transfers_are_written_in_the_order_of_their_identifiers() {
    let (feed, report) = convert_to_directory(&fixture("uferweg"), &Options::default());
    assert_eq!(
        table(
            feed.path(),
            "stops.txt",
            &["stop_id", "location_type", "parent_station"]
        ),
        [
            "uw:StopPlace:anger|1|",
            "uw:StopPlace:hafen|1|",
            "uw:StopPlace:muehle|1|",
            "uw:StopPlace:steg|1|",
            "uw:Quay:anger-1|0|uw:StopPlace:anger",
            "uw:Quay:hafen-1|0|uw:StopPlace:hafen",
            "uw:Quay:muehle-1|0|uw:StopPlace:muehle",
            "uw:Quay:steg-1|0|uw:StopPlace:steg",
        ]
    );
    // The walks by the stops they lead from and to; the interchange both
    // documents state, as the document whose name sorts first states it.
    assert_eq!(
        table(
            feed.path(),
            "transfers.txt",
            &[
                "from_stop_id",
                "to_stop_id",
                "from_trip_id",
                "to_trip_id",
                "transfer_type",
                "min_transfer_time"
            ]
        ),
        [
            "uw:Quay:hafen-1|uw:Quay:steg-1|||2|240",
            "uw:Quay:muehle-1|uw:Quay:steg-1|||2|120",
            "uw:Quay:steg-1|uw:Quay:muehle-1|||2|120",
            "uw:Quay:muehle-1|uw:Quay:steg-1|uw:ServiceJourney:N-0800|uw:ServiceJourney:S-0815|1|",
        ]
    );
    let repeated = report
        .decisions
        .iter()
        .find(|decision| decision.code == "interchange-repeated")
        .expect("the repetition is reported");
    assert_eq!(
        (repeated.count, repeated.example.as_deref()),
        (1, Some("uw:ServiceJourney:N-0800"))
    );
}

/// The bytes of the feed archive at `path`.
fn archive_bytes(path: &Path) -> Vec<u8> {
    std::fs::read(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

#[test]
fn a_feed_is_the_same_bytes_whatever_number_of_threads_reads_the_delivery() {
    let written = tempfile::tempdir().expect("a directory");
    for name in ["uferweg", "seeweg"] {
        let source = Source::open(fixture(name)).expect("the delivery opens");
        let mut feeds = Vec::new();
        for threads in [1, 2, 4] {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(threads)
                .build()
                .expect("a pool");
            let path = written.path().join(format!("{name}-{threads}.zip"));
            let report = pool.install(|| {
                let delivery = Delivery::read(
                    &source,
                    Reading {
                        geometry: true,
                        ..Reading::default()
                    },
                )
                .expect("the delivery reads");
                let mut options = Options::default();
                options.shapes = true;
                convert(&delivery, &options, output::at(&path).expect("an archive"))
                    .expect("converts")
            });
            feeds.push((threads, archive_bytes(&path), report));
        }
        let (_, bytes, report) = &feeds[0];
        for (threads, other_bytes, other_report) in &feeds[1..] {
            assert!(
                other_bytes == bytes,
                "{name}: the feed read on {threads} threads differs from the one read on 1"
            );
            assert_eq!(other_report, report, "{name} on {threads} threads");
        }
    }
}

#[test]
fn an_empty_web_address_is_no_web_address() {
    // Four Norwegian taxi operators write `<Url/>`.
    let delivery = tempfile::tempdir().expect("a directory");
    for name in ["seeweg-line-7.xml", "seeweg-other.xml"] {
        let xml = std::fs::read_to_string(fixture("seeweg").join(name)).expect("the fixture");
        let xml = xml
            .replace("<Url>https://bahnen.seeweg.example</Url>", "<Url/>")
            .replace("<Url>https://seeweg.example</Url>", "<Url> </Url>");
        std::fs::write(delivery.path().join(name), xml).expect("written");
    }
    let publisher = Publisher {
        name: "Example".to_owned(),
        url: "https://example.org".to_owned(),
        language: None,
    };
    let mut options = Options::default();
    options.publisher = Some(publisher);
    let (feed, report) = convert_to_directory(delivery.path(), &options);
    assert_eq!(
        table(feed.path(), "agency.txt", &["agency_id", "agency_url"]),
        ["sw:Operator:SWB|https://example.org"]
    );
    let without = report
        .decisions
        .iter()
        .find(|decision| decision.code == "agency-without-url")
        .expect("the organisation without a web address is reported");
    assert_eq!(
        (without.count, without.example.as_deref()),
        (1, Some("sw:Operator:SWB"))
    );
    // A data source with an empty web address names the feed's producer,
    // but publishes no feed.
    let (feed, report) = convert_to_directory(delivery.path(), &Options::default());
    assert!(!feed.path().join("feed_info.txt").exists());
    assert!(report
        .decisions
        .iter()
        .any(|decision| decision.code == "feed-info-without-publisher"));
    assert_eq!(
        table(
            feed.path(),
            "attributions.txt",
            &["organization_name", "attribution_url"]
        ),
        ["Seeweg Verkehrsverbund|", "SEEWEG|"]
    );
}

#[test]
fn shapes_follow_the_links_a_pattern_names_with_their_geometry_in_a_projection() {
    let (feed, report) = convert_to_directory(&fixture("fjordvei"), &shapes());
    // The links the line file's patterns name are the shared data's; the
    // pattern naming none, and the one naming a link between other stop
    // points, take the link between their stop points there.
    assert_eq!(
        table(feed.path(), "trips.txt", &["trip_id", "shape_id"]),
        [
            "FV:ServiceJourney:1-0800|s0",
            "FV:ServiceJourney:1-0900|s1",
            "FV:ServiceJourney:1-1000|",
            "FV:ServiceJourney:1-1100|s2",
            "FV:ServiceJourney:1-1200|s3",
            "FV:ServiceJourney:1-1300|s2",
        ]
    );
    assert_eq!(
        table(
            feed.path(),
            "shapes.txt",
            &[
                "shape_id",
                "shape_pt_lat",
                "shape_pt_lon",
                "shape_pt_sequence"
            ]
        ),
        [
            // On to Fjord, the two links one after the other.
            "s0|59.9000000|10.7000000|0",
            "s0|59.9050000|10.7080000|1",
            "s0|59.9100000|10.7200000|2",
            "s0|59.9160000|10.7290000|3",
            "s0|59.9200000|10.7400000|4",
            // The detour its pattern names, not the direct link between the
            // same stop points.
            "s1|59.9000000|10.7000000|0",
            "s1|59.8980000|10.7150000|1",
            "s1|59.9100000|10.7200000|2",
            "s2|59.9100000|10.7200000|0",
            "s2|59.9160000|10.7290000|1",
            "s2|59.9200000|10.7400000|2",
            // One link for the whole way, past Brygga.
            "s3|59.9000000|10.7000000|0",
            "s3|59.9040000|10.7060000|1",
            "s3|59.9100000|10.7200000|2",
            "s3|59.9200000|10.7400000|3",
        ]
    );
    let decision = |code: &str| {
        report
            .decisions
            .iter()
            .find(|decision| decision.code == code)
            .map(|decision| (decision.count, decision.example.clone().unwrap_or_default()))
    };
    assert_eq!(
        decision("shape-incomplete"),
        Some((1, "FV:ServiceJourney:1-1000".to_owned()))
    );
    assert_eq!(
        decision("link-without-geometry"),
        Some((1, "FV:ServiceLink:fjord-torget".to_owned()))
    );
    assert_eq!(decision("shape-in-unknown-system"), None);
}

/// The elements of the report's "not carried", each with its count.
fn not_carried(report: &Report) -> Vec<String> {
    report
        .not_carried
        .iter()
        .map(|entry| format!("{} {}", entry.element, entry.count))
        .collect()
}

#[test]
fn a_shape_is_the_projection_its_journey_names_among_a_general_frames_members() {
    // Stop points alone, no stop place: no country to take a time zone from.
    let mut options = shapes();
    options.time_zone = Some("Europe/Paris".to_owned());
    let (feed, report) = convert_to_directory(&fixture("rive"), &options);
    // Two journeys name one projection and share its shape; the journey
    // naming a projection without a geometry, and the one naming none, have
    // no service link to follow either.
    assert_eq!(
        table(feed.path(), "trips.txt", &["trip_id", "shape_id"]),
        [
            "RV:ServiceJourney:0700:LOC|s0",
            "RV:ServiceJourney:0800:LOC|s0",
            "RV:ServiceJourney:0900:LOC|s1",
            "RV:ServiceJourney:1000:LOC|",
            "RV:ServiceJourney:1100:LOC|",
        ]
    );
    // Written latitude first under a name that says longitude first: read
    // in the order that starts where the journey does.
    assert_eq!(
        table(
            feed.path(),
            "shapes.txt",
            &[
                "shape_id",
                "shape_pt_lat",
                "shape_pt_lon",
                "shape_pt_sequence"
            ]
        ),
        [
            "s0|44.8400000|-0.5700000|0",
            "s0|44.8420000|-0.5660000|1",
            "s0|44.8450000|-0.5600000|2",
            "s0|44.8500000|-0.5500000|3",
            "s1|44.8400000|-0.5700000|0",
            "s1|44.8380000|-0.5600000|1",
            "s1|44.8450000|-0.5600000|2",
            "s1|44.8500000|-0.5500000|3",
        ]
    );
    let decision = |code: &str| {
        report
            .decisions
            .iter()
            .find(|decision| decision.code == code)
            .map(|decision| (decision.count, decision.example.clone().unwrap_or_default()))
    };
    assert_eq!(
        decision("projection-without-geometry"),
        Some((1, "RV:LinkSequenceProjection:sans-trace:LOC".to_owned()))
    );
    assert_eq!(
        decision("shape-incomplete"),
        Some((2, "RV:ServiceJourney:1000:LOC".to_owned()))
    );
    assert!(
        !not_carried(&report)
            .iter()
            .any(|e| e.starts_with("LinkSequenceProjection")),
        "the projections are carried: {:?}",
        not_carried(&report)
    );
    // Without shapes, the projections are what the feed does not carry.
    options.shapes = false;
    let (_, report) = convert_to_directory(&fixture("rive"), &options);
    assert!(
        not_carried(&report).contains(&"LinkSequenceProjection 3".to_owned()),
        "{:?}",
        not_carried(&report)
    );
}

#[test]
fn shapes_asked_of_a_delivery_stating_no_geometry_are_each_said_to_be_missing() {
    let (feed, report) = convert_to_directory(&fixture("uferweg"), &shapes());
    assert!(rows(feed.path(), "shapes.txt").is_empty());
    let missing = report
        .decisions
        .iter()
        .find(|decision| decision.code == "shape-incomplete")
        .expect("the trips without a shape are reported");
    assert_eq!(
        (missing.count, missing.example.as_deref()),
        (2, Some("uw:ServiceJourney:N-0800"))
    );
}

/// The Seeweg delivery, each of its documents as `edit` makes it of its name
/// and text, with the documents `more` beside them, in a directory of its own.
fn seeweg_edited(edit: impl Fn(&str, &str) -> String, more: &[(&str, &str)]) -> tempfile::TempDir {
    let delivery = tempfile::tempdir().expect("a directory");
    for name in ["seeweg-line-7.xml", "seeweg-other.xml"] {
        let xml = std::fs::read_to_string(fixture("seeweg").join(name)).expect("the fixture");
        std::fs::write(delivery.path().join(name), edit(name, &xml)).expect("written");
    }
    for (name, xml) in more {
        std::fs::write(delivery.path().join(name), xml).expect("written");
    }
    delivery
}

/// The value of `column` of each row of `file`.
fn column(feed: &Path, file: &str, column: &str) -> Vec<String> {
    rows(feed, file)
        .into_iter()
        .map(|mut row| row.remove(column).unwrap_or_default())
        .collect()
}

/// A line defined again under the identifier of Seeweg's line 7, otherwise,
/// and a line whose identifier is the one a second route of line 7 would
/// be numbered, each with a journey.
const EXPRESS: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<PublicationDelivery xmlns="http://www.netex.org.uk/netex" version="1.10">
  <PublicationTimestamp>2026-10-10T06:00:00</PublicationTimestamp>
  <ParticipantRef>SEEWEG</ParticipantRef>
  <dataObjects>
    <CompositeFrame id="sw:CompositeFrame:express" version="1">
      <frames>
        <ServiceFrame id="sw:ServiceFrame:express" version="1">
          <lines>
            <Line id="sw:Line:7" version="2">
              <Name>Seeweg Express</Name>
              <TransportMode>bus</TransportMode>
              <PublicCode>7E</PublicCode>
              <OperatorRef ref="sw:Operator:SWB" version="1"/>
            </Line>
            <Line id="sw:Line:7#2" version="1">
              <Name>Seeweg Nacht</Name>
              <TransportMode>bus</TransportMode>
              <PublicCode>N7</PublicCode>
              <OperatorRef ref="sw:Operator:SWB" version="1"/>
            </Line>
          </lines>
        </ServiceFrame>
        <TimetableFrame id="sw:TimetableFrame:express" version="1">
          <vehicleJourneys>
            <ServiceJourney id="sw:ServiceJourney:express" version="1">
              <dayTypes><DayTypeRef ref="sw:DayType:weekdays" version="1"/></dayTypes>
              <ServiceJourneyPatternRef ref="sw:ServiceJourneyPattern:7-out" version="1"/>
              <LineRef ref="sw:Line:7" version="2"/>
              <passingTimes>
                <TimetabledPassingTime id="sw:TimetabledPassingTime:express-1" version="1">
                  <StopPointInJourneyPatternRef ref="sw:StopPointInJourneyPattern:7-out-1" version="1"/>
                  <DepartureTime>08:00:00</DepartureTime>
                </TimetabledPassingTime>
                <TimetabledPassingTime id="sw:TimetabledPassingTime:express-3" version="1">
                  <StopPointInJourneyPatternRef ref="sw:StopPointInJourneyPattern:7-out-3" version="1"/>
                  <ArrivalTime>08:12:00</ArrivalTime>
                </TimetabledPassingTime>
              </passingTimes>
            </ServiceJourney>
            <ServiceJourney id="sw:ServiceJourney:nacht" version="1">
              <dayTypes><DayTypeRef ref="sw:DayType:weekdays" version="1"/></dayTypes>
              <ServiceJourneyPatternRef ref="sw:ServiceJourneyPattern:7-out" version="1"/>
              <LineRef ref="sw:Line:7#2" version="1"/>
              <passingTimes>
                <TimetabledPassingTime id="sw:TimetabledPassingTime:nacht-1" version="1">
                  <StopPointInJourneyPatternRef ref="sw:StopPointInJourneyPattern:7-out-1" version="1"/>
                  <DepartureTime>01:00:00</DepartureTime>
                </TimetabledPassingTime>
                <TimetabledPassingTime id="sw:TimetabledPassingTime:nacht-3" version="1">
                  <StopPointInJourneyPatternRef ref="sw:StopPointInJourneyPattern:7-out-3" version="1"/>
                  <ArrivalTime>01:15:00</ArrivalTime>
                </TimetabledPassingTime>
              </passingTimes>
            </ServiceJourney>
          </vehicleJourneys>
        </TimetableFrame>
      </frames>
    </CompositeFrame>
  </dataObjects>
</PublicationDelivery>
"#;

#[test]
fn a_route_numbered_apart_takes_no_identifier_another_route_has() {
    let delivery = seeweg_edited(|_, xml| xml.to_owned(), &[("seeweg-express.xml", EXPRESS)]);
    let (feed, report) = convert_to_directory(delivery.path(), &Options::default());
    let routes = table(feed.path(), "routes.txt", &["route_id", "route_short_name"]);
    let ids: std::collections::BTreeSet<&str> = routes
        .iter()
        .map(|route| route.split('|').next().expect("an identifier"))
        .collect();
    assert_eq!((ids.len(), routes.len()), (3, 3), "{routes:?}");
    let short_name: BTreeMap<&str, &str> = routes
        .iter()
        .map(|route| route.split_once('|').expect("two columns"))
        .collect();
    let trips: Vec<String> = table(feed.path(), "trips.txt", &["trip_id", "route_id"])
        .into_iter()
        .filter(|trip| !trip.starts_with("sw:ServiceJourney:7-"))
        .map(|trip| {
            let (trip, route) = trip.split_once('|').expect("two columns");
            format!("{trip} {}", short_name[route])
        })
        .collect();
    assert_eq!(
        trips,
        ["sw:ServiceJourney:express 7E", "sw:ServiceJourney:nacht N7"]
    );
    assert!(decision(&report, "route-made-unique").is_some());
}

#[test]
fn a_time_zone_written_as_an_abbreviation_is_no_time_zone() {
    // As the examples of the standard write it, an offset rather than a zone.
    let delivery = seeweg_edited(
        |_, xml| {
            xml.replace(
                "<TimeZone>Europe/Zurich</TimeZone>",
                "<TimeZone>CEST</TimeZone>",
            )
        },
        &[],
    );
    let (feed, report) = convert_to_directory(delivery.path(), &Options::default());
    assert_eq!(
        column(feed.path(), "agency.txt", "agency_timezone"),
        ["Europe/Zurich"]
    );
    assert_eq!(
        decision(&report, "time-zone-not-a-name"),
        Some((1, "seeweg-line-7.xml: CEST".to_owned()))
    );
    assert!(decision(&report, "time-zone-of-stops").is_some());
}

#[test]
fn every_agency_of_a_feed_is_in_the_one_time_zone_of_the_feed() {
    // GTFS has one time zone for all agencies of a feed.
    let delivery = tempfile::tempdir().expect("a directory");
    for name in ["uferweg-nord.xml", "uferweg-sued.xml"] {
        let xml = std::fs::read_to_string(fixture("uferweg").join(name)).expect("the fixture");
        let xml = if name == "uferweg-sued.xml" {
            xml.replace(
                "<TimeZone>Europe/Zurich</TimeZone>",
                "<TimeZone>Europe/Berlin</TimeZone>",
            )
        } else {
            xml
        };
        std::fs::write(delivery.path().join(name), xml).expect("written");
    }
    let (feed, report) = convert_to_directory(delivery.path(), &Options::default());
    assert_eq!(
        table(feed.path(), "agency.txt", &["agency_id", "agency_timezone"]),
        [
            "uw:Operator:nord|Europe/Zurich",
            "uw:Operator:sued|Europe/Zurich"
        ]
    );
    assert_eq!(
        decision(&report, "agency-time-zone-unified"),
        Some((1, "uw:Operator:sued: Europe/Berlin".to_owned()))
    );
}

/// The Seeweg delivery whose outbound pattern begins at the stop point
/// placed nowhere, as `edit` makes it beyond that.
fn seeweg_from_nowhere(edit: impl Fn(&str) -> String) -> tempfile::TempDir {
    seeweg_edited(
        |_, xml| {
            edit(&xml.replacen(
                "<StopPointInJourneyPattern id=\"sw:StopPointInJourneyPattern:7-out-1\" version=\"1\" order=\"1\">\n                  <ScheduledStopPointRef ref=\"sw:ScheduledStopPoint:seehof\"",
                "<StopPointInJourneyPattern id=\"sw:StopPointInJourneyPattern:7-out-1\" version=\"1\" order=\"1\">\n                  <ScheduledStopPointRef ref=\"sw:ScheduledStopPoint:nirgends\"",
                1,
            ))
        },
        &[],
    )
}

#[test]
fn the_first_and_the_last_call_of_every_trip_have_a_time() {
    // The template journey's first call is at a stop point placed nowhere and
    // left out; its next, at Bergdorf, has no time: GTFS requires one at a
    // trip's first and last stop. Cut there, one call is left.
    let delivery = seeweg_from_nowhere(str::to_owned);
    let (feed, report) = convert_to_directory(delivery.path(), &Options::default());
    let mut trips: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    for row in rows(feed.path(), "stop_times.txt") {
        trips
            .entry(row["trip_id"].clone())
            .or_default()
            .push((row["arrival_time"].clone(), row["departure_time"].clone()));
    }
    for (trip, times) in &trips {
        let (first, last) = (&times[0], &times[times.len() - 1]);
        assert!(
            !first.1.is_empty() && !last.0.is_empty(),
            "{trip}: {times:?}"
        );
    }
    let written: Vec<String> = column(feed.path(), "trips.txt", "trip_id");
    assert!(column(feed.path(), "frequencies.txt", "trip_id")
        .iter()
        .all(|trip| written.contains(trip)));
    assert_eq!(
        decision(&report, "trip-untimed-end-cut"),
        Some((1, "sw:ServiceJourney:7-takt".to_owned()))
    );
}

#[test]
fn a_template_journey_without_its_first_call_runs_its_headway_from_its_first_call_written() {
    // With a time at Bergdorf, eight minutes after its first call, the
    // template journey's first call written: each run leaves Bergdorf eight
    // minutes after the run leaves the stop point placed nowhere.
    let delivery = seeweg_from_nowhere(|xml| {
        xml.replace(
            "<TimetabledPassingTime id=\"sw:TimetabledPassingTime:takt-3\" version=\"1\">",
            "<TimetabledPassingTime id=\"sw:TimetabledPassingTime:takt-2\" version=\"1\">
                  <StopPointInJourneyPatternRef ref=\"sw:StopPointInJourneyPattern:7-out-2\" version=\"1\"/>
                  <ArrivalTime>06:08:00</ArrivalTime>
                  <DepartureTime>06:08:00</DepartureTime>
                </TimetabledPassingTime>
                <TimetabledPassingTime id=\"sw:TimetabledPassingTime:takt-3\" version=\"1\">",
        )
    });
    let (feed, _) = convert_to_directory(delivery.path(), &Options::default());
    assert_eq!(
        table(
            feed.path(),
            "frequencies.txt",
            &["trip_id", "start_time", "end_time", "headway_secs"]
        ),
        [
            "sw:ServiceJourney:7-takt|06:08:00|07:08:01|1200",
            "sw:ServiceJourney:7-takt|18:08:00|18:58:01|1200",
        ]
    );
    assert_eq!(
        table(
            feed.path(),
            "stop_times.txt",
            &["trip_id", "stop_id", "arrival_time", "departure_time"]
        )
        .into_iter()
        .filter(|row| row.starts_with("sw:ServiceJourney:7-takt|"))
        .collect::<Vec<_>>(),
        [
            "sw:ServiceJourney:7-takt|sw:StopPlace:bergdorf|06:08:00|06:08:00",
            "sw:ServiceJourney:7-takt|sw:Quay:talwil-1|06:15:00|06:15:00",
        ]
    );
}

#[test]
fn a_trip_names_no_shape_without_two_positions() {
    // Every link stated in a reference system the crate does not know.
    let delivery = seeweg_edited(
        |_, xml| xml.replace("srsName=\"EPSG:2056\"", "srsName=\"EPSG:99999\""),
        &[],
    );
    let (feed, report) = convert_to_directory(delivery.path(), &shapes());
    let points = column(feed.path(), "shapes.txt", "shape_id");
    for shape in column(feed.path(), "trips.txt", "shape_id") {
        assert!(
            shape.is_empty() || points.iter().filter(|&id| *id == shape).count() >= 2,
            "{shape} has fewer than two positions"
        );
    }
    assert_eq!(
        decision(&report, "shape-without-two-positions"),
        Some((2, "sw:ServiceJourney:7-2310".to_owned()))
    );
}

#[test]
fn an_agency_without_a_web_address_takes_the_feed_publisher_s() {
    // No publisher given: the delivery's data source publishes the feed, and
    // its web address is the agency's.
    let delivery = seeweg_edited(
        |_, xml| xml.replace("<Url>https://bahnen.seeweg.example</Url>", ""),
        &[],
    );
    let (feed, report) = convert_to_directory(delivery.path(), &Options::default());
    assert_eq!(
        table(feed.path(), "agency.txt", &["agency_id", "agency_url"]),
        ["sw:Operator:SWB|https://seeweg.example"]
    );
    assert_eq!(
        column(feed.path(), "feed_info.txt", "feed_publisher_url"),
        ["https://seeweg.example"]
    );
    assert!(decision(&report, "agency-without-url").is_some());
}

#[test]
fn an_agency_a_route_and_a_stop_without_a_name_are_named_by_their_identifier() {
    // GTFS requires a name of each; the identifier is the only one stated.
    let delivery = seeweg_edited(
        |_, xml| {
            xml.replace("<Name>Seeweg Bahnen</Name>", "")
                .replace("<Name>Seeweg</Name>", "")
                .replace("<PublicCode>7</PublicCode>", "")
                .replace("<Name>Weide</Name>", "")
        },
        &[],
    );
    let (feed, report) = convert_to_directory(delivery.path(), &Options::default());
    assert_eq!(
        table(feed.path(), "agency.txt", &["agency_id", "agency_name"]),
        ["sw:Operator:SWB|sw:Operator:SWB"]
    );
    assert_eq!(
        table(
            feed.path(),
            "routes.txt",
            &["route_id", "route_short_name", "route_long_name"]
        ),
        ["sw:Line:7||sw:Line:7"]
    );
    assert!(table(feed.path(), "stops.txt", &["stop_id", "stop_name"])
        .contains(&"sw:ScheduledStopPoint:weide|sw:ScheduledStopPoint:weide".to_owned()));
    for code in [
        "agency-named-by-its-identifier",
        "route-named-by-its-identifier",
        "stop-named-by-its-identifier",
    ] {
        assert!(decision(&report, code).is_some(), "{code}");
    }
}

#[test]
fn a_feed_written_again_into_its_directory_holds_only_the_files_of_this_conversion() {
    let feed = tempfile::tempdir().expect("a directory");
    let convert_into = |delivery: &Path, options: &Options| {
        let delivery = Delivery::read(
            &Source::open(delivery).expect("the delivery opens"),
            Reading {
                geometry: options.shapes,
                ..Reading::default()
            },
        )
        .expect("the delivery reads");
        convert(
            &delivery,
            options,
            output::at(feed.path()).expect("an output"),
        )
        .expect("converts")
    };
    let publisher = Publisher {
        name: "Earlier".to_owned(),
        url: "https://earlier.example".to_owned(),
        language: None,
    };
    let mut options = shapes();
    options.publisher = Some(publisher);
    convert_into(&fixture("seeweg"), &options);
    // A file of a GTFS feed another program wrote, and one of no feed.
    std::fs::write(feed.path().join("fare_attributes.txt"), "fare_id,price\n").expect("written");
    std::fs::write(feed.path().join("notes.txt"), "kept\n").expect("written");
    // Without a data source with a web address: no feed_info.txt, nor shapes.
    let delivery = seeweg_edited(
        |_, xml| xml.replace("<Url>https://seeweg.example</Url>", ""),
        &[],
    );
    let report = convert_into(delivery.path(), &Options::default());
    let mut files: Vec<String> = std::fs::read_dir(feed.path())
        .expect("the feed")
        .map(|entry| {
            entry
                .expect("a file")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    files.sort_unstable();
    let mut expected: Vec<String> = report.rows.keys().cloned().collect();
    expected.push("notes.txt".to_owned());
    expected.sort_unstable();
    assert_eq!(files, expected);
}

#[test]
fn a_destination_display_holds_across_a_call_its_pattern_does_not_name() {
    // A journey stating its calls, Weide among them, on the outbound pattern,
    // whose display changes at Bergdorf.
    let journey = r#"<ServiceJourney id="sw:ServiceJourney:7-calls" version="1">
              <dayTypes><DayTypeRef ref="sw:DayType:weekdays" version="1"/></dayTypes>
              <ServiceJourneyPatternRef ref="sw:ServiceJourneyPattern:7-out" version="1"/>
              <LineRef ref="sw:Line:7" version="1"/>
              <calls>
                <Call id="sw:Call:c1" version="1" order="1"><ScheduledStopPointRef ref="sw:ScheduledStopPoint:seehof" version="1"/><Departure><Time>09:00:00</Time></Departure></Call>
                <Call id="sw:Call:c2" version="1" order="2"><ScheduledStopPointRef ref="sw:ScheduledStopPoint:weide" version="1"/><Arrival><Time>09:03:00</Time></Arrival><Departure><Time>09:03:00</Time></Departure></Call>
                <Call id="sw:Call:c3" version="1" order="3"><ScheduledStopPointRef ref="sw:ScheduledStopPoint:bergdorf" version="1"/><Arrival><Time>09:06:00</Time></Arrival><Departure><Time>09:06:00</Time></Departure></Call>
                <Call id="sw:Call:c4" version="1" order="4"><ScheduledStopPointRef ref="sw:ScheduledStopPoint:talwil" version="1"/><Arrival><Time>09:10:00</Time></Arrival></Call>
              </calls>
            </ServiceJourney>
            <TemplateServiceJourney "#;
    let delivery = seeweg_edited(
        |_, xml| xml.replacen("<TemplateServiceJourney ", journey, 1),
        &[],
    );
    let (feed, _) = convert_to_directory(delivery.path(), &Options::default());
    assert_eq!(
        table(feed.path(), "trips.txt", &["trip_id", "trip_headsign"])
            .into_iter()
            .filter(|trip| trip.starts_with("sw:ServiceJourney:7-calls|"))
            .collect::<Vec<_>>(),
        ["sw:ServiceJourney:7-calls|Bergdorf"]
    );
    assert_eq!(
        table(
            feed.path(),
            "stop_times.txt",
            &["trip_id", "stop_id", "stop_headsign"]
        )
        .into_iter()
        .filter(|row| row.starts_with("sw:ServiceJourney:7-calls|"))
        .collect::<Vec<_>>(),
        [
            "sw:ServiceJourney:7-calls|sw:Quay:seehof-1|",
            "sw:ServiceJourney:7-calls|sw:ScheduledStopPoint:weide|",
            "sw:ServiceJourney:7-calls|sw:StopPlace:bergdorf|Talwil Mitte",
            "sw:ServiceJourney:7-calls|sw:Quay:talwil-1|Talwil Mitte",
        ]
    );
}

#[test]
fn a_parent_site_without_a_position_is_no_station_and_said() {
    let delivery = seeweg_edited(
        |_, xml| {
            xml.replace(
                "<Name>Bergdorf Ort</Name>\n              <Centroid><Location><Longitude>8.3202</Longitude><Latitude>47.0601</Latitude></Location></Centroid>",
                "<Name>Bergdorf Ort</Name>",
            )
        },
        &[],
    );
    let (feed, report) = convert_to_directory(delivery.path(), &Options::default());
    let stops = table(feed.path(), "stops.txt", &["stop_id", "parent_station"]);
    assert!(!stops
        .iter()
        .any(|stop| stop.starts_with("sw:StopPlace:bergdorf-ort|")));
    assert!(
        stops.contains(&"sw:StopPlace:bergdorf|".to_owned()),
        "{stops:?}"
    );
    assert_eq!(
        decision(&report, "parent-site-without-position"),
        Some((1, "sw:StopPlace:bergdorf-ort".to_owned()))
    );
}

#[test]
fn a_colour_is_read_in_whatever_prefix_its_namespace_is_written() {
    let delivery = seeweg_edited(
        |_, xml| {
            xml.replace(
                "<Colour>1F6FB2</Colour>",
                "<n:Colour xmlns:n=\"http://www.netex.org.uk/netex\">1F6FB2</n:Colour>",
            )
            .replace(
                "<TextColour>FFFFFF</TextColour>",
                "<n:TextColour xmlns:n=\"http://www.netex.org.uk/netex\">FFFFFF</n:TextColour>",
            )
        },
        &[],
    );
    let (feed, _) = convert_to_directory(delivery.path(), &Options::default());
    assert_eq!(
        table(
            feed.path(),
            "routes.txt",
            &["route_color", "route_text_color"]
        ),
        ["1F6FB2|FFFFFF"]
    );
}

#[test]
fn the_feed_s_version_is_the_latest_publication_in_time() {
    // 06:00 at UTC+2 is 04:00 UTC, an hour before the other document's.
    let delivery = seeweg_edited(
        |name, xml| {
            let published = if name == "seeweg-line-7.xml" {
                "2026-10-10T06:00:00+02:00"
            } else {
                "2026-10-10T05:00:00Z"
            };
            xml.replace(
                "<PublicationTimestamp>2026-10-10T06:00:00</PublicationTimestamp>",
                &format!("<PublicationTimestamp>{published}</PublicationTimestamp>"),
            )
        },
        &[],
    );
    let (feed, _) = convert_to_directory(delivery.path(), &Options::default());
    assert_eq!(
        column(feed.path(), "feed_info.txt", "feed_version"),
        ["2026-10-10T05:00:00Z"]
    );
}

/// The Uferweg delivery, each of its documents as `edit` makes it of its
/// name and text, in a directory of its own.
fn uferweg_edited(edit: impl Fn(&str, &str) -> String) -> tempfile::TempDir {
    let delivery = tempfile::tempdir().expect("a directory");
    for name in ["uferweg-nord.xml", "uferweg-sued.xml"] {
        let xml = std::fs::read_to_string(fixture("uferweg").join(name)).expect("the fixture");
        std::fs::write(delivery.path().join(name), edit(name, &xml)).expect("written");
    }
    delivery
}

/// Uferweg with the documents named by `zones` in the time zone given, the
/// other in Zürich's.
fn uferweg_in(zones: &[(&str, &str)], edit: impl Fn(&str, &str) -> String) -> tempfile::TempDir {
    uferweg_edited(|name, xml| {
        let xml = edit(name, xml);
        match zones.iter().find(|(document, _)| *document == name) {
            Some((_, zone)) => xml.replace(
                "<TimeZone>Europe/Zurich</TimeZone>",
                &format!("<TimeZone>{zone}</TimeZone>"),
            ),
            None => xml,
        }
    })
}

/// The days each service of the feed at `feed` runs on, `YYYYMMDD`: the
/// weekdays of `calendar.txt` from its first date to its last, with the
/// dates of `calendar_dates.txt` added and taken away.
fn service_days(feed: &Path) -> BTreeMap<String, std::collections::BTreeSet<String>> {
    const WEEKDAYS: [&str; 7] = [
        "monday",
        "tuesday",
        "wednesday",
        "thursday",
        "friday",
        "saturday",
        "sunday",
    ];
    let mut days: BTreeMap<String, std::collections::BTreeSet<String>> = BTreeMap::new();
    for row in rows(feed, "calendar.txt") {
        let date = |column: &str| {
            let text = &row[column];
            netex_rs::model::Date::from(
                format!("{}-{}-{}", &text[..4], &text[4..6], &text[6..]).as_str(),
            )
            .days_since_epoch()
            .expect("a date")
        };
        for day in date("start_date")..=date("end_date") {
            // 1970-01-01 was a Thursday.
            let weekday = WEEKDAYS[(day + 3).rem_euclid(7) as usize];
            if row[weekday] == "1" {
                let text = netex_rs::model::Date::from_days_since_epoch(day)
                    .as_str()
                    .replace('-', "");
                days.entry(row["service_id"].clone())
                    .or_default()
                    .insert(text);
            }
        }
    }
    for row in rows(feed, "calendar_dates.txt") {
        let service = days.entry(row["service_id"].clone()).or_default();
        if row["exception_type"] == "1" {
            service.insert(row["date"].clone());
        } else {
            service.remove(&row["date"]);
        }
    }
    days
}

/// The days `YYYYMMDD` from `first` to `last` of one month, `YYYYMM`.
fn days_of(month: &str, first: u32, last: u32) -> Vec<String> {
    (first..=last)
        .map(|day| format!("{month}{day:02}"))
        .collect()
}

/// The days each trip of the feed at `feed` runs on.
fn trip_days(feed: &Path) -> BTreeMap<String, Vec<String>> {
    let services = service_days(feed);
    rows(feed, "trips.txt")
        .into_iter()
        .map(|trip| {
            let days = services[&trip["service_id"]].iter().cloned().collect();
            (trip["trip_id"].clone(), days)
        })
        .collect()
}

#[test]
fn an_agency_in_another_zone_runs_at_its_instants_in_the_zone_of_the_feed() {
    // Greenland's buses in Denmark's delivery: Nuuk three hours behind
    // Copenhagen, both keeping summer time on the same days. Here Zürich's
    // delivery, as Copenhagen's offsets are.
    let delivery = uferweg_in(&[("uferweg-sued.xml", "America/Nuuk")], |_, xml| {
        xml.to_owned()
    });
    let (feed, report) = convert_to_directory(delivery.path(), &Options::default());
    assert_eq!(
        table(feed.path(), "agency.txt", &["agency_id", "agency_timezone"]),
        [
            "uw:Operator:nord|Europe/Zurich",
            "uw:Operator:sued|Europe/Zurich"
        ]
    );
    assert_eq!(
        table(
            feed.path(),
            "stop_times.txt",
            &["trip_id", "arrival_time", "departure_time"]
        ),
        [
            "uw:ServiceJourney:N-0800|08:00:00|08:00:00",
            "uw:ServiceJourney:N-0800|08:10:00|08:10:00",
            "uw:ServiceJourney:S-0815|11:15:00|11:15:00",
            "uw:ServiceJourney:S-0815|11:30:00|11:30:00",
        ]
    );
    let days = trip_days(feed.path());
    assert_eq!(days["uw:ServiceJourney:S-0815"], days_of("202610", 12, 16));
    // Its stops are in Nuuk: their stations say so, their quays stand under them.
    assert_eq!(
        table(feed.path(), "stops.txt", &["stop_id", "stop_timezone"]),
        [
            "uw:StopPlace:anger|America/Nuuk",
            "uw:StopPlace:hafen|",
            "uw:StopPlace:muehle|",
            "uw:StopPlace:steg|America/Nuuk",
            "uw:Quay:anger-1|",
            "uw:Quay:hafen-1|",
            "uw:Quay:muehle-1|",
            "uw:Quay:steg-1|",
        ]
    );
    assert_eq!(
        decision(&report, "agency-time-zone-unified"),
        Some((1, "uw:Operator:sued: America/Nuuk".to_owned()))
    );
    assert_eq!(
        decision(&report, "trip-times-in-feed-zone"),
        Some((1, "uw:ServiceJourney:S-0815".to_owned()))
    );
    assert_eq!(decision(&report, "trip-split-by-offset"), None);
}

#[test]
fn a_trip_whose_zone_keeps_summer_time_on_other_days_is_split_by_its_days() {
    // New York begins its summer time on 14 March 2027, Zürich on 28 March:
    // six hours apart before and after, five between.
    let delivery = uferweg_in(&[("uferweg-sued.xml", "America/New_York")], |name, xml| {
        if name == "uferweg-sued.xml" {
            xml.replace("2026-10-12T00:00:00", "2027-03-08T00:00:00")
                .replace("2026-10-16T00:00:00", "2027-03-31T00:00:00")
                .replace(
                    "<ValidDayBits>11111</ValidDayBits>",
                    &format!("<ValidDayBits>{}</ValidDayBits>", "1".repeat(24)),
                )
        } else {
            xml.to_owned()
        }
    });
    let (feed, report) = convert_to_directory(delivery.path(), &Options::default());
    let times: Vec<String> = table(
        feed.path(),
        "stop_times.txt",
        &["trip_id", "departure_time"],
    )
    .into_iter()
    .filter(|row| row.starts_with("uw:ServiceJourney:S-"))
    .collect();
    assert_eq!(
        times,
        [
            "uw:ServiceJourney:S-0815|14:15:00",
            "uw:ServiceJourney:S-0815|14:30:00",
            "uw:ServiceJourney:S-0815#2|13:15:00",
            "uw:ServiceJourney:S-0815#2|13:30:00",
        ]
    );
    let days = trip_days(feed.path());
    let mut six_hours = days_of("202703", 8, 13);
    six_hours.extend(days_of("202703", 28, 31));
    assert_eq!(days["uw:ServiceJourney:S-0815"], six_hours);
    assert_eq!(
        days["uw:ServiceJourney:S-0815#2"],
        days_of("202703", 14, 27)
    );
    assert_eq!(days["uw:ServiceJourney:N-0800"], days_of("202610", 12, 16));
    // The interchange to the trip holds for each of its parts.
    assert_eq!(
        table(
            feed.path(),
            "transfers.txt",
            &["from_trip_id", "to_trip_id", "transfer_type"]
        )
        .into_iter()
        .filter(|row| !row.starts_with('|'))
        .collect::<Vec<_>>(),
        [
            "uw:ServiceJourney:N-0800|uw:ServiceJourney:S-0815|1",
            "uw:ServiceJourney:N-0800|uw:ServiceJourney:S-0815#2|1",
        ]
    );
    assert_eq!(
        decision(&report, "trip-split-by-offset"),
        Some((1, "uw:ServiceJourney:S-0815".to_owned()))
    );
    assert_eq!(decision(&report, "trip-made-unique"), None);
    assert_eq!(
        report.period,
        Some(("20261012".to_owned(), "20270331".to_owned()))
    );
}

#[test]
fn a_trip_beginning_before_its_day_in_the_zone_of_the_feed_runs_on_the_day_before() {
    // The feed in New York's zone; a trip at 05:15 in Zürich is at 23:15 of
    // the day before there.
    let delivery = uferweg_in(&[("uferweg-nord.xml", "America/New_York")], |name, xml| {
        if name == "uferweg-sued.xml" {
            xml.replace("08:15:00", "05:15:00")
                .replace("08:30:00", "05:30:00")
        } else {
            xml.to_owned()
        }
    });
    let (feed, report) = convert_to_directory(delivery.path(), &Options::default());
    assert_eq!(
        column(feed.path(), "agency.txt", "agency_timezone"),
        ["America/New_York", "America/New_York"]
    );
    let times: Vec<String> = table(feed.path(), "stop_times.txt", &["trip_id", "arrival_time"])
        .into_iter()
        .filter(|row| row.starts_with("uw:ServiceJourney:S-"))
        .collect();
    assert_eq!(
        times,
        [
            "uw:ServiceJourney:S-0815|23:15:00",
            "uw:ServiceJourney:S-0815|23:30:00"
        ]
    );
    assert_eq!(
        trip_days(feed.path())["uw:ServiceJourney:S-0815"],
        days_of("202610", 11, 15)
    );
    assert_eq!(
        column(feed.path(), "stops.txt", "stop_timezone")
            .into_iter()
            .filter(|zone| !zone.is_empty())
            .collect::<Vec<_>>(),
        ["Europe/Zurich", "Europe/Zurich"]
    );
    assert_eq!(
        decision(&report, "trip-times-in-feed-zone"),
        Some((1, "uw:ServiceJourney:S-0815".to_owned()))
    );
}

#[test]
fn a_name_the_time_zone_database_does_not_hold_is_no_time_zone() {
    let delivery = seeweg_edited(
        |_, xml| {
            xml.replace(
                "<TimeZone>Europe/Zurich</TimeZone>",
                "<TimeZone>Europe/Nowhere</TimeZone>",
            )
        },
        &[],
    );
    let (feed, report) = convert_to_directory(delivery.path(), &Options::default());
    assert_eq!(
        column(feed.path(), "agency.txt", "agency_timezone"),
        ["Europe/Zurich"]
    );
    assert_eq!(
        decision(&report, "time-zone-not-a-name"),
        Some((1, "seeweg-line-7.xml: Europe/Nowhere".to_owned()))
    );
    // A zone given for the feed must be one, too.
    let delivery = uferweg_edited(|_, xml| xml.replace("<TimeZone>Europe/Zurich</TimeZone>", ""));
    let source = Source::open(delivery.path()).expect("the delivery opens");
    let delivery = Delivery::read(&source, Reading::default()).expect("the delivery reads");
    let feed = tempfile::tempdir().expect("a directory");
    let mut options = Options::default();
    options.time_zone = Some("Atlantic/Nowhere".to_owned());
    let converted = convert(
        &delivery,
        &options,
        output::at(feed.path()).expect("an output"),
    );
    let error = converted.expect_err("no zone of the database");
    assert!(error.to_string().contains("Atlantic/Nowhere"), "{error}");
}

#[test]
fn a_position_off_the_earth_or_at_zero_zero_is_no_position() {
    // Sweden's register states a latitude of 56162332, and a stop at 0, 0.
    let delivery = seeweg_edited(
        |name, xml| {
            if name != "seeweg-line-7.xml" {
                return xml.to_owned();
            }
            xml.replace(
                "<Longitude>8.3001</Longitude><Latitude>47.0501</Latitude>",
                "<Longitude>8.3001</Longitude><Latitude>56162332</Latitude>",
            )
            .replace(
                "<Longitude>8.3401</Longitude><Latitude>47.0701</Latitude>",
                "<Longitude>0</Longitude><Latitude>0</Latitude>",
            )
        },
        &[],
    );
    let (feed, report) = convert_to_directory(delivery.path(), &Options::default());
    let positions: BTreeMap<String, String> = table(
        feed.path(),
        "stops.txt",
        &["stop_id", "stop_lat", "stop_lon"],
    )
    .into_iter()
    .map(|row| {
        let (id, position) = row.split_once('|').expect("three columns");
        (id.to_owned(), position.to_owned())
    })
    .collect();
    // Each quay is placed at its stop place.
    assert_eq!(positions["sw:Quay:seehof-1"], "47.0500000|8.3000000");
    assert_eq!(positions["sw:Quay:talwil-1"], "47.0700000|8.3400000");
    assert_eq!(
        decision(&report, "position-off-the-earth"),
        Some((1, "sw:Quay:seehof-1".to_owned()))
    );
    assert_eq!(
        decision(&report, "position-at-zero-zero"),
        Some((1, "sw:Quay:talwil-1".to_owned()))
    );
    assert_eq!(
        decision(&report, "quay-at-its-stop-place").map(|d| d.0),
        Some(3)
    );
}

#[test]
fn a_web_address_without_its_scheme_is_no_web_address() {
    // The Austrian and Norwegian deliveries write `www.…` and `//…`.
    let delivery = seeweg_edited(
        |_, xml| {
            xml.replace(
                "<Url>https://bahnen.seeweg.example</Url>",
                "<Url>bahnen.seeweg.example</Url>",
            )
        },
        &[],
    );
    let (feed, report) = convert_to_directory(delivery.path(), &Options::default());
    assert_eq!(
        table(feed.path(), "agency.txt", &["agency_id", "agency_url"]),
        ["sw:Operator:SWB|https://seeweg.example"],
        "the publisher's, the delivery's data source"
    );
    assert_eq!(
        decision(&report, "url-without-scheme"),
        Some((1, "sw:Operator:SWB: bahnen.seeweg.example".to_owned()))
    );
    assert!(decision(&report, "agency-without-url").is_some());
    let delivery = seeweg_edited(
        |_, xml| {
            xml.replace(
                "<Url>https://seeweg.example</Url>",
                "<Url>//seeweg.example</Url>",
            )
        },
        &[],
    );
    let (feed, report) = convert_to_directory(delivery.path(), &Options::default());
    assert!(
        !feed.path().join("feed_info.txt").exists(),
        "a data source without a web address publishes no feed"
    );
    assert_eq!(
        table(
            feed.path(),
            "attributions.txt",
            &["organization_name", "attribution_url"]
        ),
        ["Seeweg Verkehrsverbund|", "SEEWEG|"]
    );
    assert_eq!(
        decision(&report, "url-without-scheme"),
        Some((1, "Seeweg Verkehrsverbund: //seeweg.example".to_owned()))
    );
}

#[test]
fn a_level_numbered_by_no_finite_number_has_no_number() {
    let delivery = seeweg_edited(
        |_, xml| {
            xml.replace(
                "<Name>Strassenebene</Name><PublicCode>0</PublicCode>",
                "<Name>Strassenebene</Name><PublicCode>NaN</PublicCode>",
            )
        },
        &[],
    );
    let (feed, report) = convert_to_directory(delivery.path(), &Options::default());
    assert!(rows(feed.path(), "levels.txt").is_empty());
    assert!(column(feed.path(), "stops.txt", "level_id")
        .iter()
        .all(String::is_empty));
    assert_eq!(
        decision(&report, "level-without-number"),
        Some((1, "sw:Quay:seehof-1".to_owned()))
    );
}

#[test]
fn a_day_beyond_the_reach_of_the_time_zone_rules_is_left_out_and_said() {
    // The rules reach instants up to 9999-12-30: noon of the last day of
    // the calendar in New York is beyond them.
    let delivery = uferweg_in(&[("uferweg-sued.xml", "America/New_York")], |name, xml| {
        if name == "uferweg-sued.xml" {
            xml.replace("2026-10-12T00:00:00", "9999-12-29T00:00:00")
                .replace("2026-10-16T00:00:00", "9999-12-31T00:00:00")
                .replace(
                    "<ValidDayBits>11111</ValidDayBits>",
                    "<ValidDayBits>111</ValidDayBits>",
                )
        } else {
            xml.to_owned()
        }
    });
    let (feed, report) = convert_to_directory(delivery.path(), &Options::default());
    assert_eq!(
        trip_days(feed.path())["uw:ServiceJourney:S-0815"],
        ["99991229", "99991230"]
    );
    assert_eq!(
        decision(&report, "trip-days-beyond-zone-rules"),
        Some((1, "uw:ServiceJourney:S-0815".to_owned()))
    );
}
