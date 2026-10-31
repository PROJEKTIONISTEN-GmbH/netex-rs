//! The lean mode: a document read entity by entity, typing only the kinds the
//! caller selects and skipping the rest at the tokenizer, and the index built
//! from that stream — the same index the full model builds.

mod support;

use std::collections::BTreeMap;

use netex_rs::dataset::Source;
use netex_rs::index::{Index, IndexBuilder};
use netex_rs::stream::{self, Entity, FrameKind, Kind, ReadOptions, Selection};

fn web_fixture() -> support::Fixture {
    support::delfi_fixtures()
        .into_iter()
        .find(|f| f.name.contains("WEB-WEB_HB-511699"))
        .expect("the WEB fixture is there")
}

#[test]
fn the_routing_profile_yields_what_the_importer_needs_and_nothing_else() {
    let fixture = web_fixture();
    let options = ReadOptions::new(Selection::routing());
    let mut counts: BTreeMap<Kind, usize> = BTreeMap::new();
    let mut passing_times = 0;
    let mut quays = 0;
    let mut notice_assignments = 0;
    let mut inline_notices = 0;
    let mut count_notices = |assignments: &Option<netex_rs::model::NoticeAssignments>| {
        for assignment in assignments.iter().flat_map(|a| &a.notice_assignments) {
            notice_assignments += 1;
            if let Some(netex_rs::model::NoticeOrRef::Notice(_)) = assignment.notice {
                inline_notices += 1;
            }
        }
    };
    stream::read(&fixture.xml, &options, |entity, scope| {
        *counts.entry(entity.kind()).or_default() += 1;
        match &entity {
            Entity::ServiceJourneyPattern(pattern) => {
                for point in pattern.points_in_sequence.iter().flat_map(|p| &p.items) {
                    if let netex_rs::model::PointInSequenceItem::StopPointInJourneyPattern(point) =
                        point
                    {
                        count_notices(&point.notice_assignments);
                    }
                }
            }
            Entity::ServiceJourney(journey) => {
                count_notices(&journey.notice_assignments);
                passing_times += journey
                    .passing_times
                    .as_ref()
                    .map_or(0, |p| p.passing_times.len());
                assert_eq!(
                    scope.frames().last().map(|f| f.kind),
                    Some(FrameKind::Timetable)
                );
            }
            Entity::StopPlace(place) => {
                quays += place.quays.as_ref().map_or(0, |q| q.items.len());
                let frames = scope.frames();
                assert_eq!(frames.len(), 2, "composite frame, then the site frame");
                assert_eq!(frames[0].kind, FrameKind::Composite);
                assert_eq!(
                    frames[0].id.as_ref().map(|id| id.as_str()),
                    Some("DE::CompositeFrame_EU_PI_LINE_OFFER:WEB-WEB_HB-511699")
                );
                assert_eq!(frames[0].valid_between.len(), 1);
                assert_eq!(
                    frames[0].valid_between[0]
                        .from_date
                        .as_ref()
                        .map(|d| d.as_str()),
                    Some("2026-09-05T00:00:00")
                );
                assert_eq!(frames[1].kind, FrameKind::Site);
                assert!(frames[1].valid_between.is_empty());
            }
            Entity::DayType(_) => {
                assert_eq!(
                    scope.frames().last().map(|f| f.kind),
                    Some(FrameKind::ServiceCalendar)
                );
            }
            _ => {}
        }
        Ok(())
    })
    .expect("the fixture streams");
    let expected: BTreeMap<Kind, usize> = [
        (Kind::StopPlace, 2),
        (Kind::Line, 1),
        (Kind::ScheduledStopPoint, 3),
        (Kind::PassengerStopAssignment, 3),
        (Kind::ServiceJourneyPattern, 2),
        (Kind::SiteConnection, 1),
        (Kind::DayType, 1),
        (Kind::UicOperatingPeriod, 1),
        (Kind::DayTypeAssignment, 1),
        (Kind::ServiceJourney, 2),
    ]
    .into_iter()
    .collect();
    assert_eq!(counts, expected);
    assert_eq!(quays, 3, "the quays come with their stop places");
    assert_eq!(
        passing_times, 4,
        "the passing times come with their journeys"
    );
}

#[test]
fn a_selection_outside_the_profile_reaches_every_typed_entity_kind() {
    let fixture = web_fixture();
    let options = ReadOptions::new(
        Selection::none()
            .with(Kind::Direction)
            .with(Kind::Authority),
    );
    let mut seen = Vec::new();
    stream::read(&fixture.xml, &options, |entity, _| {
        seen.push(entity.kind());
        Ok(())
    })
    .expect("the fixture streams");
    assert_eq!(
        seen,
        vec![
            Kind::Authority,
            Kind::Authority,
            Kind::Authority,
            Kind::Direction,
            Kind::Direction
        ],
        "resource frame before service frame, in document order"
    );
    assert!(Selection::all().contains(Kind::Operator));
    assert!(!Selection::routing().contains(Kind::Operator));
    assert!(!Selection::none().contains(Kind::StopPlace));
}

#[test]
fn selecting_nothing_reads_the_document_without_yielding_and_still_checks_it() {
    let fixture = web_fixture();
    let options = ReadOptions::new(Selection::none());
    let mut yielded = 0;
    stream::read(&fixture.xml, &options, |_, _| {
        yielded += 1;
        Ok(())
    })
    .expect("the fixture streams");
    assert_eq!(yielded, 0);

    let error = stream::read("<Foo/>", &options, |_, _| Ok(())).expect_err("not NeTEx");
    assert!(
        matches!(error.kind, netex_rs::Kind::NotNetex { .. }),
        "{error}"
    );
    let truncated = &fixture.xml[..fixture.xml.len() / 2];
    assert!(
        stream::read(truncated, &options, |_, _| Ok(())).is_err(),
        "a truncated document is refused even when nothing is selected"
    );
}

#[test]
fn the_sink_can_stop_the_read() {
    let fixture = web_fixture();
    let options = ReadOptions::new(Selection::routing());
    let mut seen = 0;
    let error = stream::read(&fixture.xml, &options, |entity, _| {
        seen += 1;
        if entity.kind() == Kind::Line {
            return Err(netex_rs::Error::new(netex_rs::Kind::Io(
                "enough".to_owned(),
            )));
        }
        Ok(())
    })
    .expect_err("the sink's error comes back");
    assert!(matches!(error.kind, netex_rs::Kind::Io(ref what) if what == "enough"));
    assert!(
        seen < 20,
        "the read stopped at the line, {seen} entities in"
    );
}

#[test]
fn the_index_built_from_the_stream_is_the_index_the_full_model_builds() {
    let mut fixtures = support::delfi_fixtures();
    fixtures.sort_by_key(|f| std::cmp::Reverse(f.xml.len()));
    let options = ReadOptions::new(Selection::routing());
    let mut full = IndexBuilder::new();
    let mut lean = IndexBuilder::new();
    for fixture in &fixtures {
        full.add(
            &netex_rs::read(&fixture.xml).expect("the fixture reads"),
            &fixture.name,
        );
        lean.add_streamed(&fixture.xml, &options, &fixture.name)
            .expect("the fixture streams");
    }
    let full = full.finish();
    let lean = lean.finish();
    assert_eq!(lean.document_count(), 5);
    assert!(
        lean == full,
        "the lean index differs from the full one: {} vs {} stops, {} vs {} journeys, {} vs {} passing times, {} vs {} strings, {} vs {} unresolved",
        lean.stop_count(),
        full.stop_count(),
        lean.journey_count(),
        full.journey_count(),
        lean.passing_time_count(),
        full.passing_time_count(),
        lean.string_count(),
        full.string_count(),
        lean.unresolved().len(),
        full.unresolved().len()
    );

    // The dataset-level build, on one thread, reads largest first — the same
    // order — and merges nothing but one worker's builder.
    let dir = support::scratch_dir("stream-index-build");
    let zip = support::zip_of_delfi_fixtures(&dir);
    let source = Source::open(&zip).expect("the archive opens");
    let built = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .build()
        .expect("a pool of one thread")
        .install(|| Index::build(&source, &options).expect("the archive indexes"));
    assert!(built == full, "the dataset-level lean build differs");
}

/// What a sink of [`stream::read_with`] was handed: every entity's kind, every skipped element's
/// name with where it stood, in document order.
#[derive(Default)]
struct Recorded {
    seen: Vec<String>,
}

impl<'i> stream::Sink<'i> for Recorded {
    fn entity(&mut self, entity: Entity, _: &stream::Scope) -> netex_rs::Result<()> {
        self.seen.push(format!("entity {:?}", entity.kind()));
        Ok(())
    }

    fn skipped(
        &mut self,
        cursor: &mut netex_rs::xml::Cursor<'i>,
        element: &netex_rs::xml::Child<'i>,
        place: stream::Skipped<'i>,
        scope: &stream::Scope,
    ) -> netex_rs::Result<()> {
        let frame = scope.frames().last().map(|frame| frame.kind);
        self.seen
            .push(format!("{} at {place:?} in {frame:?}", element.local));
        cursor.skip_over(element)
    }
}

#[test]
fn every_element_the_walk_skips_is_handed_to_the_sink_with_where_it_stands() {
    let xml = r#"<PublicationDelivery xmlns="http://www.netex.org.uk/netex" xmlns:o="urn:other" version="1.10">
      <PublicationTimestamp>2026-10-10T00:00:00</PublicationTimestamp>
      <dataObjects>
        <FareFrame id="f" version="1"/>
        <CompositeFrame id="c" version="1">
          <FrameDefaults><DefaultLocale><TimeZone>Europe/Berlin</TimeZone></DefaultLocale></FrameDefaults>
          <frames>
            <ServiceFrame id="s" version="1">
              <o:extension/>
              <lines><Line id="l:1" version="1"/><FlexibleLine id="f:1" version="1"/></lines>
              <journeyPatterns><ServiceJourneyPattern id="p:1" version="1"/></journeyPatterns>
            </ServiceFrame>
            <ServiceCalendarFrame id="k" version="1">
              <ServiceCalendar id="sc" version="1"><FromDate>2026-10-10</FromDate><dayTypes><DayType id="d:1" version="1"/></dayTypes></ServiceCalendar>
            </ServiceCalendarFrame>
          </frames>
        </CompositeFrame>
      </dataObjects>
    </PublicationDelivery>"#;
    let options = ReadOptions::new(Selection::none().with(Kind::Line).with(Kind::DayType));
    let mut recorded = Recorded::default();
    stream::read_with(xml, &options, &mut recorded).expect("the document streams");
    assert_eq!(
        recorded.seen,
        [
            "PublicationTimestamp at Delivery in None",
            "FareFrame at Frames in None",
            "FrameDefaults at Frame(Composite) in Some(Composite)",
            "extension at Frame(Service) in Some(Service)",
            "entity Line",
            "FlexibleLine at Item(\"lines\") in Some(Service)",
            "journeyPatterns at Frame(Service) in Some(Service)",
            "FromDate at Calendar in Some(ServiceCalendar)",
            "entity DayType",
        ]
    );
}

#[test]
fn a_sink_that_skips_what_it_is_handed_reads_as_the_plain_stream() {
    let fixture = web_fixture();
    let options = ReadOptions::new(Selection::routing());
    let mut plain = Vec::new();
    stream::read(&fixture.xml, &options, |entity, _| {
        plain.push(entity);
        Ok(())
    })
    .expect("the fixture streams");

    /// Keeps the entities, skipping what it is handed as the default does.
    struct Entities(Vec<Entity>);
    impl stream::Sink<'_> for Entities {
        fn entity(&mut self, entity: Entity, _: &stream::Scope) -> netex_rs::Result<()> {
            self.0.push(entity);
            Ok(())
        }
    }
    let mut sink = Entities(Vec::new());
    stream::read_with(&fixture.xml, &options, &mut sink).expect("the fixture streams");
    assert!(!plain.is_empty());
    assert_eq!(sink.0, plain);
}

#[test]
fn an_element_the_sink_reads_with_the_cursor_is_read_as_its_reader_reads_it() {
    use netex_rs::model::{Element, FrameDefaults};

    /// Reads the frame defaults it is handed with their reader.
    #[derive(Default)]
    struct Defaults(Vec<FrameDefaults>);
    impl<'i> stream::Sink<'i> for Defaults {
        fn entity(&mut self, _: Entity, _: &stream::Scope) -> netex_rs::Result<()> {
            Ok(())
        }
        fn skipped(
            &mut self,
            cursor: &mut netex_rs::xml::Cursor<'i>,
            element: &netex_rs::xml::Child<'i>,
            _: stream::Skipped<'i>,
            _: &stream::Scope,
        ) -> netex_rs::Result<()> {
            if element.local == "FrameDefaults" {
                self.0.push(FrameDefaults::read(cursor, element)?);
                Ok(())
            } else {
                cursor.skip_over(element)
            }
        }
    }
    let fixture = web_fixture();
    let mut defaults = Defaults::default();
    stream::read_with(
        &fixture.xml,
        &ReadOptions::new(Selection::routing()),
        &mut defaults,
    )
    .expect("the fixture streams");
    let document = netex_rs::read(&fixture.xml).expect("the fixture reads");
    let in_model: Vec<&FrameDefaults> = document
        .delivery
        .frames()
        .filter_map(|frame| match frame {
            netex_rs::model::Frame::Composite(frame) => frame.frame_defaults.as_ref(),
            _ => None,
        })
        .collect();
    assert!(
        !in_model.is_empty(),
        "the fixture states its frame defaults"
    );
    assert_eq!(defaults.0.iter().collect::<Vec<_>>(), in_model);
}

#[test]
fn a_builder_hands_what_it_reads_on_to_a_sink_and_builds_the_index_it_builds_alone() {
    // One walk of each document: the index takes the routing entities, the
    // caller's sink gets them after it and everything the walk skips — here
    // the operators the index does not hold.
    let mut fixtures = support::delfi_fixtures();
    fixtures.sort_by(|a, b| a.name.cmp(&b.name));
    let options = ReadOptions::new(Selection::routing().with(Kind::Authority));
    let mut alone = IndexBuilder::new();
    let mut with_sink = IndexBuilder::new();
    let mut recorded = Recorded::default();
    for fixture in &fixtures {
        alone
            .add_streamed(&fixture.xml, &options, &fixture.name)
            .expect("the fixture streams");
        with_sink
            .add_streamed_with(&fixture.xml, &options, &fixture.name, &mut recorded)
            .expect("the fixture streams");
    }
    assert!(with_sink.finish() == alone.finish());
    let mut plain = Recorded::default();
    for fixture in &fixtures {
        stream::read_with(&fixture.xml, &options, &mut plain).expect("the fixture streams");
    }
    assert_eq!(
        recorded.seen, plain.seen,
        "the sink sees what the plain walk hands it"
    );
    assert!(recorded.seen.iter().any(|seen| seen == "entity Authority"));
    assert!(recorded
        .seen
        .iter()
        .any(|seen| seen.starts_with("FrameDefaults at ")));
}
