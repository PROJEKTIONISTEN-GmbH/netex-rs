//! The Swiss form of a timetable, typed and indexed: journeys stating their
//! calls rather than a pattern with passing times, their days as availability
//! conditions rather than day types, and template journeys running at a
//! headway; and the Austrian on-demand services, journeys of flexible lines.

mod support;

use netex_rs::index::{CallUse, Concern, Headway, Index};
use netex_rs::model::{
    Call, Date, Frame, StopPointRef, StopUse, ValidityConditionItem, VehicleJourney,
};
use netex_rs::stream::{self, Entity, Kind, ReadOptions, Selection};
use support::timetable::{merged, serially};

fn publication(frames: &str) -> String {
    format!(
        concat!(
            r#"<PublicationDelivery xmlns="http://www.netex.org.uk/netex" version="1.10">"#,
            "<PublicationTimestamp>2026-10-10T11:31:07</PublicationTimestamp>",
            "<ParticipantRef>MENTZ</ParticipantRef>",
            r#"<dataObjects><CompositeFrame id="ch:1:CompositeFrame:j26" version="any"><frames>{}</frames></CompositeFrame></dataObjects>"#,
            "</PublicationDelivery>"
        ),
        frames
    )
}

/// The shared documents: the stop points and the line, and the calendar —
/// availability conditions over the week of 12 October, a bit per day.
fn shared() -> String {
    publication(concat!(
        r#"<ServiceFrame id="ch:1:ServiceFrame:j26" version="any">"#,
        r#"<lines><Line id="ch:1:Line:IC1" version="any"><Name>IC 1</Name><TransportMode>rail</TransportMode></Line></lines>"#,
        "<scheduledStopPoints>",
        r#"<ScheduledStopPoint id="ch:1:ScheduledStopPoint:GE" version="any"><Name>Genève</Name></ScheduledStopPoint>"#,
        r#"<ScheduledStopPoint id="ch:1:ScheduledStopPoint:LS" version="any"><Name>Lausanne</Name></ScheduledStopPoint>"#,
        r#"<ScheduledStopPoint id="ch:1:ScheduledStopPoint:ZH" version="any"><Name>Zürich HB</Name></ScheduledStopPoint>"#,
        r#"<ScheduledStopPoint id="ch:1:ScheduledStopPoint:SG" version="any"><Name>St. Gallen</Name></ScheduledStopPoint>"#,
        "</scheduledStopPoints></ServiceFrame>",
        r#"<ServiceCalendarFrame id="ch:1:ServiceCalendarFrame:j26" version="any"><validityConditions>"#,
        r#"<AvailabilityCondition id="ch:1:AvailabilityCondition:week" version="any"><FromDate>2026-10-12T00:00:00</FromDate><ToDate>2026-10-18T23:59:59</ToDate><ValidDayBits>1111111</ValidDayBits></AvailabilityCondition>"#,
        r#"<AvailabilityCondition id="ch:1:AvailabilityCondition:mon" version="any"><FromDate>2026-10-12T00:00:00</FromDate><ToDate>2026-10-18T23:59:59</ToDate><ValidDayBits>1000000</ValidDayBits></AvailabilityCondition>"#,
        r#"<AvailabilityCondition id="ch:1:AvailabilityCondition:wed" version="any"><FromDate>2026-10-12T00:00:00</FromDate><ToDate>2026-10-18T23:59:59</ToDate><ValidDayBits>0010000</ValidDayBits></AvailabilityCondition>"#,
        "</validityConditions>",
        r#"<dayTypes><DayType id="ch:1:DayType:weekend" version="any"/></dayTypes>"#,
        r#"<operatingPeriods><UicOperatingPeriod id="ch:1:UicOperatingPeriod:weekend" version="any"><FromDate>2026-10-12T00:00:00</FromDate><ToDate>2026-10-18T00:00:00</ToDate><ValidDayBits>0000011</ValidDayBits></UicOperatingPeriod></operatingPeriods>"#,
        r#"<dayTypeAssignments><DayTypeAssignment id="ch:1:DayTypeAssignment:weekend" version="any" order="1"><OperatingPeriodRef ref="ch:1:UicOperatingPeriod:weekend" versionRef="any"/><DayTypeRef ref="ch:1:DayType:weekend" versionRef="any"/></DayTypeAssignment></dayTypeAssignments>"#,
        "</ServiceCalendarFrame>",
    ))
}

/// A call as the Swiss delivery writes it.
fn call(
    id: &str,
    order: u32,
    stop_point: &str,
    arrival: Option<&str>,
    departure: Option<&str>,
    rest: &str,
) -> String {
    let arrival = arrival.map_or(String::new(), |time| {
        format!("<Arrival>{time}<ForAlighting>true</ForAlighting><IsFlexible>false</IsFlexible></Arrival>")
    });
    let departure = departure.map_or(String::new(), |time| {
        format!("<Departure>{time}<ForBoarding>true</ForBoarding><IsFlexible>false</IsFlexible></Departure>")
    });
    format!(
        r#"<Call id="{id}" version="any" order="{order}"><ScheduledStopPointRef ref="ch:1:ScheduledStopPoint:{stop_point}" versionRef="any"/>{arrival}{departure}{rest}</Call>"#
    )
}

/// The train of the Swiss part's example: Genève 19:48, Lausanne, passing
/// Zürich HB, St. Gallen after midnight.
fn train(id: &str, conditions: &[&str], day_types: &[&str]) -> String {
    let conditions: String = conditions
        .iter()
        .map(|c| format!(r#"<AvailabilityConditionRef ref="ch:1:AvailabilityCondition:{c}" versionRef="any"/>"#))
        .collect();
    let day_types: String = day_types
        .iter()
        .map(|d| format!(r#"<DayTypeRef ref="ch:1:DayType:{d}" versionRef="any"/>"#))
        .collect();
    let day_types = if day_types.is_empty() {
        String::new()
    } else {
        format!("<dayTypes>{day_types}</dayTypes>")
    };
    format!(
        concat!(
            r#"<ServiceJourney id="{id}" version="any"><keyList><KeyValue><Key>SJYID</Key><Value>{id}</Value></KeyValue></keyList>"#,
            "<validityConditions>{}</validityConditions>",
            "<Extensions/><TransportMode>rail</TransportMode>{}",
            r#"<LineRef ref="ch:1:Line:IC1" versionRef="any"/><calls>{}{}{}{}</calls></ServiceJourney>"#
        ),
        conditions,
        day_types,
        call(
            "c1",
            1,
            "GE",
            None,
            Some("<Time>19:48:00</Time>"),
            "<StopUse>access</StopUse>"
        ),
        call(
            "c2",
            2,
            "LS",
            Some("<Time>20:16:00</Time>"),
            Some("<Time>20:18:00</Time>"),
            "<StopUse>access</StopUse>"
        ),
        call(
            "c3",
            3,
            "ZH",
            Some("<Time>23:24:00</Time>"),
            Some("<Time>23:58:00</Time>"),
            "<StopUse>passthrough</StopUse>"
        ),
        call(
            "c4",
            4,
            "SG",
            Some("<Time>00:30:00</Time><DayOffset>1</DayOffset>"),
            None,
            "<StopUse>access</StopUse>"
        ),
        id = id,
    )
}

fn timetable(journeys: &str) -> String {
    publication(&format!(
        r#"<TimetableFrame id="ch:1:TimetableFrame:j26" version="any"><vehicleJourneys>{journeys}</vehicleJourneys></TimetableFrame>"#
    ))
}

fn delivery(journeys: &str) -> Vec<(String, String)> {
    vec![
        (
            "PROD_NETEX_TT_1.10_CHE_SKI_2026_OEV-SCHWEIZ_SERVICE_1_1.xml".to_owned(),
            shared(),
        ),
        (
            "PROD_NETEX_TT_1.10_CHE_SKI_2026_OEV-SCHWEIZ_TIMETABLE_1_515.xml".to_owned(),
            timetable(journeys),
        ),
    ]
}

fn day(text: &str) -> i64 {
    Date::from(text).days_since_epoch().expect("a date")
}

/// The days of October the journey `id` runs on.
fn october(index: &Index, id: &str) -> Vec<i64> {
    let journey = index.journey_by_id(id).expect("the journey");
    index
        .days_of(journey)
        .map(|d| d - day("2026-09-30"))
        .collect()
}

fn findings(index: &Index, concern: Concern) -> Vec<String> {
    index
        .findings()
        .filter(|finding| finding.concern == concern)
        .map(|finding| finding.to_string())
        .collect()
}

#[test]
fn a_call_is_typed_in_the_model_and_in_the_stream() {
    let document = netex_rs::read(&timetable(&train("ch:1:ServiceJourney:1", &["week"], &[])))
        .expect("the document reads");
    let journey = document
        .delivery
        .frames()
        .find_map(|frame| match frame {
            Frame::Timetable(timetable) => timetable.vehicle_journeys.as_ref(),
            _ => None,
        })
        .and_then(|journeys| match journeys.items.first() {
            Some(VehicleJourney::ServiceJourney(journey)) => Some(journey.clone()),
            _ => None,
        })
        .expect("the journey");
    let calls: Vec<&Call> = journey
        .calls
        .as_ref()
        .expect("calls")
        .items
        .iter()
        .filter_map(|item| match item {
            netex_rs::model::CallItem::Call(call) => Some(&**call),
            _ => None,
        })
        .collect();
    assert_eq!(calls.len(), 4);
    assert!(matches!(
        &calls[0].scheduled_stop_point_ref,
        Some(StopPointRef::ScheduledStopPoint(r)) if r.r#ref.as_ref().map(|id| id.as_str()) == Some("ch:1:ScheduledStopPoint:GE")
    ));
    assert_eq!(calls[2].stop_use, Some(StopUse::Passthrough));
    let arrival = calls[3].arrival.as_ref().expect("an arrival");
    assert_eq!(arrival.day_offset, Some(1));
    assert_eq!(arrival.for_alighting, Some(true));
    let condition = journey
        .validity_conditions
        .as_ref()
        .and_then(|conditions| conditions.items.first());
    assert!(matches!(
        condition,
        Some(ValidityConditionItem::AvailabilityConditionRef(r)) if r.r#ref.as_ref().map(|id| id.as_str()) == Some("ch:1:AvailabilityCondition:week")
    ));
    // Written and read again, the same.
    let again = netex_rs::read(&document.write()).expect("written, it reads");
    assert_eq!(again, document);
    // The lean stream yields the conditions of the calendar frame.
    let mut conditions = Vec::new();
    let options = ReadOptions::new(Selection::none().with(Kind::AvailabilityCondition));
    stream::read(&shared(), &options, |entity, _| {
        if let Entity::AvailabilityCondition(condition) = entity {
            conditions.push(condition.valid_day_bits.clone().expect("bits"));
        }
        Ok(())
    })
    .expect("the shared document reads");
    assert_eq!(conditions.len(), 3);
    assert_eq!(&*conditions[1], "1000000");
}

#[test]
fn a_journey_stating_its_calls_calls_there_but_where_it_passes_through() {
    let documents = delivery(&train("ch:1:ServiceJourney:1", &["week"], &[]));
    for index in [serially(&documents), merged(&documents)] {
        let id = index
            .journey_by_id("ch:1:ServiceJourney:1")
            .expect("the train");
        let journey = index.journey(id);
        let stops: Vec<&str> = journey
            .stop_points
            .iter()
            .map(|&point| index.stop_point(point).name)
            .collect();
        assert_eq!(stops, ["Genève", "Lausanne", "St. Gallen"]);
        assert_eq!(journey.pattern, None, "it names no pattern");
        let times: Vec<(Option<u32>, Option<u32>)> = journey
            .passing_times
            .iter()
            .map(|time| (time.arrival, time.departure))
            .collect();
        assert_eq!(
            times,
            [
                (None, Some(19 * 3600 + 48 * 60)),
                (Some(20 * 3600 + 16 * 60), Some(20 * 3600 + 18 * 60)),
                (Some(86_400 + 30 * 60), None),
            ]
        );
        assert_eq!(
            journey.uses,
            [CallUse {
                boarding: true,
                alighting: true
            }; 3]
        );
        assert_eq!(
            findings(&index, Concern::PassedThrough),
            ["point passed through, no call: Call c3 in PROD_NETEX_TT_1.10_CHE_SKI_2026_OEV-SCHWEIZ_TIMETABLE_1_515.xml (ch:1:ServiceJourney:1)"]
        );
        assert_eq!(
            october(&index, "ch:1:ServiceJourney:1"),
            [12, 13, 14, 15, 16, 17, 18]
        );
        assert!(index.unresolved().is_empty(), "{:?}", index.unresolved());
    }
}

#[test]
fn a_journey_runs_on_the_days_of_its_conditions_within_its_day_types() {
    let journeys = [
        train("ch:1:ServiceJourney:mon-wed", &["mon", "wed"], &[]),
        train("ch:1:ServiceJourney:weekend", &["week"], &["weekend"]),
        train("ch:1:ServiceJourney:none", &["mon"], &["weekend"]),
    ]
    .concat();
    let index = serially(&delivery(&journeys));
    assert_eq!(
        october(&index, "ch:1:ServiceJourney:mon-wed"),
        [12, 14],
        "the days of every condition it names"
    );
    assert_eq!(
        october(&index, "ch:1:ServiceJourney:weekend"),
        [17, 18],
        "the days of its day types within its conditions"
    );
    assert_eq!(
        october(&index, "ch:1:ServiceJourney:none"),
        Vec::<i64>::new()
    );
}

#[test]
fn a_template_journey_runs_at_every_departure_of_its_headway_groups() {
    let template = concat!(
        r#"<TemplateServiceJourney id="ch:1:ServiceJourney:fsu10" version="any">"#,
        r#"<validityConditions><AvailabilityConditionRef ref="ch:1:AvailabilityCondition:week" versionRef="any"/></validityConditions>"#,
        r#"<LineRef ref="ch:1:Line:IC1" versionRef="any"/><calls>"#,
        r#"<Call id="t1" version="any" order="1"><ScheduledStopPointRef ref="ch:1:ScheduledStopPoint:ZH" versionRef="any"/><Departure><Time>09:30:00</Time></Departure></Call>"#,
        r#"<Call id="t2" version="any" order="2"><ScheduledStopPointRef ref="ch:1:ScheduledStopPoint:SG" versionRef="any"/><Arrival><Time>09:40:00</Time></Arrival></Call>"#,
        "</calls><frequencyGroups>",
        r#"<HeadwayJourneyGroup id="ch:1:HeadwayJourneyGroup:1" version="any"><FirstDepartureTime>09:30:00</FirstDepartureTime><LastDepartureTime>10:30:00</LastDepartureTime><ScheduledHeadwayInterval>PT30M</ScheduledHeadwayInterval></HeadwayJourneyGroup>"#,
        r#"<HeadwayJourneyGroup id="ch:1:HeadwayJourneyGroup:2" version="any"><FirstDepartureTime>23:50:00</FirstDepartureTime></HeadwayJourneyGroup>"#,
        "</frequencyGroups></TemplateServiceJourney>",
    );
    let documents = delivery(template);
    for index in [serially(&documents), merged(&documents)] {
        let journey = index.journey(
            index
                .journey_by_id("ch:1:ServiceJourney:fsu10")
                .expect("the template"),
        );
        assert_eq!(
            journey.headways,
            [Headway {
                first_departure: 9 * 3600 + 30 * 60,
                last_departure: 10 * 3600 + 30 * 60,
                interval: 30 * 60,
            }]
        );
        let departures: Vec<u32> = journey.headways[0].departures().collect();
        assert_eq!(departures, [34_200, 36_000, 37_800]);
        assert_eq!(
            findings(&index, Concern::HeadwayGroupWithoutRuns),
            ["headway group without a last departure or an interval, no runs: HeadwayJourneyGroup ch:1:HeadwayJourneyGroup:2 in PROD_NETEX_TT_1.10_CHE_SKI_2026_OEV-SCHWEIZ_TIMETABLE_1_515.xml (ch:1:ServiceJourney:fsu10)"]
        );
    }
}

#[test]
fn a_journey_of_a_flexible_line_is_left_out() {
    let flexible = concat!(
        r#"<ServiceJourney id="at:ktn:ServiceJourney:Berg-Bad-Bus_504" version="any">"#,
        r#"<validityConditions><AvailabilityConditionRef ref="ch:1:AvailabilityCondition:week" version="any"/></validityConditions>"#,
        r#"<DepartureTime>06:15:00</DepartureTime><TimeDemandTypeRef ref="at:ktn:TimeDemandType:NFZ:" version="any"/>"#,
        r#"<FlexibleLineRef ref="at:ktn:FlexibleLine:Berg-Bad-Bus:" version="any"/></ServiceJourney>"#,
    );
    let index = serially(&delivery(flexible));
    assert_eq!(index.journey_count(), 0);
    assert_eq!(
        findings(&index, Concern::FlexibleLineJourney),
        ["journey of a flexible line, not runs at fixed times: left out: ServiceJourney at:ktn:ServiceJourney:Berg-Bad-Bus_504 in PROD_NETEX_TT_1.10_CHE_SKI_2026_OEV-SCHWEIZ_TIMETABLE_1_515.xml (at:ktn:FlexibleLine:Berg-Bad-Bus:)"]
    );
    assert!(
        index.unresolved().is_empty(),
        "the flexible line is not looked for"
    );
}

/// The stop points of the calls a document states for the journey `id`, in
/// the order of the calls.
fn stated_calls(xml: &str, id: &str) -> Vec<String> {
    let document = netex_rs::read(xml).expect("the document reads");
    let journey = document
        .delivery
        .frames()
        .filter_map(|frame| match frame {
            Frame::Timetable(timetable) => timetable.vehicle_journeys.as_ref(),
            _ => None,
        })
        .flat_map(|journeys| &journeys.items)
        .find_map(|journey| match journey {
            VehicleJourney::ServiceJourney(journey)
                if journey.id.as_ref().map(|i| i.as_str()) == Some(id) =>
            {
                Some(journey)
            }
            _ => None,
        })
        .expect("the journey");
    let calls = journey.calls.as_ref().expect("calls");
    calls
        .items
        .iter()
        .filter_map(|item| match item {
            netex_rs::model::CallItem::Call(call) => match &call.scheduled_stop_point_ref {
                Some(StopPointRef::ScheduledStopPoint(r)) => {
                    r.r#ref.as_ref().map(|id| id.as_str().to_owned())
                }
                _ => None,
            },
            _ => None,
        })
        .collect()
}

/// A small line offer (`tests/fixtures/fragments/frames/journeys-in-place.xml`):
/// a journey whose day is an availability condition stated in place, and one
/// stating its calls.
#[test]
fn journeys_with_calls_and_conditions_in_place_are_read() {
    let name = "journeys-in-place.xml";
    let publication = support::fragments::document(name);
    let index = serially(&[(name.to_owned(), publication.clone())]);
    let days = |id: &str| -> Vec<i64> {
        let journey = index.journey_by_id(id).expect("the journey");
        index
            .days_of(journey)
            .map(|d| d - day("2026-06-01"))
            .collect()
    };
    assert_eq!(
        days("ex:ServiceJourney:conditioned"),
        [8, 9, 10],
        "9 to 11 June"
    );
    assert_eq!(days("ex:ServiceJourney:calling"), [0, 7, 14], "the Mondays");
    let calling = index.journey(
        index
            .journey_by_id("ex:ServiceJourney:calling")
            .expect("the calling journey"),
    );
    let stops: Vec<&str> = calling
        .stop_points
        .iter()
        .map(|&point| index.stop_point(point).id)
        .collect();
    assert_eq!(
        stops,
        stated_calls(&publication, "ex:ServiceJourney:calling"),
        "the calls it states"
    );
    assert_eq!(calling.passing_times[1].arrival, Some(9 * 3600 + 27 * 60));
}
