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

fn seeweg() -> (tempfile::TempDir, Report) {
    convert_to_directory(
        &fixture("seeweg"),
        &Options {
            shapes: true,
            ..Options::default()
        },
    )
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
        [
            "sw:StopPlace:seehof|Seehof|47.0500000|8.3000000|1||0||",
            // The parent site of Bergdorf, which is called at itself.
            "sw:StopPlace:bergdorf-ort|Bergdorf Ort|47.0601000|8.3202000|1||0||",
            "sw:StopPlace:talwil|Talwil|47.0700000|8.3400000|1||0||",
            "sw:Quay:seehof-1|Seehof|47.0501000|8.3001000|0|sw:StopPlace:seehof|1|1|sw:Level:seehof-0",
            // Without a position of its own: at its stop place.
            "sw:Quay:seehof-2|Seehof|47.0500000|8.3000000|0|sw:StopPlace:seehof|0|2|",
            // Called at itself, without quays: a stop, under its parent site.
            "sw:StopPlace:bergdorf|Bergdorf|47.0600000|8.3200000|0|sw:StopPlace:bergdorf-ort|2||",
            "sw:Quay:talwil-1|Talwil|47.0701000|8.3401000|0|sw:StopPlace:talwil|0||",
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
    // minutes" —, as GTFS frequencies without exact times do; the group
    // whose last departure is not a whole number of intervals after its
    // first is written as stated too.
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
            "sw:ServiceJourney:7-takt|06:00:00|07:00:00|1200|0",
            "sw:ServiceJourney:7-takt|18:00:00|18:50:00|1200|0",
        ]
    );
    assert!(!report
        .decisions
        .iter()
        .any(|decision| decision.code == "headway-written-as-runs"));
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
    let options = Options {
        stop_ids: StopIds::Global,
        publisher: Some(Publisher {
            name: "Example".to_owned(),
            url: "https://example.org".to_owned(),
            language: Some("en".to_owned()),
        }),
        ..Options::default()
    };
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
            "de:99001:1|",
            "de:99001:2|",
            "ch:1:sloid:99003|",
            "de:99001:1:0:1|de:99001:1",
            "de:99001:1:0:1#2|de:99001:1",
            "sw:StopPlace:bergdorf|de:99001:2",
            "de:99001:3|ch:1:sloid:99003",
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
        .find(|decision| decision.code == "time-zone-of-profile")
        .expect("the decision is reported");
    assert_eq!(decision.example.as_deref(), Some("Europe/Zurich"));
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
