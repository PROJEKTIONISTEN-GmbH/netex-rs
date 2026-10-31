//! The crate's own examples, one small delivery for each profile it is used
//! with most: a line offer of the European Passenger Information Profile
//! (EPIP), a line offer of the Italian profile of South Tyrol (IT-ITH1), two
//! line documents of an Austrian-style delivery, the shared and the
//! timetable document of a Swiss-style one, and the four documents of a
//! delivery in the French profile (NeTEx France). They are written for these
//! tests and licensed as the crate is (`tests/fixtures/own/README.md`), so that
//! the cases each profile poses are pinned without data under a licence of its
//! own: identifiers that are each document's own, lines in versions, journeys
//! stating their calls, template journeys run at a headway, names in several
//! languages, patterns naming their line only through their route, and
//! everything in general frames — quays defined beside their stop place,
//! passing times naming no point, day types given their days by a validity and
//! days of the week.
//!
//! Every example is read four ways — the full model one document after the
//! other, in the reverse order, the lean stream one builder per document, and
//! the dataset reader over the directory — and the four indexes must hold the
//! same content before anything is asserted about one of them.

mod support;

use std::collections::BTreeSet;

use netex_rs::dataset::Source;
use netex_rs::index::{CallUse, Concern, Headway, Index, StopKind, Transfer};
use netex_rs::model::{Date, Submode};
use netex_rs::stream::{Entity, FrameKind, Kind, ReadOptions, Selection};
use netex_rs::Release;
use support::render::{assert_same, render};
use support::timetable::{merged, reversed, serially};
use support::{compare, parse, validate_all, Candidate, VALIDATOR_MISSING};

/// The documents of one example delivery, by file name, in name order.
fn documents(profile: &str) -> Vec<(String, String)> {
    support::own_fixtures(profile)
        .into_iter()
        .map(|fixture| (fixture.name, fixture.xml))
        .collect()
}

/// The index of one example delivery, after checking that every way of
/// reading it gives the same.
fn index(profile: &str) -> Index {
    let documents = documents(profile);
    let directory = support::fixtures_dir().join("own").join(profile);
    let source = Source::open(&directory).expect("the directory opens");
    let streamed = Index::build(&source, &ReadOptions::new(Selection::routing()))
        .expect("the directory indexes");
    let full = serially(&documents);
    let expected = render(&full);
    assert_same(
        &expected,
        &render(&serially(&reversed(&documents))),
        "in reverse order",
    );
    assert_same(&expected, &render(&merged(&documents)), "lean, merged");
    assert_same(&expected, &render(&streamed), "from the directory");
    full
}

fn day(text: &str) -> i64 {
    Date::from(text).days_since_epoch().expect("a date")
}

/// The days of October 2026 a journey runs on.
fn october(index: &Index, journey: &str) -> Vec<i64> {
    let id = index.journey_by_id(journey).expect("the journey");
    index.days_of(id).map(|d| d - day("2026-09-30")).collect()
}

fn findings(index: &Index, concern: Concern) -> Vec<String> {
    index
        .findings()
        .filter(|finding| finding.concern == concern)
        .map(|finding| finding.to_string())
        .collect()
}

/// The names of the stop points a journey calls at, in order.
fn calls<'a>(index: &'a Index, journey: &str) -> Vec<&'a str> {
    let id = index.journey_by_id(journey).expect("the journey");
    index
        .journey(id)
        .stop_points
        .iter()
        .map(|&point| index.stop_point(point).name)
        .collect()
}

/// Arrival and departure at each call, in seconds since midnight of the
/// operating day.
fn times(index: &Index, journey: &str) -> Vec<(Option<u32>, Option<u32>)> {
    let id = index.journey_by_id(journey).expect("the journey");
    index
        .journey(id)
        .passing_times
        .iter()
        .map(|time| (time.arrival, time.departure))
        .collect()
}

const fn at(hours: u32, minutes: u32) -> u32 {
    hours * 3600 + minutes * 60
}

#[test]
fn every_own_example_reads_and_writes_back_element_for_element() {
    let fixtures = support::own_fixtures("");
    assert_eq!(
        fixtures.len(),
        10,
        "EPIP 1, Italy 1, Austria 2, Switzerland 2, France 4"
    );
    for fixture in &fixtures {
        let document = netex_rs::read(&fixture.xml)
            .unwrap_or_else(|error| panic!("{}: {error}", fixture.name));
        if let Err(difference) = compare(&parse(&fixture.xml), &parse(&document.write())) {
            panic!("{}: {difference}", fixture.name);
        }
        for release in [Release::V1, Release::V2] {
            let written = document.write_for(release);
            assert!(
                written.report.is_lossless(),
                "{} written for {release:?}: {:?}",
                fixture.name,
                written
                    .report
                    .losses()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            );
        }
    }
}

/// The examples are valid against the schemas of both releases, as written
/// and as the crate writes them for each — but for two documents. The Swiss
/// timetable document names what the shared document defines: an XML Schema
/// key reference cannot reach into another document, so each of its
/// references to the shared document is a complaint, and nothing else is. The
/// French line document states passing times as NeTEx 1.x allows them, the
/// release its profile is written for — without the point they are at, and
/// without an identifier —, which NeTEx 2.0 requires: against 2.0 its passing
/// times are its only complaints.
#[test]
fn every_own_example_is_valid_for_both_releases_but_for_references_into_another_document() {
    let shared = documents("ch")
        .into_iter()
        .find(|(name, _)| name == "ch-service.xml")
        .expect("the shared Swiss document")
        .1;
    for release in [Release::V1, Release::V2] {
        if !support::official_fixtures(release) {
            continue;
        }
        assert!(support::validator_available(), "{VALIDATOR_MISSING}");
        let mut candidates = Vec::new();
        for fixture in support::own_fixtures("") {
            let written = netex_rs::read(&fixture.xml)
                .expect("the example reads")
                .write_for(release)
                .xml;
            candidates.push(Candidate {
                name: format!("{} written for {release:?}", fixture.name),
                xml: written,
            });
            candidates.push(Candidate {
                name: fixture.name,
                xml: fixture.xml,
            });
        }
        let mut complaints = validate_all(release, &candidates);
        let french = [
            "fr/line_10.xml".to_owned(),
            format!("fr/line_10.xml written for {release:?}"),
        ];
        for name in &french {
            let complaint = complaints.remove(name);
            if release == Release::V1 {
                assert_eq!(complaint, None, "{name} is valid for NeTEx 1.x");
                continue;
            }
            let complaint = complaint.unwrap_or_else(|| panic!("{name} is not valid for 2.0"));
            for line in complaint.lines().filter(|l| *l != "fails to validate") {
                assert!(
                    ["TimetabledPassingTime", "ArrivalTime", "DepartureTime"]
                        .iter()
                        .any(|element| line.contains(&format!("netex}}{element}'"))),
                    "not about a passing time: {line}"
                );
            }
        }
        let named: Vec<&String> = complaints.keys().collect();
        assert_eq!(
            named,
            [
                "ch/ch-timetable.xml",
                &format!("ch/ch-timetable.xml written for {release:?}")
            ],
            "{complaints:#?}"
        );
        for complaint in complaints.values() {
            let mut references = BTreeSet::new();
            for line in complaint.lines().filter(|l| *l != "fails to validate") {
                let reference = line
                    .split_once("No match found for key-sequence ['")
                    .and_then(|(_, rest)| rest.split_once('\''))
                    .map(|(id, _)| id)
                    .unwrap_or_else(|| panic!("not a key reference: {line}"));
                references.insert(reference);
            }
            assert!(!references.is_empty(), "{complaint}");
            for reference in references {
                assert!(
                    shared.contains(&format!("id=\"{reference}\"")),
                    "{reference} is not defined in the shared document"
                );
            }
        }
    }
}

#[test]
fn the_epip_line_offer_indexes_its_stops_its_line_and_its_journeys() {
    let index = index("epip");
    assert_eq!(index.stop_count(), 4, "three stop places, one quay");
    let quay = index.stop(index.stop_by_dhid("ex:00001:1:1").expect("the quay"));
    assert_eq!((quay.name, quay.kind), ("Bahnhof, Steig A", StopKind::Quay));
    assert_eq!(
        quay.parent.map(|parent| index.stop(parent).id),
        Some("ex:StopPlace:bahnhof")
    );
    let bahnhof = index
        .stop_point_by_id("ex:ScheduledStopPoint:bahnhof")
        .expect("the stop point");
    assert_eq!(
        index
            .stop_of_stop_point(bahnhof)
            .map(|stop| index.stop(stop).id),
        Some("ex:Quay:bahnhof-a"),
        "a stop point is assigned to its quay where it names one"
    );
    let line = index.line(index.line_by_id("ex:Line:12").expect("the line"));
    assert_eq!((line.public_code, line.mode), (Some("12"), Some("bus")));

    assert_eq!(
        calls(&index, "ex:ServiceJourney:12-0700"),
        ["Bahnhof", "Markt", "Klinikum"]
    );
    assert_eq!(
        times(&index, "ex:ServiceJourney:12-0700"),
        [
            (None, Some(at(7, 0))),
            (Some(at(7, 6)), Some(at(7, 7))),
            (Some(at(7, 15)), None)
        ]
    );
    let first = index.journey(
        index
            .journey_by_id("ex:ServiceJourney:12-0700")
            .expect("the journey"),
    );
    assert_eq!(
        first.uses,
        [
            CallUse {
                boarding: true,
                alighting: false
            },
            CallUse {
                boarding: true,
                alighting: true
            },
            CallUse {
                boarding: false,
                alighting: true
            }
        ],
        "as the pattern states it"
    );
    assert_eq!(
        times(&index, "ex:ServiceJourney:12-2350")[2],
        (Some(86_400 + at(0, 5)), None),
        "an arrival on the next day is past midnight of the operating day"
    );
    for journey in ["ex:ServiceJourney:12-0700", "ex:ServiceJourney:12-2350"] {
        assert_eq!(october(&index, journey), [12, 13, 14, 15, 16], "{journey}");
    }
    assert!(index.unresolved().is_empty(), "{:?}", index.unresolved());
    let findings: Vec<String> = index.findings().map(|f| f.to_string()).collect();
    assert_eq!(findings, Vec::<String>::new());
}

/// The one document of the Italian line offer.
fn italian_line_offer() -> String {
    let mut documents = documents("it");
    assert_eq!(
        documents.len(),
        1,
        "one document, as the access point publishes it"
    );
    documents.remove(0).1
}

/// Every name of a stop place comes with the language it is written in: its
/// own name, and each alternative name with the language it names (`Lang`),
/// its kind and the language of its text — as written, none taken for another.
#[test]
fn every_name_of_an_italian_stop_place_comes_with_its_language() {
    let xml = italian_line_offer();
    let mut names = Vec::new();
    let options = ReadOptions::new(Selection::none().with(Kind::StopPlace));
    netex_rs::stream::read(&xml, &options, |entity, _| {
        let Entity::StopPlace(place) = entity else {
            panic!("only stop places are selected");
        };
        let id = place.id.as_ref().map_or("", |id| id.as_str()).to_owned();
        let name = place.name.as_ref().expect("a stop place's name");
        names.push(format!(
            "{id} [{}] {}",
            name.lang.as_deref().unwrap_or("-"),
            name.text
        ));
        for alternative in place
            .alternative_names
            .iter()
            .flat_map(|a| &a.alternative_names)
        {
            let text = alternative
                .name
                .as_ref()
                .expect("an alternative name's text");
            names.push(format!(
                "{id} {} {} [{}] {}",
                alternative.name_type.as_deref().unwrap_or("-"),
                alternative.lang.as_deref().unwrap_or("-"),
                text.lang.as_deref().unwrap_or("-"),
                text.text
            ));
        }
        Ok(())
    })
    .expect("the line offer reads");
    assert_eq!(
        names,
        [
            "IT:ITH1:StopPlace:it-29901-1: [it] Esempio, Piazza Centrale",
            "IT:ITH1:StopPlace:it-29901-1: translation de [de] Beispiel, Zentralplatz",
            "IT:ITH1:StopPlace:it-29901-1: translation lld [lld] Ejëmpl, Plaza Zentrala",
            "IT:ITH1:StopPlace:it-29901-2: [it] Esempio, Stazione",
            "IT:ITH1:StopPlace:it-29901-2: alias - [it] Stazione",
            "IT:ITH1:StopPlace:it-29902-1: [it] Altro Paese, Stazione",
        ]
    );
}

/// The typed model reads the whole line offer: nothing of it is carried
/// through untyped — neither the submodes the stop places state beside their
/// mode nor the train numbers of the timetable frame.
#[test]
fn the_italian_line_offer_is_read_with_nothing_left_untyped() {
    let document = netex_rs::read(&italian_line_offer()).expect("the line offer reads");
    let mut untyped = Vec::new();
    document.for_each_unprocessed(&mut |parent, fragment| {
        untyped.push(format!("{parent}/{}", fragment.name));
    });
    assert_eq!(untyped, Vec::<String>::new());
}

/// The train number of the timetable frame and the submodes the stop places
/// state beside their modes, as the lean stream reads them.
#[test]
fn the_italian_train_number_and_stop_place_submodes_read_as_written() {
    let options = ReadOptions::new(
        Selection::none()
            .with(Kind::StopPlace)
            .with(Kind::TrainNumber),
    );
    let mut submodes = Vec::new();
    let mut numbers = Vec::new();
    netex_rs::stream::read(&italian_line_offer(), &options, |entity, scope| {
        match entity {
            Entity::StopPlace(place) => {
                let id = place.id.as_ref().map(|id| id.as_str().to_owned());
                submodes.push((id, place.submode));
            }
            Entity::TrainNumber(number) => {
                let frame = scope.frames().last().expect("the frame it stands in");
                numbers.push((
                    number.id.as_ref().map(|id| id.as_str().to_owned()),
                    number.for_advertisement,
                    frame.kind,
                ));
            }
            other => panic!("not selected: {other:?}"),
        }
        Ok(())
    })
    .expect("the line offer reads");
    let id = |id: &str| Some(id.to_owned());
    assert_eq!(
        submodes,
        [
            (
                id("IT:ITH1:StopPlace:it-29901-1:"),
                Some(Submode::Bus("unknown".into()))
            ),
            (
                id("IT:ITH1:StopPlace:it-29901-2:"),
                Some(Submode::Rail("regionalRail".into()))
            ),
            (id("IT:ITH1:StopPlace:it-29902-1:"), None),
        ]
    );
    assert_eq!(
        numbers,
        [(
            id("IT:ITH1:TrainNumber:29001:"),
            Some("29001".into()),
            FrameKind::Timetable
        )]
    );
}

/// The bus pattern names its line through a route view, the train's pattern
/// only its route, which names the line: both journeys run in their lines,
/// on the weekdays of the calendar.
#[test]
fn the_italian_train_runs_in_the_line_its_route_names() {
    let index = index("it");
    assert_eq!(index.stop_count(), 5, "three stop places, two quays");
    assert_eq!(index.line_count(), 2);
    let line_of = |journey: &str| {
        let journey = index.journey_by_id(journey).expect("the journey");
        index.journey(journey).line.map(|line| index.line(line).id)
    };
    assert_eq!(
        line_of("IT:ITH1:ServiceJourney:201-0800:"),
        Some("IT:ITH1:Line:201:")
    );
    assert_eq!(
        line_of("IT:ITH1:ServiceJourney:R1-0815:"),
        Some("IT:ITH1:Line:R1:"),
        "the line of the route the pattern names"
    );
    let r1 = index.line_by_id("IT:ITH1:Line:R1:").expect("the line");
    assert_eq!(
        index.journeys_of_line(r1).len(),
        1,
        "the train is among its line's journeys"
    );
    assert_eq!(
        calls(&index, "IT:ITH1:ServiceJourney:R1-0815:"),
        ["Esempio, Stazione", "Altro Paese, Stazione"]
    );
    for journey in [
        "IT:ITH1:ServiceJourney:201-0800:",
        "IT:ITH1:ServiceJourney:R1-0815:",
    ] {
        assert_eq!(october(&index, journey), [12, 13, 14, 15, 16], "{journey}");
    }
    assert!(index.unresolved().is_empty(), "{:?}", index.unresolved());
    // The profile names its UIC operating periods with the `OperatingPeriodRef`
    // of a plain one; it resolves, and is a finding.
    let findings: Vec<String> = index.findings().map(|f| f.to_string()).collect();
    assert_eq!(
        findings,
        ["reference resolved to another class than it names: OperatingPeriodRef IT:ITH1:UicOperatingPeriod:feriali: in it-ith1-line-offer.xml (UicOperatingPeriod)"]
    );
}

/// Both Austrian documents define `at:ex:DayType:1` with its period and its
/// assignment — every day in line 100's document, the weekend in line 200's:
/// each journey runs on its own document's calendar, and each identifier
/// defined otherwise in the other document is a finding.
#[test]
fn each_austrian_document_reads_its_own_calendar() {
    for (name, xml) in documents("at") {
        for id in [
            "at:ex:DayType:1",
            "at:ex:UicOperatingPeriod:1",
            "at:ex:DayTypeAssignment:1",
        ] {
            assert!(xml.contains(&format!("id=\"{id}\"")), "{name} defines {id}");
        }
    }
    let index = index("at");
    assert_eq!(
        october(&index, "at:ex:ServiceJourney:200-1"),
        [17, 18],
        "the weekend of line 200's calendar, not every day of line 100's"
    );
    let journey = index.journey(
        index
            .journey_by_id("at:ex:ServiceJourney:200-1")
            .expect("the journey"),
    );
    assert_eq!(journey.document, "NX-PI_01_at_ex_LINE_ex_200_20261001.xml");
    assert_eq!(
        findings(&index, Concern::RedefinedOtherwise),
        [
            "redefined with other content: DayType at:ex:DayType:1 in NX-PI_01_at_ex_LINE_ex_200_20261001.xml, kept as NX-PI_01_at_ex_LINE_ex_100_20261001.xml defines it",
            "redefined with other content: UicOperatingPeriod at:ex:UicOperatingPeriod:1 in NX-PI_01_at_ex_LINE_ex_200_20261001.xml, kept as NX-PI_01_at_ex_LINE_ex_100_20261001.xml defines it"
        ],
        "the assignment is the same in both"
    );
}

/// Version 2 of line 100 is valid on 15 and 16 October and takes those days
/// from version 1, which is valid from 12 October on. Both journeys run on
/// the same calendar, every day of the week: the journey of version 1 runs on
/// the days but those two, the journey of version 2 on those two only.
#[test]
fn an_austrian_journey_runs_on_the_days_the_version_of_its_line_governs() {
    let index = index("at");
    assert_eq!(
        october(&index, "at:ex:ServiceJourney:100-v1"),
        [12, 13, 14, 17, 18]
    );
    assert_eq!(october(&index, "at:ex:ServiceJourney:100-v2"), [15, 16]);
    assert_eq!(
        findings(&index, Concern::DaysOutsideLineVersion),
        [
            "days its line version does not govern, taken away: ServiceJourney at:ex:ServiceJourney:100-v1 in NX-PI_01_at_ex_LINE_ex_100_20261001.xml (at:ex:Line:100)",
            "days its line version does not govern, taken away: ServiceJourney at:ex:ServiceJourney:100-v2 in NX-PI_01_at_ex_LINE_ex_100_20261001.xml (at:ex:Line:100)"
        ]
    );
    // The Austrian deliveries name a UIC operating period with the
    // `OperatingPeriodRef` of a plain one; it resolves, and is counted.
    assert_eq!(index.finding_count(Concern::ResolvedAsOtherClass), 2);
    // Each version is a line of the index; the identifier names the latest.
    let line_of = |journey: &str| {
        let journey = index.journey_by_id(journey).expect("the journey");
        index.journey(journey).line.expect("a line")
    };
    let (first, second) = (
        line_of("at:ex:ServiceJourney:100-v1"),
        line_of("at:ex:ServiceJourney:100-v2"),
    );
    assert_ne!(first, second);
    assert_eq!(
        [index.line(first).name, index.line(second).name],
        ["Hauptplatz – Bahnhof", "Hauptplatz – Bahnhof (Umleitung)"]
    );
    assert_eq!(index.line_by_id("at:ex:Line:100"), Some(second));
}

/// Line 200's document repeats the stop place Bahnhof with another quay; the
/// stop place is the delivery's, gathered once with the quays of both.
#[test]
fn an_austrian_stop_place_stated_in_two_documents_is_one_with_the_quays_of_both() {
    let index = index("at");
    let bahnhof = index.stop_by_dhid("at:99:2").expect("the stop place");
    let mut quays: Vec<&str> = index
        .stops()
        .filter(|&stop| index.stop(stop).parent == Some(bahnhof))
        .map(|stop| index.stop(stop).id)
        .collect();
    quays.sort_unstable();
    assert_eq!(quays, ["at:ex:Quay:2:1", "at:ex:Quay:2:2"]);
    assert_eq!(index.stop_count(), 3 + 4, "three stop places, four quays");
    assert!(index.unresolved().is_empty(), "{:?}", index.unresolved());
}

/// The train states its calls, the calendar is availability conditions, and
/// both stand in the shared document: the references resolve across the two.
#[test]
fn the_swiss_train_calls_where_its_calls_say_but_where_it_passes_through() {
    let index = index("ch");
    let train = "ch:ex:ServiceJourney:IR9-2148";
    assert_eq!(calls(&index, train), ["Seestadt", "Bergdorf", "Grenzheim"]);
    assert_eq!(
        times(&index, train),
        [
            (None, Some(at(21, 48))),
            (Some(at(22, 16)), Some(at(22, 18))),
            (Some(86_400 + at(0, 30)), None)
        ]
    );
    assert_eq!(
        index
            .journey(index.journey_by_id(train).expect("the train"))
            .pattern,
        None,
        "it names no pattern"
    );
    assert_eq!(
        findings(&index, Concern::PassedThrough),
        ["point passed through, no call: Call ch:ex:Call:IR9-2148-3 in ch-timetable.xml (ch:ex:ServiceJourney:IR9-2148)"]
    );
    assert_eq!(october(&index, train), [12, 13, 14, 15, 16, 17, 18]);
    assert!(index.unresolved().is_empty(), "{:?}", index.unresolved());
}

#[test]
fn the_swiss_shuttle_runs_every_twenty_minutes_on_weekdays() {
    let index = index("ch");
    let shuttle = "ch:ex:ServiceJourney:S1";
    let journey = index.journey(index.journey_by_id(shuttle).expect("the shuttle"));
    assert_eq!(
        journey.headways,
        [Headway {
            first_departure: at(6, 0),
            last_departure: at(7, 0),
            interval: 20 * 60,
        }]
    );
    let departures: Vec<u32> = journey.headways[0].departures().collect();
    assert_eq!(departures, [at(6, 0), at(6, 20), at(6, 40), at(7, 0)]);
    assert_eq!(calls(&index, shuttle), ["Bergdorf", "Talwil"]);
    assert_eq!(october(&index, shuttle), [12, 13, 14, 15, 16]);
}

/// The French stops: three stop places, each naming its quays by reference,
/// and four quays defined beside them, each naming its stop place. Every quay
/// is a quay of its stop place, the stop points are assigned to quays, and of
/// the two walks the one stating how long it takes is a transfer.
#[test]
fn a_french_quay_defined_beside_its_stop_place_is_one_of_its_quays() {
    let index = index("fr");
    assert_eq!(index.stop_count(), 7, "three stop places, four quays");
    let stop = |id: &str| index.stop(index.stop_by_id(id).expect("the stop"));
    let mut quays: Vec<(&str, &str)> = index
        .stops()
        .map(|stop| index.stop(stop))
        .filter(|stop| stop.kind == StopKind::Quay)
        .map(|quay| (quay.id, quay.parent.map_or("-", |p| index.stop(p).id)))
        .collect();
    quays.sort_unstable();
    assert_eq!(
        quays,
        [
            ("FR:00001:ZE:gare-a:LOC", "FR:00001:LMO:gare:LOC"),
            ("FR:00001:ZE:gare-r:LOC", "FR:00001:LMO:gare:LOC"),
            ("FR:00001:ZE:mairie:LOC", "FR:00001:LMO:mairie:LOC"),
            ("FR:00002:ZE:eglise:LOC", "FR:00002:LMO:eglise:LOC"),
        ]
    );
    let quay = stop("FR:00001:ZE:gare-a:LOC");
    assert_eq!(
        (quay.name, quay.latitude, quay.longitude),
        ("Gare", Some(48.8501), Some(2.3502))
    );
    let gare = index
        .stop_point_by_id("EX:ScheduledStopPoint:gare-a:LOC")
        .expect("the stop point");
    assert_eq!(
        index.stop_of_stop_point(gare).map(|s| index.stop(s).id),
        Some("FR:00001:ZE:gare-a:LOC"),
        "the quay the assignment names, in another document"
    );
    let gare_a = index
        .stop_by_id("FR:00001:ZE:gare-a:LOC")
        .expect("the quay");
    let gare_r = index
        .stop_by_id("FR:00001:ZE:gare-r:LOC")
        .expect("the quay");
    assert_eq!(
        index.transfers_from(gare_a),
        [Transfer {
            from: gare_a,
            to: gare_r,
            seconds: 120,
            both_ways: true,
        }],
        "the walk of the common frame between the two quays"
    );
    assert!(index.unresolved().is_empty(), "{:?}", index.unresolved());
    let findings: Vec<String> = index.findings().map(|f| f.to_string()).collect();
    assert_eq!(
        findings,
        ["walk stating no duration, no transfer: SiteConnection EX:SiteConnection:mairie-eglise:LOC in resource.xml"],
        "the walk stating its distance alone"
    );
}

/// The lean stream yields the quays defined on their own as entities of
/// their own, standing in the general frame.
#[test]
fn the_lean_stream_yields_the_french_quays_in_their_general_frame() {
    let (_, stops) = documents("fr")
        .into_iter()
        .find(|(name, _)| name == "stop.xml")
        .expect("the stops");
    let options = ReadOptions::new(Selection::none().with(Kind::Quay));
    let mut quays = Vec::new();
    netex_rs::stream::read(&stops, &options, |entity, scope| {
        let Entity::Quay(quay) = entity else {
            panic!("only quays are selected");
        };
        let frame = scope.frames().last().expect("the frame it stands in");
        let site = quay.site_ref.as_ref().and_then(|r| r.r#ref.as_ref());
        quays.push(format!(
            "{} of {} in {:?}",
            quay.id.as_ref().map_or("-", |id| id.as_str()),
            site.map_or("-", |id| id.as_str()),
            frame.kind
        ));
        Ok(())
    })
    .expect("the stops read");
    assert_eq!(
        quays,
        [
            "FR:00001:ZE:gare-a:LOC of FR:00001:LMO:gare:LOC in General",
            "FR:00001:ZE:gare-r:LOC of FR:00001:LMO:gare:LOC in General",
            "FR:00001:ZE:mairie:LOC of FR:00001:LMO:mairie:LOC in General",
            "FR:00002:ZE:eglise:LOC of FR:00002:LMO:eglise:LOC in General",
        ]
    );
}

/// The passing times name no point of the pattern: each is at the point of
/// its position, the last one of the night journey on the next day.
#[test]
fn a_french_passing_time_is_at_the_point_of_its_position() {
    let index = index("fr");
    let line = index.line(index.line_by_id("EX:Line:10:LOC").expect("the line"));
    assert_eq!((line.public_code, line.mode), (Some("10"), Some("bus")));
    for journey in [
        "EX:ServiceJourney:10-0730:LOC",
        "EX:ServiceJourney:10-0930:LOC",
        "EX:ServiceJourney:10-2345:LOC",
    ] {
        assert_eq!(calls(&index, journey), ["Gare", "Mairie", "Église"]);
    }
    assert_eq!(
        times(&index, "EX:ServiceJourney:10-0730:LOC"),
        [
            (None, Some(at(7, 30))),
            (Some(at(7, 41)), Some(at(7, 42))),
            (Some(at(7, 55)), None)
        ]
    );
    assert_eq!(
        times(&index, "EX:ServiceJourney:10-2345:LOC"),
        [
            (None, Some(at(23, 45))),
            (Some(at(23, 56)), Some(at(23, 57))),
            (Some(86_400 + at(0, 10)), None)
        ]
    );
    let journey = index.journey(
        index
            .journey_by_id("EX:ServiceJourney:10-0730:LOC")
            .expect("the journey"),
    );
    assert_eq!(
        journey.uses,
        [
            CallUse {
                boarding: true,
                alighting: false
            },
            CallUse {
                boarding: true,
                alighting: true
            },
            CallUse {
                boarding: false,
                alighting: true
            }
        ],
        "as the pattern states it"
    );
}

/// The three forms of a French day type. School days state a validity and
/// the days of the week, and assignments listing each day: those days alone,
/// the holiday Friday and the last three days of the validity among none of
/// them. The weekend states a validity and its days, and assignments as
/// exceptions: every Saturday and Sunday of its validity but the Sunday taken
/// away, and the Friday added. Wednesdays are assigned a period stating no
/// day bits: its Wednesdays.
#[test]
fn a_french_day_type_takes_its_days_from_its_dates_or_from_its_days_of_the_week() {
    let index = index("fr");
    assert_eq!(
        october(&index, "EX:ServiceJourney:10-0730:LOC"),
        [12, 13, 14, 15, 19, 20]
    );
    assert_eq!(
        october(&index, "EX:ServiceJourney:10-0930:LOC"),
        [10, 11, 16, 17, 24, 25]
    );
    assert_eq!(october(&index, "EX:ServiceJourney:10-2345:LOC"), [14, 21]);
}

/// What the model does not type in the French delivery — the stop places'
/// kinds of place and the network — is carried through as written.
#[test]
fn the_french_delivery_carries_through_only_its_kinds_of_place_and_its_network() {
    let mut untyped = Vec::new();
    for (name, xml) in documents("fr") {
        let document = netex_rs::read(&xml).expect("the document reads");
        document.for_each_unprocessed(&mut |parent, fragment| {
            untyped.push(format!("{name}: {parent}/{}", fragment.name));
        });
    }
    assert_eq!(
        untyped,
        [
            "network.xml: members/Network",
            "stop.xml: StopPlace/placeTypes",
            "stop.xml: StopPlace/placeTypes",
            "stop.xml: StopPlace/placeTypes",
        ]
    );
}

#[test]
fn the_own_examples_are_licensed_as_the_crate_is() {
    let directory = support::fixtures_dir().join("own");
    let readme =
        std::fs::read_to_string(directory.join("README.md")).expect("the examples' README");
    assert!(readme.contains("MIT OR Apache-2.0"), "{readme}");
    for fixture in support::own_fixtures("") {
        assert!(
            fixture.xml.contains("(MIT OR Apache-2.0)"),
            "{} names its licence",
            fixture.name
        );
    }
}
