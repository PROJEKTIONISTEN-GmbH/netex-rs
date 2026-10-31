//! The crate's own fragments (`tests/fixtures/fragments/`): small sets of
//! frames, and entities standing alone, written for these tests from the
//! NeTEx schemas and XML Schema and licensed as the crate is. Each pins what
//! the reader makes of one kind of thing deliveries state — the lexical forms
//! of times and durations, white space, frames in an order of their own,
//! references, nested objects, reference systems, calendars, interchanges and
//! meetings, levels, axis order, train numbers, who is responsible for a line,
//! journeys that cannot be ridden, submodes beside a mode; the comment at the head of each says what
//! it holds and what in the standard it rests on.
//!
//! Every fragment must read, write back element for element, and be written
//! for either release and read again; a set of frames must index the same
//! read in full and streamed with every kind selected; an entity standing alone must read into its
//! typed model and write back as it was. What the index makes of single
//! fragments is asserted here and beside the tests of its subject (calendars,
//! calls, the axis order, journeys stating calls and conditions in place).

mod support;

use std::collections::BTreeMap;

use netex_rs::index::{Concern, IndexBuilder, LineId, StopPointId};
use netex_rs::model::{
    DayType, DayTypeAssignment, EntranceItem, OperatingPeriod, QuayItem, ScheduledStopPoint,
    ServiceJourney, ServiceJourneyInterchange, Submode,
};
use netex_rs::stream::{self, Entity, Kind, ReadOptions, Selection};
use netex_rs::{Limits, Release};
use quick_xml::events::Event;
use quick_xml::Reader;
use support::render::{assert_same, render};
use support::timetable::serially;
use support::{compare, fragments, parse, validate_all, Candidate, Fixture, VALIDATOR_MISSING};

/// The fragments that are deliberately not valid against a release, each
/// with what the validator says about it.
const NOT_VALID: &[(&str, &[Release], &str)] = &[
    (
        "frames/day-offsets.xml",
        &[Release::V1, Release::V2],
        "'24:03:00' is not a valid value of the atomic type 'xs:time'",
    ),
    (
        "frames/nested-objects.xml",
        &[Release::V1, Release::V2],
        "Element '{http://www.netex.org.uk/netex}StopPointInJourneyPattern'",
    ),
    (
        "frames/site-structures.xml",
        &[Release::V1],
        "Element '{http://www.netex.org.uk/netex}siteStructures': This element is not expected.",
    ),
    (
        "frames/times.xml",
        &[Release::V1, Release::V2],
        "'25:42:00' is not a valid value of the atomic type 'xs:time'",
    ),
    (
        "frames/unreadable-values.xml",
        &[Release::V1, Release::V2],
        "'tbd' is not a valid value of the atomic type 'xs:time'",
    ),
];

/// The elements at the top of `xml`, each with its local name and its text,
/// comments left out.
fn top_level(xml: &str) -> Vec<(String, &str)> {
    let mut reader = Reader::from_str(xml);
    let mut elements = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    loop {
        let before = reader.buffer_position() as usize;
        match reader.read_event().expect("a fragment is well-formed") {
            Event::Start(_) => {
                if depth == 0 {
                    start = before;
                }
                depth += 1;
            }
            Event::Empty(element) if depth == 0 => {
                let end = reader.buffer_position() as usize;
                elements.push((element.local_name().as_ref().to_owned(), &xml[before..end]));
            }
            Event::End(element) => {
                depth -= 1;
                if depth == 0 {
                    let end = reader.buffer_position() as usize;
                    elements.push((element.local_name().as_ref().to_owned(), &xml[start..end]));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    elements
}

/// Reads `$xml` as a lone `$name` into `$type`, writes it, and checks that
/// the written form holds what the text held and reads back the same.
macro_rules! read_back {
    ($type:ty, $xml:expr, $name:expr) => {{
        let limits = Limits::default();
        netex_rs::read_element::<$type>($xml, $name, &limits)
            .map_err(|error| error.to_string())
            .and_then(|read| {
                let written = netex_rs::write_element(&read, $name);
                compare(&parse($xml), &parse(&written))?;
                let again = netex_rs::read_element::<$type>(&written, $name, &limits)
                    .map_err(|error| format!("written again: {error}"))?;
                if read == again {
                    Ok(())
                } else {
                    Err("read back other than written".to_owned())
                }
            })
    }};
}

/// Reads one entity standing alone into the typed model its name has.
fn read_entity(name: &str, xml: &str) -> Result<(), String> {
    match name {
        "DayType" => read_back!(DayType, xml, name),
        "DayTypeAssignment" => read_back!(DayTypeAssignment, xml, name),
        "UicOperatingPeriod" => read_back!(OperatingPeriod, xml, name),
        "ScheduledStopPoint" => read_back!(ScheduledStopPoint, xml, name),
        "ServiceJourney" => read_back!(ServiceJourney, xml, name),
        "ServiceJourneyInterchange" => read_back!(ServiceJourneyInterchange, xml, name),
        other => Err(format!("no typed reading for a lone `{other}`")),
    }
}

/// Reads a document, checks that it writes back element for element and that
/// the lean reader takes it, and writes it for both releases, which must
/// read again; returns what each release left out.
fn read_document(xml: &str) -> Result<Vec<String>, String> {
    let document = netex_rs::read(xml).map_err(|error| error.to_string())?;
    compare(&parse(xml), &parse(&document.write()))?;
    stream::read(xml, &ReadOptions::default(), |_, _| Ok(()))
        .map_err(|error| format!("lean: {error}"))?;
    let mut losses = Vec::new();
    for release in [Release::V1, Release::V2] {
        let written = document.write_for(release);
        netex_rs::read(&written.xml)
            .map_err(|error| format!("written for {release:?}: {error}"))?;
        losses.extend(written.report.losses().map(|loss| loss.to_string()));
    }
    Ok(losses)
}

#[test]
fn every_set_of_frames_reads_writes_back_and_indexes_alike_in_full_and_streamed() {
    let frames = fragments::frames();
    assert_eq!(frames.len(), 23);
    let mut failures = Vec::new();
    for fixture in &frames {
        let document = fragments::publication(&fixture.xml);
        match read_document(&document) {
            Ok(losses) if fixture.name == "site-structures.xml" => assert!(
                !losses.is_empty() && losses.iter().all(|loss| loss.contains("siteStructures")),
                "a site structure is left out for 1.x and nothing else: {losses:?}"
            ),
            Ok(losses) => assert!(losses.is_empty(), "{}: {losses:?}", fixture.name),
            Err(error) => failures.push(format!("{}: {error}", fixture.name)),
        }
        let mut streamed = IndexBuilder::new();
        streamed
            .add_streamed(&document, &ReadOptions::default(), &fixture.name)
            .unwrap_or_else(|error| panic!("{}, streamed: {error}", fixture.name));
        assert_same(
            &render(&serially(&[(fixture.name.clone(), document)])),
            &render(&streamed.finish()),
            &format!("{}, streamed", fixture.name),
        );
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn every_entity_standing_alone_reads_into_its_typed_model_and_writes_back_as_it_was() {
    let files = fragments::entities();
    assert_eq!(files.len(), 5);
    let mut failures = Vec::new();
    let mut entities = 0;
    for fixture in &files {
        for (element, text) in top_level(&fixture.xml) {
            entities += 1;
            if let Err(error) = read_entity(&element, text) {
                failures.push(format!("{}: {element}: {error}", fixture.name));
            }
        }
        if let Err(error) = read_document(&fragments::members(&fixture.xml)) {
            failures.push(format!("{} as a frame's members: {error}", fixture.name));
        }
    }
    assert_eq!(entities, 15);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The fragments of one directory joined into one document by `wrap`, each
/// starting on a line of its own, with the line each starts on: a schema
/// validator compiles the NeTEx schema in many seconds, so it is given one
/// document rather than one per fragment.
fn joined(directory: &str, fixtures: &[Fixture], wrap: fn(&str) -> String) -> Joined {
    let mut text = String::new();
    let mut starts = Vec::new();
    let mut line = 1;
    for fixture in fixtures {
        text.push('\n');
        line += 1;
        starts.push((line, format!("{directory}/{}", fixture.name)));
        text.push_str(&fixture.xml);
        line += fixture.xml.matches('\n').count();
    }
    let xml = wrap(&text);
    assert!(
        !xml[..xml.find(&text).expect("the fragments are inside")].contains('\n'),
        "the wrapping opens on one line, so that the lines count from the first"
    );
    Joined { xml, starts }
}

/// Fragments joined into one document.
struct Joined {
    xml: String,
    starts: Vec<(usize, String)>,
}

impl Joined {
    /// The fragment a line of the document belongs to.
    fn fragment_at(&self, line: usize) -> &str {
        let at = self.starts.partition_point(|(start, _)| *start <= line);
        assert!(at > 0, "line {line} is before the first fragment");
        &self.starts[at - 1].1
    }
}

/// Every fragment is valid against the schemas of both releases but for what
/// its comment says is not, and for references to objects other fragments
/// define or none does, which an XML Schema key reference cannot resolve.
#[test]
fn every_fragment_is_valid_for_both_releases_but_where_its_comment_says_otherwise() {
    let documents = [
        joined("frames", &fragments::frames(), fragments::publication),
        joined("entities", &fragments::entities(), fragments::members),
    ];
    let candidates: Vec<Candidate> = documents
        .iter()
        .enumerate()
        .map(|(number, document)| Candidate {
            name: number.to_string(),
            xml: document.xml.clone(),
        })
        .collect();
    let releases: Vec<Release> = [Release::V1, Release::V2]
        .into_iter()
        .filter(|&release| support::official_fixtures(release))
        .collect();
    if !releases.is_empty() {
        assert!(support::validator_available(), "{VALIDATOR_MISSING}");
    }
    // The two releases are validated side by side.
    let verdicts: Vec<(Release, BTreeMap<String, String>)> = std::thread::scope(|scope| {
        let validations: Vec<_> = releases
            .iter()
            .map(|&release| {
                let candidates = &candidates;
                scope.spawn(move || (release, validate_all(release, candidates)))
            })
            .collect();
        validations
            .into_iter()
            .map(|validation| validation.join().expect("a validation"))
            .collect()
    });
    for (release, verdicts) in &verdicts {
        let release = *release;
        let mut complaints: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for (number, verdict) in verdicts {
            let document = &documents[number.parse::<usize>().expect("a number")];
            for complaint in verdict.lines() {
                if complaint == "fails to validate"
                    || complaint.contains("No match found for key-sequence")
                {
                    continue;
                }
                let line = complaint
                    .split_once(':')
                    .and_then(|(line, _)| line.parse().ok())
                    .unwrap_or_else(|| panic!("a complaint names its line: {complaint}"));
                complaints
                    .entry(document.fragment_at(line))
                    .or_default()
                    .push(complaint);
            }
        }
        for (fragment, said) in &complaints {
            let (_, _, expected) = NOT_VALID
                .iter()
                .find(|(name, releases, _)| name == fragment && releases.contains(&release))
                .unwrap_or_else(|| panic!("{fragment} is not valid for {release:?}: {said:#?}"));
            assert!(
                said.iter().any(|complaint| complaint.contains(expected)),
                "{fragment}, {release:?}: {said:#?}"
            );
        }
        let expected: Vec<&str> = NOT_VALID
            .iter()
            .filter(|(_, releases, _)| releases.contains(&release))
            .map(|(fragment, _, _)| *fragment)
            .collect();
        assert_eq!(
            complaints.keys().copied().collect::<Vec<_>>(),
            expected,
            "{release:?}"
        );
    }
}

#[test]
fn every_fragment_names_its_licence_and_what_in_the_standard_it_rests_on() {
    for fixture in fragments::frames().into_iter().chain(fragments::entities()) {
        let comment = fixture
            .xml
            .strip_prefix("<!--")
            .and_then(|rest| rest.split_once("-->"))
            .map(|(comment, _)| comment.split_whitespace().collect::<Vec<_>>().join(" "))
            .unwrap_or_else(|| panic!("{} opens with a comment", fixture.name));
        assert!(
            comment.contains("Written for netex-rs's tests (MIT OR Apache-2.0)"),
            "{} names its licence",
            fixture.name
        );
        assert!(
            [".xsd", "XML Schema", "GML 3.2"]
                .iter()
                .any(|source| comment.contains(source)),
            "{} names the schema it rests on",
            fixture.name
        );
    }
}

/// A time after midnight stated as the standard states it, a time of day
/// with the days after the journey's first, and as hours counted past 24 —
/// `24:00:00` is the one such value `xs:time` allows — come to the same
/// seconds of the operating day.
#[test]
fn a_time_after_midnight_is_the_same_with_a_day_offset_and_as_hours_past_24() {
    let documents = [(
        "day-offsets.xml".to_owned(),
        fragments::document("day-offsets.xml"),
    )];
    let index = serially(&documents);
    let times = |id: &str| -> Vec<(Option<u32>, Option<u32>)> {
        let journey = index.journey_by_id(id).expect("the journey");
        index
            .journey(journey)
            .passing_times
            .iter()
            .map(|time| (time.arrival, time.departure))
            .collect()
    };
    let at = |hours: u32, minutes: u32| Some(hours * 3600 + minutes * 60);
    assert_eq!(
        times("ex:ServiceJourney:offset"),
        [
            (None, at(23, 35)),
            (at(23, 58), at(24, 3)),
            (at(24, 44), None)
        ]
    );
    assert_eq!(
        times("ex:ServiceJourney:counted"),
        times("ex:ServiceJourney:offset")
    );
    assert_eq!(
        index.journey_by_id("ex:ServiceJourney:day"),
        None,
        "a journey with a time at one point only cannot be ridden"
    );
}

/// The reader keeps a value as it was written: the white space around an
/// enumeration, which XML Schema would collapse, and the white space of a
/// name, which it keeps.
#[test]
fn white_space_around_an_enumeration_and_within_a_name_is_kept_as_written() {
    let documents = [(
        "whitespace.xml".to_owned(),
        fragments::document("whitespace.xml"),
    )];
    let index = serially(&documents);
    let mut modes: Vec<(&str, Option<&str>)> = (0..index.line_count() as u32)
        .map(|line| {
            let line = index.line(LineId(line));
            (line.name, line.mode)
        })
        .collect();
    modes.sort_unstable();
    assert_eq!(
        modes,
        [
            ("leading", Some("\tcoach")),
            ("plain", Some("funicular")),
            ("trailing", Some("tram ")),
            ("wrapped", Some("\n        ferry\n      ")),
        ]
    );
    let names: Vec<&str> = (0..index.stop_point_count() as u32)
        .map(|point| index.stop_point(StopPointId(point)).name)
        .collect();
    assert_eq!(names, ["  Lindener   Marktplatz "]);
}

/// A stop place, its entrance and its quay each read the submode they state
/// beside their mode; a stop place stating none reads none.
#[test]
fn a_submode_stated_beside_a_mode_is_read_where_it_stands() {
    let xml = fragments::document("submodes.xml");
    let options = ReadOptions::new(Selection::none().with(Kind::StopPlace));
    let mut read = Vec::new();
    stream::read(&xml, &options, |entity, _| {
        let Entity::StopPlace(place) = entity else {
            panic!("only stop places are selected");
        };
        let id =
            |id: &Option<netex_rs::model::Id>| id.as_ref().map_or("", |id| id.as_str()).to_owned();
        read.push((id(&place.id), place.submode.clone()));
        for entrance in place.entrances.iter().flat_map(|e| &e.items) {
            if let EntranceItem::StopPlaceEntrance(entrance) = entrance {
                read.push((id(&entrance.id), entrance.submode.clone()));
            }
        }
        for quay in place.quays.iter().flat_map(|q| &q.items) {
            if let QuayItem::Quay(quay) = quay {
                read.push((id(&quay.id), quay.submode.clone()));
            }
        }
        Ok(())
    })
    .expect("the fragment reads");
    let ferry = || Some(Submode::Water("localPassengerFerry".into()));
    assert_eq!(
        read,
        [
            ("ex:StopPlace:Hafen".to_owned(), ferry()),
            ("ex:StopPlaceEntrance:Hafen:Kai".to_owned(), ferry()),
            ("ex:Quay:Hafen:1".to_owned(), ferry()),
            ("ex:StopPlace:Markt".to_owned(), None),
        ]
    );
}

/// Quays defined beside their stop place (`quays-beside-their-stop-place.xml`):
/// a quay belongs to the stop place naming it among its quays, else to the one
/// its `SiteRef` names. A second claim — a second stop place naming it, or its
/// `SiteRef` naming another than the one that does — leaves it where it is and
/// is a finding; a `QuayRef` to nothing is a reference to nothing, and one
/// naming a stop place, like a `SiteRef` naming a quay, resolves to another
/// class than it names.
#[test]
fn a_quay_defined_beside_its_stop_place_belongs_to_the_one_naming_it() {
    let name = "quays-beside-their-stop-place.xml";
    let index = serially(&[(name.to_owned(), fragments::document(name))]);
    let place_of = |quay: &str| {
        let quay = index.stop(index.stop_by_id(quay).expect("the quay"));
        quay.parent.map(|place| index.stop(place).id)
    };
    for quay in ["ex:Quay:1", "ex:Quay:2", "ex:Quay:3", "ex:Quay:4"] {
        assert_eq!(place_of(quay), Some("ex:StopPlace:A"), "{quay}");
    }
    assert_eq!(place_of("ex:Quay:5"), None);
    assert_eq!(place_of("ex:Quay:6"), None);
    let findings = |concern: Concern| -> Vec<String> {
        index
            .findings()
            .filter(|finding| finding.concern == concern)
            .map(|finding| finding.to_string())
            .collect()
    };
    assert_eq!(
        findings(Concern::QuayOfTwoStopPlaces),
        [
            "quay two stop places claim, kept with the first: Quay ex:Quay:3 in quays-beside-their-stop-place.xml (ex:StopPlace:B)",
            "quay two stop places claim, kept with the first: Quay ex:Quay:4 in quays-beside-their-stop-place.xml (ex:StopPlace:B)"
        ]
    );
    assert_eq!(
        findings(Concern::ResolvedAsOtherClass),
        [
            "reference resolved to another class than it names: QuayRef ex:StopPlace:A in quays-beside-their-stop-place.xml (StopPlace)",
            "reference resolved to another class than it names: SiteRef ex:Quay:1 in quays-beside-their-stop-place.xml (Quay)"
        ]
    );
    let unresolved: Vec<String> = index.unresolved().iter().map(ToString::to_string).collect();
    assert_eq!(unresolved.len(), 1, "{unresolved:?}");
    assert_eq!(
        (
            index.unresolved()[0].element.as_str(),
            index.unresolved()[0].id.as_str(),
            index.unresolved()[0].referrer.as_str()
        ),
        ("QuayRef", "ex:Quay:gone", "StopPlace ex:StopPlace:A")
    );
}
