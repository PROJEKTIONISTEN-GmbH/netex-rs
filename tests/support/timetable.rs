//! Small timetable documents written for a test: the frames a line file of the
//! Austrian or German deliveries holds, cut down to what one question needs.
//!
//! A [`Timetable`] collects lines, stop points, journey patterns, a calendar
//! and journeys, and writes them as one document, every container in schema
//! order — the order the lean reader meets them in, so that the full model and
//! the lean stream index the document alike.

use std::fmt::Write as _;

use netex_rs::index::{Index, IndexBuilder};
use netex_rs::stream::{ReadOptions, Selection};

/// One point of a journey pattern: its identifier, the stop point it is at,
/// and what it states about stopping there.
#[derive(Debug, Clone, Copy)]
pub struct Point<'a> {
    pub id: &'a str,
    pub stop_point: &'a str,
    /// `StopUse`, where the point states one.
    pub stop_use: Option<&'a str>,
    /// `ForBoarding`, where the point states it.
    pub boarding: Option<bool>,
    /// `ForAlighting`, where the point states it.
    pub alighting: Option<bool>,
}

impl<'a> Point<'a> {
    /// A point stating nothing but its stop point.
    pub fn at(id: &'a str, stop_point: &'a str) -> Self {
        Point {
            id,
            stop_point,
            stop_use: None,
            boarding: None,
            alighting: None,
        }
    }
}

/// A passing time: the pattern point it is at, arrival and departure as written.
pub type Passing<'a> = (&'a str, Option<&'a str>, Option<&'a str>);

/// A document under construction.
#[derive(Debug, Default, Clone)]
pub struct Timetable {
    stop_places: String,
    lines: String,
    stop_points: String,
    patterns: String,
    day_types: String,
    periods: String,
    assignments: String,
    journeys: String,
}

impl Timetable {
    pub fn new() -> Self {
        Self::default()
    }

    /// A stop place with one quay, `<id>:1`.
    pub fn stop_place(mut self, id: &str, name: &str) -> Self {
        let _ = write!(
            self.stop_places,
            r#"<StopPlace id="{id}" version="1"><Name>{name}</Name><quays><Quay id="{id}:1" version="1"><Name>{name}</Name></Quay></quays></StopPlace>"#
        );
        self
    }

    /// A stop place standing at a position, longitude and latitude in
    /// degrees, with no quay.
    pub fn stop_place_at(
        mut self,
        id: &str,
        name: &str,
        (longitude, latitude): (f64, f64),
    ) -> Self {
        let _ = write!(
            self.stop_places,
            r#"<StopPlace id="{id}" version="1"><Name>{name}</Name><Centroid><Location><Longitude>{longitude}</Longitude><Latitude>{latitude}</Latitude></Location></Centroid></StopPlace>"#
        );
        self
    }

    /// A line, in a version, valid from one day — to another, where given —
    /// where given.
    pub fn line(mut self, id: &str, version: &str, valid: Option<(&str, Option<&str>)>) -> Self {
        let valid = valid.map_or(String::new(), |(from, to)| {
            let to = to.map_or(String::new(), |to| {
                format!("<ToDate>{to}T23:59:59</ToDate>")
            });
            format!("<ValidBetween><FromDate>{from}T00:00:00</FromDate>{to}</ValidBetween>")
        });
        let _ = write!(
            self.lines,
            r#"<Line id="{id}" version="{version}">{valid}<Name>{id}</Name><TransportMode>bus</TransportMode></Line>"#
        );
        self
    }

    /// A scheduled stop point, named as it is identified.
    pub fn stop_point(mut self, id: &str) -> Self {
        let _ = write!(
            self.stop_points,
            r#"<ScheduledStopPoint id="{id}" version="1"><Name>{id}</Name></ScheduledStopPoint>"#
        );
        self
    }

    /// A journey pattern of `line` over `points`.
    pub fn pattern(mut self, id: &str, line: &str, points: &[Point<'_>]) -> Self {
        let _ = write!(
            self.patterns,
            r#"<ServiceJourneyPattern id="{id}" version="1"><RouteView><LineRef ref="{line}" version="any"/></RouteView><pointsInSequence>"#
        );
        for (order, point) in points.iter().enumerate() {
            let _ = write!(
                self.patterns,
                r#"<StopPointInJourneyPattern id="{}" version="1" order="{}"><ScheduledStopPointRef ref="{}" version="1"/>"#,
                point.id,
                order + 1,
                point.stop_point
            );
            if let Some(alighting) = point.alighting {
                let _ = write!(self.patterns, "<ForAlighting>{alighting}</ForAlighting>");
            }
            if let Some(boarding) = point.boarding {
                let _ = write!(self.patterns, "<ForBoarding>{boarding}</ForBoarding>");
            }
            if let Some(stop_use) = point.stop_use {
                let _ = write!(self.patterns, "<StopUse>{stop_use}</StopUse>");
            }
            self.patterns.push_str("</StopPointInJourneyPattern>");
        }
        self.patterns
            .push_str("</pointsInSequence></ServiceJourneyPattern>");
        self
    }

    /// A day type.
    pub fn day_type(mut self, id: &str) -> Self {
        let _ = write!(
            self.day_types,
            r#"<DayType id="{id}" version="1"><Name>{id}</Name></DayType>"#
        );
        self
    }

    /// A day type valid from `valid`'s first to its last day, where given,
    /// with a property of day for each of `properties`, the property's
    /// children as written: `<DaysOfWeek>Saturday</DaysOfWeek>`.
    pub fn day_type_with(
        mut self,
        id: &str,
        valid: Option<(&str, &str)>,
        properties: &[&str],
    ) -> Self {
        let valid = valid.map_or(String::new(), |(from, to)| {
            format!("<ValidBetween><FromDate>{from}T00:00:00</FromDate><ToDate>{to}T00:00:00</ToDate></ValidBetween>")
        });
        let properties: String = properties
            .iter()
            .map(|property| format!("<PropertyOfDay>{property}</PropertyOfDay>"))
            .collect();
        let _ = write!(
            self.day_types,
            r#"<DayType id="{id}" version="1">{valid}<Name>{id}</Name><properties>{properties}</properties></DayType>"#
        );
        self
    }

    /// A UIC operating period from `from` to `to` with a bit per day.
    pub fn uic_period(mut self, id: &str, from: &str, to: &str, bits: &str) -> Self {
        let _ = write!(
            self.periods,
            r#"<UicOperatingPeriod id="{id}" version="1"><FromDate>{from}T00:00:00</FromDate><ToDate>{to}T00:00:00</ToDate><ValidDayBits>{bits}</ValidDayBits></UicOperatingPeriod>"#
        );
        self
    }

    /// A plain operating period, every day from `from` — where it states one —
    /// to `to`.
    pub fn period(mut self, id: &str, from: Option<&str>, to: &str) -> Self {
        let from = from.map_or(String::new(), |from| {
            format!("<FromDate>{from}T00:00:00</FromDate>")
        });
        let _ = write!(
            self.periods,
            r#"<OperatingPeriod id="{id}" version="1">{from}<ToDate>{to}T00:00:00</ToDate></OperatingPeriod>"#
        );
        self
    }

    /// A day type assignment of `period` to `day_type`, in `order`, available or not.
    pub fn assignment(
        mut self,
        id: &str,
        order: u32,
        period: &str,
        day_type: &str,
        available: bool,
    ) -> Self {
        let available = if available {
            String::new()
        } else {
            "<isAvailable>false</isAvailable>".to_owned()
        };
        let _ = write!(
            self.assignments,
            r#"<DayTypeAssignment id="{id}" version="1" order="{order}"><OperatingPeriodRef ref="{period}" version="1"/><DayTypeRef ref="{day_type}" version="1"/>{available}</DayTypeAssignment>"#
        );
        self
    }

    /// A day type assignment of the date `date` to `day_type`, in `order`,
    /// available or not.
    pub fn date_assignment(
        mut self,
        id: &str,
        order: u32,
        date: &str,
        day_type: &str,
        available: bool,
    ) -> Self {
        let _ = write!(
            self.assignments,
            r#"<DayTypeAssignment id="{id}" version="1" order="{order}"><Date>{date}</Date><DayTypeRef ref="{day_type}" version="1"/><isAvailable>{available}</isAvailable></DayTypeAssignment>"#
        );
        self
    }

    /// A journey of `line` in `line_version` along `pattern` on `day_types`,
    /// whose passing times — arrival and departure as written — name no point
    /// of the pattern, as NeTEx 1.x allows.
    pub fn journey_by_position(
        mut self,
        id: &str,
        (line, line_version): (&str, &str),
        pattern: &str,
        day_types: &[&str],
        times: &[(Option<&str>, Option<&str>)],
    ) -> Self {
        let passing: String = times
            .iter()
            .map(|(arrival, departure)| {
                let arrival = arrival.map_or(String::new(), |time| {
                    format!("<ArrivalTime>{time}</ArrivalTime>")
                });
                let departure = departure.map_or(String::new(), |time| {
                    format!("<DepartureTime>{time}</DepartureTime>")
                });
                format!(r#"<TimetabledPassingTime version="1">{arrival}{departure}</TimetabledPassingTime>"#)
            })
            .collect();
        let day_types: String = day_types
            .iter()
            .map(|day_type| format!(r#"<DayTypeRef ref="{day_type}" version="1"/>"#))
            .collect();
        let _ = write!(
            self.journeys,
            r#"<ServiceJourney id="{id}" version="1"><dayTypes>{day_types}</dayTypes><ServiceJourneyPatternRef ref="{pattern}" version="1"/><LineRef ref="{line}" version="{line_version}"/><passingTimes>{passing}</passingTimes></ServiceJourney>"#
        );
        self
    }

    /// A journey of `line` in `line_version` along `pattern` on `day_types`,
    /// with its passing times.
    pub fn journey(
        mut self,
        id: &str,
        (line, line_version): (&str, &str),
        pattern: &str,
        day_types: &[&str],
        times: &[Passing<'_>],
    ) -> Self {
        let _ = write!(
            self.journeys,
            r#"<ServiceJourney id="{id}" version="1"><dayTypes>"#
        );
        for day_type in day_types {
            let _ = write!(
                self.journeys,
                r#"<DayTypeRef ref="{day_type}" version="1"/>"#
            );
        }
        let _ = write!(
            self.journeys,
            r#"</dayTypes><ServiceJourneyPatternRef ref="{pattern}" version="1"/><LineRef ref="{line}" version="{line_version}"/><passingTimes>"#
        );
        for (point, arrival, departure) in times {
            let _ = write!(
                self.journeys,
                r#"<TimetabledPassingTime version="1"><StopPointInJourneyPatternRef ref="{point}" version="1"/>"#
            );
            if let Some(arrival) = arrival {
                let _ = write!(self.journeys, "<ArrivalTime>{arrival}</ArrivalTime>");
            }
            if let Some(departure) = departure {
                let _ = write!(self.journeys, "<DepartureTime>{departure}</DepartureTime>");
            }
            self.journeys.push_str("</TimetabledPassingTime>");
        }
        self.journeys.push_str("</passingTimes></ServiceJourney>");
        self
    }

    /// The document.
    pub fn xml(&self) -> String {
        let container = |name: &str, content: &str| {
            if content.is_empty() {
                String::new()
            } else {
                format!("<{name}>{content}</{name}>")
            }
        };
        format!(
            concat!(
                r#"<PublicationDelivery xmlns="http://www.netex.org.uk/netex" version="ntx:1.1">"#,
                "<PublicationTimestamp>2026-10-14T00:00:00</PublicationTimestamp>",
                "<ParticipantRef>test</ParticipantRef>",
                r#"<dataObjects><CompositeFrame id="c" version="1"><frames>"#,
                r#"<SiteFrame id="g" version="1">{}</SiteFrame>"#,
                r#"<ServiceFrame id="s" version="1">{}{}{}</ServiceFrame>"#,
                r#"<ServiceCalendarFrame id="k" version="1">{}{}{}</ServiceCalendarFrame>"#,
                r#"<TimetableFrame id="t" version="1">{}</TimetableFrame>"#,
                "</frames></CompositeFrame></dataObjects></PublicationDelivery>"
            ),
            container("stopPlaces", &self.stop_places),
            container("lines", &self.lines),
            container("scheduledStopPoints", &self.stop_points),
            container("journeyPatterns", &self.patterns),
            container("dayTypes", &self.day_types),
            container("operatingPeriods", &self.periods),
            container("dayTypeAssignments", &self.assignments),
            container("vehicleJourneys", &self.journeys),
        )
    }
}

/// A line document of the Linz AG's kind, cut down: a line, two stop points,
/// a pattern, the calendar `DT` over the period `OP` with the bits given — the
/// same identifiers in every document, other days in each — and a journey on it.
pub fn line_document(journey: &str, bits: &str, departure: &str) -> String {
    Timetable::new()
        .line("L", "1", None)
        .stop_point("A")
        .stop_point("B")
        .pattern("P", "L", &[Point::at("P1", "A"), Point::at("P2", "B")])
        .day_type("DT")
        .uic_period("OP", "2026-10-12", "2026-10-18", bits)
        .assignment("DTA", 1, "OP", "DT", true)
        .journey(
            journey,
            ("L", "1"),
            "P",
            &["DT"],
            &[
                ("P1", None, Some(departure)),
                ("P2", Some("08:30:00"), None),
            ],
        )
        .xml()
}

/// Two documents of one delivery defining the same calendar identifiers with
/// other days: Monday 12 October in the first, Tuesday 13 October in the second.
pub fn two_calendars() -> Vec<(String, String)> {
    vec![
        (
            "NX-PI_01_at_esg_LINE_esg_1-2-E-e25_20260923.xml".to_owned(),
            line_document("J-e25", "1000000", "08:00:00"),
        ),
        (
            "NX-PI_01_at_esg_LINE_esg_3-46-E-f26_20260923.xml".to_owned(),
            line_document("J-f26", "0100000", "08:05:00"),
        ),
    ]
}

/// The index of `documents` built by one builder, in the order given, from the
/// full model.
pub fn serially(documents: &[(String, String)]) -> Index {
    let mut builder = IndexBuilder::new();
    for (name, xml) in documents {
        builder.add(&netex_rs::read(xml).expect("the document reads"), name);
    }
    builder.finish()
}

/// The index of `documents` built by one builder per document, lean, the
/// builders merged in the order given.
pub fn merged(documents: &[(String, String)]) -> Index {
    let options = ReadOptions::new(Selection::routing());
    let builders = documents.iter().map(|(name, xml)| {
        let mut builder = IndexBuilder::new();
        builder
            .add_streamed(xml, &options, name)
            .expect("the document reads");
        builder
    });
    let mut builders = builders.collect::<Vec<_>>().into_iter();
    let mut first = builders.next().expect("at least one document");
    for builder in builders {
        first.merge(builder);
    }
    first.finish()
}

pub fn reversed(documents: &[(String, String)]) -> Vec<(String, String)> {
    documents.iter().rev().cloned().collect()
}
