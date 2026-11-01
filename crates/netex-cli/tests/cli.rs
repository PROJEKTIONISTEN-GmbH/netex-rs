//! The `netex` command as a user runs it: its reports, their schema, and its
//! exit status.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde_json::Value;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn netex(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_netex"))
        .args(arguments)
        .output()
        .expect("the command runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("UTF-8")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("UTF-8")
}

fn defective() -> String {
    fixture("mangelhaft").display().to_string()
}

/// The report of the defective delivery as JSON, checked against its schema.
fn checked(extra: &[&str]) -> (Value, Option<i32>) {
    let source = defective();
    let mut arguments = vec!["validate", source.as_str(), "--format", "json"];
    arguments.extend_from_slice(extra);
    let output = netex(&arguments);
    let report: Value = serde_json::from_str(&stdout(&output)).expect("the report is JSON");
    let schema: Value = serde_json::from_str(include_str!("../schema/validate-report.schema.json"))
        .expect("the schema is JSON");
    let validator = jsonschema::validator_for(&schema).expect("a valid schema");
    let errors: Vec<String> = validator
        .iter_errors(&report)
        .map(|error| format!("{error} at {}", error.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "the report does not follow its schema: {errors:#?}"
    );
    (report, output.status.code())
}

/// The class of the report named `name`.
fn class<'r>(report: &'r Value, name: &str) -> &'r Value {
    report["classes"]
        .as_array()
        .expect("classes")
        .iter()
        .find(|class| class["class"] == name)
        .unwrap_or_else(|| panic!("no class {name} in {report:#}"))
}

#[test]
fn every_defect_is_a_class_with_its_count_examples_and_the_journeys_it_affects() {
    let (report, status) = checked(&[]);
    assert_eq!(
        status,
        Some(0),
        "nothing is refused unless the policy says so"
    );
    assert_eq!(report["refused"], false);
    assert_eq!(report["counts"]["documents"], 2);
    assert_eq!(report["counts"]["journeys"], 4);
    let classes: Vec<&str> = report["classes"]
        .as_array()
        .expect("classes")
        .iter()
        .map(|class| class["class"].as_str().expect("a name"))
        .collect();
    assert_eq!(
        classes,
        [
            "unresolved",
            "redefined-otherwise",
            "journey-without-days",
            "journey-without-travel-time",
            "time-goes-back",
            "stop-point-without-place",
            "stop-point-without-position",
            "unreferenced",
        ]
    );
    // The quay nothing defines leaves its stop point unplaced, and every
    // journey of the pattern calling there is affected.
    let unresolved = class(&report, "unresolved");
    assert_eq!(unresolved["count"], 2);
    assert_eq!(unresolved["affected_journeys"], 4);
    let quay = &unresolved["examples"][0];
    assert_eq!(quay["element"], "PassengerStopAssignment");
    assert_eq!(quay["entity"], "m:PassengerStopAssignment:irgendwo");
    assert_eq!(quay["about"], "<QuayRef> m:Quay:fehlt");
    assert_eq!(quay["affected_journeys"], 4);
    assert_eq!(
        quay["affected_examples"],
        serde_json::json!([
            "m:ServiceJourney:gut",
            "m:ServiceJourney:flach",
            "m:ServiceJourney:zurueck",
            "m:ServiceJourney:ohne-tage"
        ])
    );
    let day_type = &unresolved["examples"][1];
    assert_eq!(day_type["entity"], "m:ServiceJourney:ohne-tage");
    assert_eq!(day_type["affected_journeys"], 1);
    let redefined = class(&report, "redefined-otherwise");
    assert_eq!(redefined["examples"][0]["entity"], "m:StopPlace:zwei");
    assert_eq!(redefined["examples"][0]["document"], "b-line-2.xml");
    assert_eq!(redefined["examples"][0]["about"], "a-line-1.xml");
    assert_eq!(redefined["affected_journeys"], 4);
    let flat = class(&report, "journey-without-travel-time");
    assert_eq!(flat["examples"][0]["entity"], "m:ServiceJourney:flach");
    assert_eq!(flat["affected_journeys"], Value::Null);
    assert_eq!(
        class(&report, "time-goes-back")["examples"][0]["entity"],
        "m:ServiceJourney:zurueck"
    );
    let unplaced = class(&report, "stop-point-without-place");
    assert_eq!(
        unplaced["examples"][0]["entity"],
        "m:ScheduledStopPoint:irgendwo"
    );
    assert_eq!(unplaced["examples"][0]["document"], Value::Null);
    let unreferenced: Vec<String> = class(&report, "unreferenced")["examples"]
        .as_array()
        .expect("examples")
        .iter()
        .map(|example| format!("{} {}", example["element"], example["entity"]).replace('"', ""))
        .collect();
    assert_eq!(
        unreferenced,
        ["StopPlace m:StopPlace:brache", "Line m:Line:2"]
    );
}

#[test]
fn the_policy_refuses_or_tolerates_by_class_and_the_exit_status_says_which() {
    let (report, status) = checked(&["--refuse", "unresolved"]);
    assert_eq!(status, Some(1));
    assert_eq!(report["refused"], true);
    assert_eq!(class(&report, "unresolved")["refused"], true);
    assert_eq!(class(&report, "unresolved")["limit"], 0);
    let (_, status) = checked(&["--at-most", "unresolved=2", "--at-most", "time-goes-back=1"]);
    assert_eq!(status, Some(0), "as many as tolerated");
    let (report, status) = checked(&["--at-most", "journey-without-travel-time=0"]);
    assert_eq!(
        status,
        Some(1),
        "a check of the report's own refuses as a concern does"
    );
    assert_eq!(
        class(&report, "journey-without-travel-time")["refused"],
        true
    );
    let (report, status) = checked(&["--strict"]);
    assert_eq!(status, Some(1));
    // A class the policy limits is listed though it has no finding.
    assert_eq!(class(&report, "passed-through")["count"], 0);
    assert_eq!(class(&report, "passed-through")["refused"], false);
}

#[test]
fn the_human_report_names_each_class_its_examples_and_the_verdict() {
    let source = defective();
    let output = netex(&[
        "validate",
        &source,
        "--examples",
        "1",
        "--refuse",
        "redefined-otherwise",
    ]);
    assert_eq!(output.status.code(), Some(1));
    let text = stdout(&output);
    assert!(
        text.starts_with("mangelhaft: 2 documents, 4 stops, 3 stop points, 2 lines, 4 journeys"),
        "{text}"
    );
    assert!(
        text.contains("2 × unresolved — reference to nothing, 4 journeys affected"),
        "{text}"
    );
    assert!(text.contains("    PassengerStopAssignment m:PassengerStopAssignment:irgendwo in a-line-1.xml (<QuayRef> m:Quay:fehlt) — 4 journeys"), "{text}");
    assert!(
        !text.contains("m:ServiceJourney:ohne-tage in a-line-1.xml (<DayTypeRef>"),
        "one example only: {text}"
    );
    assert!(text.contains("REFUSED 1 × redefined-otherwise"), "{text}");
    assert!(text.ends_with("refused\n"), "{text}");
}

#[test]
fn a_report_file_holds_what_is_printed_as_json() {
    let source = defective();
    let directory = tempfile::tempdir().expect("a directory");
    let path = directory.path().join("report.json");
    let output = netex(&[
        "validate",
        &source,
        "--report",
        path.to_str().expect("a path"),
    ]);
    assert_eq!(output.status.code(), Some(0));
    let written: Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("the report")).expect("JSON");
    let (printed, _) = checked(&[]);
    assert_eq!(written, printed);
}

#[test]
fn what_cannot_be_read_or_understood_fails_with_status_2() {
    let source = defective();
    let output = netex(&["validate", &source, "--refuse", "no-such-class"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output)
        .contains("no class `no-such-class`; the classes are unresolved, redefined-otherwise"));
    let output = netex(&["validate", &source, "--at-most", "unresolved"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("`unresolved` is no CLASS=COUNT"));
    let output = netex(&["validate", "/no/such/delivery"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).starts_with("netex: "));
}

#[test]
fn statistics_count_by_operator_over_the_whole_calendar_or_a_period() {
    let source = defective();
    let output = netex(&["stats", &source, "--format", "json"]);
    assert_eq!(output.status.code(), Some(0));
    let stats: Value = serde_json::from_str(&stdout(&output)).expect("JSON");
    // Three journeys run, on the five weekdays; the one without days does not.
    assert_eq!(stats["total"]["journeys"], 3);
    assert_eq!(stats["total"]["journey_days"], 15);
    assert_eq!(stats["total"]["lines"], 1);
    assert_eq!(stats["total"]["stops"], 3);
    assert_eq!(stats["total"]["first_day"], "2026-10-12");
    assert_eq!(stats["total"]["last_day"], "2026-10-16");
    // The line names no operator: one group without one.
    assert_eq!(stats["operators"].as_array().expect("operators").len(), 1);
    assert_eq!(stats["operators"][0]["operator"], Value::Null);
    let output = netex(&[
        "stats",
        &source,
        "--from",
        "2026-10-14",
        "--to",
        "2026-10-14",
        "--format",
        "json",
    ]);
    let stats: Value = serde_json::from_str(&stdout(&output)).expect("JSON");
    assert_eq!(stats["total"]["journey_days"], 3);
    assert_eq!(
        stats["period"],
        serde_json::json!(["2026-10-14", "2026-10-14"])
    );
    let output = netex(&["stats", &source, "--from", "14.10.2026"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr(&output).contains("`14.10.2026` is no date"));
}

/// netex-rs's own example of the Nordic profile, and the stop register it
/// names, published apart.
fn nordic() -> (String, String) {
    let own = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/own");
    (
        own.join("nordic").display().to_string(),
        own.join("nordic-stops").display().to_string(),
    )
}

#[test]
fn a_stop_register_published_apart_is_read_along_with_the_delivery() {
    let (delivery, register) = nordic();
    let unresolved = |arguments: &[&str]| {
        let output = netex(arguments);
        assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
        let report: Value = serde_json::from_str(&stdout(&output)).expect("the report is JSON");
        report["classes"]
            .as_array()
            .expect("classes")
            .iter()
            .find(|class| class["class"] == "unresolved")
            .map_or(0, |class| class["count"].as_u64().expect("a count"))
    };
    // Alone, the delivery's stop assignments name quays it does not define.
    assert_eq!(unresolved(&["validate", &delivery, "--format", "json"]), 5);
    assert_eq!(
        unresolved(&["validate", &delivery, "--with", &register, "--format", "json"]),
        0
    );
    // Two sources read along under one name would give documents one name.
    let output = netex(&[
        "validate", &delivery, "--with", &register, "--with", &register,
    ]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("another source read along is named nordic-stops too"),
        "{}",
        stderr(&output)
    );
    let output = netex(&["stats", &delivery, "--with", &register, "--format", "json"]);
    let stats: Value = serde_json::from_str(&stdout(&output)).expect("JSON");
    assert_eq!(stats["total"]["journeys"], 4);
    assert_eq!(stats["total"]["stops"], 5);
}

#[test]
fn a_delivery_converts_to_gtfs_with_its_report() {
    let source = defective();
    let directory = tempfile::tempdir().expect("a directory");
    let feed = directory.path().join("feed");
    let feed = feed.to_str().expect("a path");
    // The delivery names no time zone: GTFS needs one.
    let output = netex(&["convert", &source, "--to", "gtfs", feed]);
    assert_eq!(output.status.code(), Some(2));
    assert!(
        stderr(&output).contains("names a time zone"),
        "{}",
        stderr(&output)
    );
    let report_path = directory.path().join("conversion.json");
    let output = netex(&[
        "convert",
        &source,
        "--to",
        "gtfs",
        feed,
        "--time-zone",
        "Europe/Berlin",
        "--publisher-name",
        "Test",
        "--publisher-url",
        "https://test.example",
        "--report",
        report_path.to_str().expect("a path"),
    ]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(&report_path).expect("the report"))
            .expect("JSON");
    // The unplaced stop point has no position: its calls are left out.
    let decisions: Vec<String> = report["decisions"]
        .as_array()
        .expect("decisions")
        .iter()
        .map(|decision| {
            format!(
                "{} {}",
                decision["code"].as_str().expect("a code"),
                decision["count"]
            )
        })
        .collect();
    assert!(
        decisions.contains(&"call-without-place 3".to_owned()),
        "{decisions:?}"
    );
    assert_eq!(report["rows"]["trips.txt"], 3);
    let agencies = std::fs::read_to_string(Path::new(feed).join("agency.txt")).expect("agency.txt");
    assert_eq!(
        agencies,
        "agency_id,agency_name,agency_url,agency_timezone,agency_phone\npublisher,Test,https://test.example,Europe/Berlin,\n"
    );
    assert!(stderr(&output).contains("trips.txt: 3 rows"));
}

#[test]
fn the_merge_writes_each_delivery_under_its_rank_and_label() {
    // An own example of netex-rs, where the repository has it: the package of
    // netex-cli does not carry the core's fixtures.
    let at = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/own/at");
    if !at.is_dir() {
        eprintln!(
            "SKIPPED: netex-rs's own examples are not at {}",
            at.display()
        );
        return;
    }
    let to = tempfile::tempdir().expect("a directory");
    let out = to.path().join("merged");
    let report = to.path().join("report.json");
    let (oebb, vor) = (
        format!("oebb={}", at.display()),
        format!("vor={}", at.display()),
    );
    let (out_arg, report_arg) = (out.display().to_string(), report.display().to_string());
    let output = netex(&[
        "merge",
        &oebb,
        &vor,
        "--to",
        &out_arg,
        "--report",
        &report_arg,
    ]);
    let printed = stdout(&output);
    assert_eq!(output.status.code(), Some(0), "{printed}");
    assert!(printed.contains("stop-paired-by-identifier"), "{printed}");
    assert!(out.join("1-oebb").is_dir());
    let report: Value =
        serde_json::from_str(&std::fs::read_to_string(report).expect("the report")).expect("JSON");
    assert_eq!(report["deliveries"][1]["label"], "vor");
}
