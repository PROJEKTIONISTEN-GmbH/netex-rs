//! What a feed needs of a delivery beyond what the index holds, read in the
//! same walk of each document as the index ([`IndexBuilder::add_streamed_with`]):
//! the organisations running the lines, destination displays, directions,
//! train numbers, the accessibility, platform codes and levels of stops, the
//! geometry of service links, the time zone and language of each document —
//! and every element the walk skipped, counted, so that the report says what
//! the feed does not carry.
//!
//! Each worker gathers into its own [`Gather`]; the gathers are merged once
//! every document is read ([`Gather::merge`]).
//!
//! [`IndexBuilder::add_streamed_with`]: netex_rs::index::IndexBuilder::add_streamed_with

use std::collections::BTreeMap;

use netex_rs::model::{
    self, AdditionalOperator, CallItem, DirectionType, Element, FrameDefaults, LimitationStatus,
    PointInSequenceItem, QuayItem, StopPointRef, StopUse, TrainNumberRefItem,
};
use netex_rs::stream::{Entity, Scope, Sink, Skipped};
use netex_rs::xml::{Child, Cursor};
use rustc_hash::FxHashMap;

use crate::names::{Name, Names};

/// Which way a journey runs, as GTFS's `direction_id` tells two apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    /// `outbound` (and `clockwise`): direction 0.
    Outbound,
    /// `inbound` (and `anticlockwise`): direction 1.
    Inbound,
}

impl Direction {
    fn of(direction: Option<&DirectionType>) -> Option<Direction> {
        match direction? {
            DirectionType::Outbound | DirectionType::Clockwise => Some(Direction::Outbound),
            DirectionType::Inbound | DirectionType::Anticlockwise => Some(Direction::Inbound),
            _ => None,
        }
    }
}

/// Whether a wheelchair user can use a stop or a vehicle, as GTFS says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Wheelchair {
    /// Accessible, at least in part: `1`.
    Accessible,
    /// Not accessible: `2`.
    Not,
}

impl Wheelchair {
    /// What an accessibility assessment's `MobilityImpairedAccess` says:
    /// `true` and `partial` accessible, `false` not, `unknown` nothing.
    fn of(assessment: Option<&model::AccessibilityAssessment>) -> Option<Wheelchair> {
        match assessment?.mobility_impaired_access.as_ref()? {
            LimitationStatus::True | LimitationStatus::Partial => Some(Wheelchair::Accessible),
            LimitationStatus::False => Some(Wheelchair::Not),
            _ => None,
        }
    }
}

/// An operator or an authority.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Organisation {
    pub(crate) document: Name,
    pub(crate) id: Name,
    pub(crate) name: Name,
    pub(crate) url: Option<Name>,
    pub(crate) phone: Option<Name>,
}

/// What a line states of the organisations running it and of its colours.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LineFacts {
    pub(crate) document: Name,
    pub(crate) id: Name,
    pub(crate) operator: Option<Name>,
    pub(crate) additional_operator: Option<Name>,
    pub(crate) authority: Option<Name>,
    pub(crate) short_name: Option<Name>,
    pub(crate) colour: Option<Name>,
    pub(crate) text_colour: Option<Name>,
}

/// A destination display: the text the vehicle shows.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Display {
    pub(crate) document: Name,
    pub(crate) id: Name,
    pub(crate) text: Name,
}

/// A point of a pattern, or a call of a journey stating its calls: its stop
/// point, the destination display it names, whether the vehicle passes it.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Point {
    pub(crate) stop_point: Name,
    pub(crate) display: Option<Name>,
    pub(crate) passes: bool,
}

/// What a journey pattern states beyond its calls: where it shows it goes
/// and which way it runs; its points, a range of the gather's points.
#[derive(Debug, Clone, Copy)]
pub(crate) struct PatternFacts {
    pub(crate) document: Name,
    pub(crate) id: Name,
    pub(crate) display: Option<Name>,
    pub(crate) direction: Option<Direction>,
    pub(crate) direction_ref: Option<Name>,
    pub(crate) points: (u32, u32),
}

/// What a journey states beyond its calls and times.
#[derive(Debug, Clone, Copy)]
pub(crate) struct JourneyFacts {
    pub(crate) document: Name,
    pub(crate) id: Name,
    pub(crate) direction: Option<Direction>,
    pub(crate) direction_ref: Option<Name>,
    pub(crate) operator: Option<Name>,
    pub(crate) public_code: Option<Name>,
    pub(crate) train_number: Option<Name>,
    pub(crate) wheelchair: Option<Wheelchair>,
    /// The calls it states, a range of the gather's points; empty for a
    /// journey following a pattern.
    pub(crate) calls: (u32, u32),
}

/// A train number: the number passengers see.
#[derive(Debug, Clone, Copy)]
pub(crate) struct TrainNumber {
    pub(crate) document: Name,
    pub(crate) id: Name,
    pub(crate) advertised: Name,
}

/// A direction: which way it is.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DirectionFacts {
    pub(crate) document: Name,
    pub(crate) id: Name,
    pub(crate) direction: Direction,
}

/// What a stop place or quay states beyond the index's record: whether a
/// wheelchair user can use it, its platform code, its level, its Swiss SLOID.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct StopFacts {
    /// The document whose definition is kept: the one whose name sorts first.
    pub(crate) document: Name,
    pub(crate) wheelchair: Option<Wheelchair>,
    pub(crate) platform: Option<Name>,
    pub(crate) level: Option<Name>,
    /// The stop place a stop place is within (`ParentSiteRef`).
    pub(crate) parent_site: Option<Name>,
    /// The Swiss location identifier, from the `SLOID` key.
    pub(crate) sloid: Option<Name>,
}

/// A level: its name and its number among the levels of a stop place.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LevelFacts {
    pub(crate) document: Name,
    pub(crate) name: Option<Name>,
    pub(crate) index: Option<f64>,
}

/// The geometry of a service link: from and to which stop point, in which
/// reference system, its positions as written — a range of the gather's
/// positions.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LinkGeometry {
    pub(crate) document: Name,
    pub(crate) from: Name,
    pub(crate) to: Name,
    pub(crate) system: Option<Name>,
    pub(crate) positions: (u32, u32),
}

/// What a document's frame defaults state: the first time zone, language
/// and location system it names.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct DocumentFacts {
    pub(crate) time_zone: Option<Name>,
    pub(crate) language: Option<Name>,
    pub(crate) location_system: Option<Name>,
    pub(crate) participant: Option<Name>,
    pub(crate) published: Option<Name>,
    /// Whether a frame of it is typed as one of the European Passenger
    /// Information Profile (`epip:…`).
    pub(crate) epip: bool,
}

/// Elements of the delivery the feed does not carry: how many, and the
/// document named first among those holding one.
#[derive(Debug, Clone, Copy)]
pub(crate) struct NotCarried {
    pub(crate) count: u64,
    pub(crate) first_document: Name,
}

/// Elements of a frame or a calendar that hold nothing a feed carries or
/// leaves out: the frame's own description, which the report leaves unsaid.
const FRAME_FURNITURE: &[&str] = &[
    "TypeOfFrameRef",
    "Name",
    "Description",
    "codespaces",
    "FrameDefaults",
    "ValidBetween",
    "validityConditions",
    "BaselineVersionFrameRef",
    "keyList",
    "FromDate",
    "ToDate",
    "PublicationTimestamp",
    "ParticipantRef",
    "PublicationRequest",
    "RequestTimestamp",
];

/// What one worker gathers from the documents it reads.
#[derive(Debug, Default)]
pub(crate) struct Gather {
    pub(crate) names: Names,
    document: Name,
    pub(crate) documents: FxHashMap<Name, DocumentFacts>,
    pub(crate) organisations: Vec<Organisation>,
    pub(crate) lines: Vec<LineFacts>,
    pub(crate) displays: Vec<Display>,
    pub(crate) points: Vec<Point>,
    pub(crate) patterns: Vec<PatternFacts>,
    pub(crate) journeys: Vec<JourneyFacts>,
    pub(crate) train_numbers: Vec<TrainNumber>,
    pub(crate) directions: Vec<DirectionFacts>,
    pub(crate) stops: FxHashMap<Name, StopFacts>,
    pub(crate) levels: FxHashMap<Name, LevelFacts>,
    pub(crate) links: Vec<LinkGeometry>,
    pub(crate) positions: Vec<(f64, f64)>,
    pub(crate) data_sources: Vec<(Name, Name, Option<Name>)>,
    pub(crate) not_carried: BTreeMap<Name, NotCarried>,
}

impl Gather {
    /// Starts gathering the document named `name`.
    pub(crate) fn begin(&mut self, name: &str) {
        self.document = self.names.intern(name);
        self.documents.entry(self.document).or_default();
    }

    fn intern(&mut self, text: Option<&str>) -> Option<Name> {
        text.map(|text| self.names.intern(text))
    }

    fn reference(&mut self, reference: Option<&model::Ref>) -> Option<Name> {
        let id = reference?.r#ref.as_ref()?;
        Some(self.names.intern(id.as_str()))
    }

    fn id(&mut self, id: Option<&model::Id>) -> Name {
        self.names.intern(id.map_or("", |id| id.as_str()))
    }

    fn not_carried(&mut self, element: &str) {
        let element = self.names.intern(element);
        let document = self.document;
        let entry = self.not_carried.entry(element).or_insert(NotCarried {
            count: 0,
            first_document: document,
        });
        entry.count += 1;
        if self.names.text(document) < self.names.text(entry.first_document) {
            entry.first_document = document;
        }
    }

    fn organisation(&mut self, organisation: &model::Organisation) {
        let text = text_of;
        let name = text(&organisation.name)
            .or(text(&organisation.short_name))
            .or(text(&organisation.legal_name))
            .or(text(&organisation.trading_name));
        let contact = organisation.contact_details.as_ref();
        let record = Organisation {
            document: self.document,
            id: self.id(organisation.id.as_ref()),
            name: self.names.intern(name.unwrap_or("")),
            url: self.intern(contact.and_then(|c| c.url.as_deref())),
            phone: self.intern(contact.and_then(|c| c.phone.as_deref())),
        };
        self.organisations.push(record);
    }

    fn line(&mut self, line: &model::Line) {
        self.notices(line.notice_assignments.as_ref());
        let additional = line
            .additional_operators
            .iter()
            .flat_map(|operators| &operators.items)
            .find_map(|item| match item {
                AdditionalOperator::OperatorRef(reference) => Some(&**reference),
                _ => None,
            });
        let (colour, text_colour) = presentation(line);
        let record = LineFacts {
            document: self.document,
            id: self.id(line.id.as_ref()),
            operator: self.reference(line.operator_ref.as_ref()),
            additional_operator: self.reference(additional),
            authority: self.reference(line.authority_ref.as_ref()),
            short_name: self.intern(line.short_name.as_ref().map(|n| n.as_str())),
            colour: self.intern(colour.as_deref()),
            text_colour: self.intern(text_colour.as_deref()),
        };
        self.lines.push(record);
    }

    fn display(&mut self, display: &model::DestinationDisplay) {
        let text = text_of;
        let Some(shown) = text(&display.front_text)
            .or(text(&display.name))
            .or(text(&display.side_text))
            .filter(|shown| !shown.trim().is_empty())
        else {
            return;
        };
        let record = Display {
            document: self.document,
            id: self.id(display.id.as_ref()),
            text: self.names.intern(shown.trim()),
        };
        self.displays.push(record);
    }

    /// Counts the notice assignments of an entity or a point of it, which a
    /// feed does not carry.
    fn notices(&mut self, assignments: Option<&model::NoticeAssignments>) {
        for _ in assignments.iter().flat_map(|a| &a.notice_assignments) {
            self.not_carried("NoticeAssignment");
        }
    }

    fn pattern(&mut self, pattern: &model::ServiceJourneyPattern) {
        self.notices(pattern.notice_assignments.as_ref());
        let start = position_of(self.points.len());
        let mut any_display = pattern.destination_display_ref.is_some();
        for item in pattern.points_in_sequence.iter().flat_map(|p| &p.items) {
            let PointInSequenceItem::StopPointInJourneyPattern(point) = item else {
                continue;
            };
            self.notices(point.notice_assignments.as_ref());
            let Some(stop_point) = stop_point_ref(point.scheduled_stop_point_ref.as_ref()) else {
                continue;
            };
            let display = self.reference(point.destination_display_ref.as_ref());
            any_display |= display.is_some();
            let record = Point {
                stop_point: self.names.intern(stop_point),
                display,
                passes: matches!(point.stop_use, Some(StopUse::Passthrough)),
            };
            self.points.push(record);
        }
        let direction = Direction::of(pattern.direction_type.as_ref());
        let direction_ref = self.reference(pattern.direction_ref.as_ref());
        if !any_display && direction.is_none() && direction_ref.is_none() {
            // Nothing a feed takes from it beyond the index's calls.
            self.points.truncate(start as usize);
            return;
        }
        let record = PatternFacts {
            document: self.document,
            id: self.id(pattern.id.as_ref()),
            display: self.reference(pattern.destination_display_ref.as_ref()),
            direction,
            direction_ref,
            points: (start, position_of(self.points.len())),
        };
        self.patterns.push(record);
    }

    fn journey(&mut self, journey: &model::ServiceJourney) {
        self.notices(journey.notice_assignments.as_ref());
        let start = position_of(self.points.len());
        let mut any_display = false;
        for item in journey.calls.iter().flat_map(|c| &c.items) {
            let CallItem::Call(call) = item else {
                continue;
            };
            self.notices(call.notice_assignments.as_ref());
            let Some(stop_point) = stop_point_ref(call.scheduled_stop_point_ref.as_ref()) else {
                continue;
            };
            let display = self.reference(call.destination_display_ref.as_ref());
            any_display |= display.is_some();
            let record = Point {
                stop_point: self.names.intern(stop_point),
                display,
                passes: matches!(call.stop_use, Some(StopUse::Passthrough)),
            };
            self.points.push(record);
        }
        if !any_display {
            self.points.truncate(start as usize);
        }
        let train_number = journey
            .train_numbers
            .iter()
            .flat_map(|numbers| &numbers.items)
            .find_map(|item| match item {
                TrainNumberRefItem::TrainNumberRef(reference) => Some(&**reference),
                _ => None,
            });
        let record = JourneyFacts {
            document: self.document,
            // Interned below, only for a journey kept: most state nothing more.
            id: Name::MAX,
            direction: Direction::of(journey.direction_type.as_ref()),
            direction_ref: self.reference(journey.direction_ref.as_ref()),
            operator: self.reference(journey.operator_ref.as_ref()),
            public_code: self.intern(journey.public_code.as_deref()),
            train_number: self.reference(train_number),
            wheelchair: Wheelchair::of(journey.accessibility_assessment.as_ref()),
            calls: (start, position_of(self.points.len())),
        };
        let says_something = record.direction.is_some()
            || record.direction_ref.is_some()
            || record.operator.is_some()
            || record.public_code.is_some()
            || record.train_number.is_some()
            || record.wheelchair.is_some()
            || record.calls.0 < record.calls.1;
        if says_something {
            let id = self.id(journey.id.as_ref());
            self.journeys.push(JourneyFacts { id, ..record });
        }
    }

    fn stop_place(&mut self, place: &model::StopPlace) {
        let wheelchair = Wheelchair::of(place.accessibility_assessment.as_ref()).or(
            match place.all_areas_wheelchair_accessible {
                Some(true) => Some(Wheelchair::Accessible),
                Some(false) => Some(Wheelchair::Not),
                None => None,
            },
        );
        let facts = StopFacts {
            document: self.document,
            wheelchair,
            platform: None,
            level: None,
            parent_site: self.reference(place.parent_site_ref.as_ref()),
            sloid: self.intern(sloid(place.key_list.as_ref())),
        };
        let id = self.id(place.id.as_ref());
        self.stop(id, facts);
        for level in place.levels.iter().flat_map(|levels| &levels.levels) {
            let id = self.id(level.id.as_ref());
            let name = level
                .name
                .as_ref()
                .map(|n| n.as_str())
                .or(level.short_name.as_ref().map(|n| n.as_str()));
            let index = level
                .public_code
                .as_deref()
                .and_then(|code| code.trim().parse::<f64>().ok())
                .or(level.relative_level_order.map(|order| order as f64));
            let facts = LevelFacts {
                document: self.document,
                name: self.intern(name),
                index,
            };
            let names = &self.names;
            self.levels
                .entry(id)
                .and_modify(|kept| {
                    if names.text(facts.document) < names.text(kept.document) {
                        *kept = facts;
                    }
                })
                .or_insert(facts);
        }
        for item in place.quays.iter().flat_map(|quays| &quays.items) {
            if let QuayItem::Quay(quay) = item {
                self.quay(quay);
            }
        }
    }

    /// A quay, in its stop place or defined beside it.
    fn quay(&mut self, quay: &model::Quay) {
        let facts = StopFacts {
            document: self.document,
            wheelchair: Wheelchair::of(quay.accessibility_assessment.as_ref()),
            platform: self.intern(quay.public_code.as_deref()),
            level: self.reference(quay.level_ref.as_ref()),
            parent_site: None,
            sloid: self.intern(sloid(quay.key_list.as_ref())),
        };
        let id = self.id(quay.id.as_ref());
        self.stop(id, facts);
    }

    /// Keeps what a stop place or quay states, as the definition of the
    /// document named first states it — the definition the index keeps.
    fn stop(&mut self, id: Name, facts: StopFacts) {
        let names = &self.names;
        self.stops
            .entry(id)
            .and_modify(|kept| {
                if names.text(facts.document) < names.text(kept.document) {
                    *kept = facts;
                }
            })
            .or_insert(facts);
    }

    fn link(&mut self, link: &model::ServiceLink) {
        let (Some(from), Some(to)) = (
            link.from_point_ref.as_ref().and_then(|r| r.r#ref.as_ref()),
            link.to_point_ref.as_ref().and_then(|r| r.r#ref.as_ref()),
        ) else {
            return;
        };
        let Some(geometry) = link
            .unprocessed
            .iter()
            .filter(|fragment| &*fragment.name == "LineString")
            .find_map(|fragment| netex_rs::geometry::geometry(&fragment.xml))
        else {
            return;
        };
        let start = position_of(self.positions.len());
        self.positions.extend(geometry.positions());
        let record = LinkGeometry {
            document: self.document,
            from: self.names.intern(from.as_str()),
            to: self.names.intern(to.as_str()),
            system: self.intern(geometry.srs_name.as_deref()),
            positions: (start, position_of(self.positions.len())),
        };
        self.links.push(record);
    }

    fn frame_defaults(&mut self, defaults: &FrameDefaults) {
        let locale = defaults.default_locale.as_ref();
        let time_zone = self.intern(locale.and_then(|l| l.time_zone.as_deref()));
        let language = self.intern(locale.and_then(|l| l.default_language.as_deref()));
        let system = self.intern(defaults.default_location_system.as_deref());
        let facts = self.documents.entry(self.document).or_default();
        facts.time_zone = facts.time_zone.or(time_zone);
        facts.language = facts.language.or(language);
        facts.location_system = facts.location_system.or(system);
    }

    /// A train number: its identifier and the number it advertises
    /// (`ForAdvertisement`, else `Description`).
    fn train_number(&mut self, number: &model::TrainNumber) {
        let advertised = number
            .for_advertisement
            .as_deref()
            .or(number.description.as_ref().map(|d| d.as_str()))
            .map(str::trim)
            .filter(|advertised| !advertised.is_empty());
        if let (Some(id), Some(advertised)) = (number.id.as_ref(), advertised) {
            let record = TrainNumber {
                document: self.document,
                id: self.names.intern(id.as_str()),
                advertised: self.names.intern(advertised),
            };
            self.train_numbers.push(record);
        }
    }

    /// Takes in what `other` gathered.
    pub(crate) fn merge(&mut self, other: Gather) {
        let renamed = self.names.absorb(&other.names);
        let name = |name: Name| renamed[name as usize];
        let option = |name: Option<Name>| name.map(|n| renamed[n as usize]);
        for (document, facts) in other.documents {
            self.documents.insert(
                name(document),
                DocumentFacts {
                    time_zone: option(facts.time_zone),
                    language: option(facts.language),
                    location_system: option(facts.location_system),
                    participant: option(facts.participant),
                    published: option(facts.published),
                    epip: facts.epip,
                },
            );
        }
        self.organisations
            .extend(other.organisations.into_iter().map(|o| Organisation {
                document: name(o.document),
                id: name(o.id),
                name: name(o.name),
                url: option(o.url),
                phone: option(o.phone),
            }));
        self.lines
            .extend(other.lines.into_iter().map(|l| LineFacts {
                document: name(l.document),
                id: name(l.id),
                operator: option(l.operator),
                additional_operator: option(l.additional_operator),
                authority: option(l.authority),
                short_name: option(l.short_name),
                colour: option(l.colour),
                text_colour: option(l.text_colour),
            }));
        self.displays
            .extend(other.displays.into_iter().map(|d| Display {
                document: name(d.document),
                id: name(d.id),
                text: name(d.text),
            }));
        let offset = position_of(self.points.len());
        self.points.extend(other.points.into_iter().map(|p| Point {
            stop_point: name(p.stop_point),
            display: option(p.display),
            passes: p.passes,
        }));
        let shift = |(start, end): (u32, u32)| (start + offset, end + offset);
        self.patterns
            .extend(other.patterns.into_iter().map(|p| PatternFacts {
                document: name(p.document),
                id: name(p.id),
                display: option(p.display),
                direction: p.direction,
                direction_ref: option(p.direction_ref),
                points: shift(p.points),
            }));
        self.journeys
            .extend(other.journeys.into_iter().map(|j| JourneyFacts {
                document: name(j.document),
                id: name(j.id),
                direction: j.direction,
                direction_ref: option(j.direction_ref),
                operator: option(j.operator),
                public_code: option(j.public_code),
                train_number: option(j.train_number),
                wheelchair: j.wheelchair,
                calls: shift(j.calls),
            }));
        self.train_numbers
            .extend(other.train_numbers.into_iter().map(|t| TrainNumber {
                document: name(t.document),
                id: name(t.id),
                advertised: name(t.advertised),
            }));
        self.directions
            .extend(other.directions.into_iter().map(|d| DirectionFacts {
                document: name(d.document),
                id: name(d.id),
                direction: d.direction,
            }));
        for (id, facts) in other.stops {
            let facts = StopFacts {
                document: name(facts.document),
                wheelchair: facts.wheelchair,
                platform: option(facts.platform),
                level: option(facts.level),
                parent_site: option(facts.parent_site),
                sloid: option(facts.sloid),
            };
            self.stop(name(id), facts);
        }
        for (id, facts) in other.levels {
            let facts = LevelFacts {
                document: name(facts.document),
                name: option(facts.name),
                index: facts.index,
            };
            let names = &self.names;
            self.levels
                .entry(name(id))
                .and_modify(|kept| {
                    if names.text(facts.document) < names.text(kept.document) {
                        *kept = facts;
                    }
                })
                .or_insert(facts);
        }
        let position_offset = position_of(self.positions.len());
        self.positions.extend(other.positions);
        self.links
            .extend(other.links.into_iter().map(|l| LinkGeometry {
                document: name(l.document),
                from: name(l.from),
                to: name(l.to),
                system: option(l.system),
                positions: (
                    l.positions.0 + position_offset,
                    l.positions.1 + position_offset,
                ),
            }));
        self.data_sources.extend(
            other
                .data_sources
                .into_iter()
                .map(|(document, source, url)| (name(document), name(source), option(url))),
        );
        for (element, not_carried) in other.not_carried {
            let element = name(element);
            let first_document = name(not_carried.first_document);
            let names = &self.names;
            self.not_carried
                .entry(element)
                .and_modify(|kept| {
                    kept.count += not_carried.count;
                    if names.text(first_document) < names.text(kept.first_document) {
                        kept.first_document = first_document;
                    }
                })
                .or_insert(NotCarried {
                    count: not_carried.count,
                    first_document,
                });
        }
    }
}

impl<'i> Sink<'i> for Gather {
    fn entity(&mut self, entity: Entity, _: &Scope) -> netex_rs::Result<()> {
        match &entity {
            Entity::StopPlace(place) => self.stop_place(place),
            Entity::Quay(quay) => self.quay(quay),
            Entity::TrainNumber(number) => self.train_number(number),
            Entity::Operator(organisation) | Entity::Authority(organisation) => {
                self.organisation(organisation)
            }
            Entity::DataSource(source) => {
                if let Some(text) = source.name.as_ref().map(|n| n.as_str()) {
                    let record = (
                        self.document,
                        self.names.intern(text),
                        self.intern(source.url.as_deref()),
                    );
                    self.data_sources.push(record);
                }
            }
            Entity::Line(line) => self.line(line),
            Entity::DestinationDisplay(display) => self.display(display),
            Entity::Direction(direction) => {
                if let Some(which) = Direction::of(direction.direction_type.as_ref()) {
                    let record = DirectionFacts {
                        document: self.document,
                        id: self.id(direction.id.as_ref()),
                        direction: which,
                    };
                    self.directions.push(record);
                }
            }
            Entity::ServiceJourneyPattern(pattern) | Entity::JourneyPattern(pattern) => {
                self.pattern(pattern)
            }
            Entity::ServiceJourney(journey)
            | Entity::DatedServiceJourney(journey)
            | Entity::TemplateServiceJourney(journey) => self.journey(journey),
            Entity::ServiceLink(link) => self.link(link),
            // Read for the index or for the feed's furniture, but not carried:
            // GTFS has no notices, and its transfers name stops and trips,
            // not the lines and stop areas a rule names.
            Entity::Notice(_) => self.not_carried("Notice"),
            Entity::NoticeAssignment(_) => self.not_carried("NoticeAssignment"),
            Entity::InterchangeRule(_) => self.not_carried("InterchangeRule"),
            _ => {}
        }
        Ok(())
    }

    fn skipped(
        &mut self,
        cursor: &mut Cursor<'i>,
        element: &Child<'i>,
        place: Skipped<'i>,
        _: &Scope,
    ) -> netex_rs::Result<()> {
        match (place, element.local) {
            (Skipped::Frame(_), "FrameDefaults") => {
                let defaults = FrameDefaults::read(cursor, element)?;
                self.frame_defaults(&defaults);
                return Ok(());
            }
            (Skipped::Delivery, "ParticipantRef" | "PublicationTimestamp") => {
                let text = cursor.text(element)?;
                let text = Some(self.names.intern(text.trim()));
                let facts = self.documents.entry(self.document).or_default();
                if element.local == "ParticipantRef" {
                    facts.participant = text;
                } else {
                    facts.published = text;
                }
                return Ok(());
            }
            (Skipped::Frame(_), "TypeOfFrameRef") => {
                for attribute in element.attributes() {
                    let (key, value) = attribute?;
                    if key == "ref" && value.starts_with("epip:") {
                        self.documents.entry(self.document).or_default().epip = true;
                    }
                }
            }
            (Skipped::Item(_), item) => self.not_carried(item),
            (Skipped::Frame(_) | Skipped::Calendar | Skipped::Frames, name)
                if !FRAME_FURNITURE.contains(&name) =>
            {
                self.not_carried(name)
            }
            _ => {}
        }
        cursor.skip_over(element)
    }
}

/// The text of a multilingual string, if there is one.
fn text_of(name: &Option<model::MultilingualString>) -> Option<&str> {
    name.as_ref().map(|name| name.as_str())
}

/// The Swiss location identifier a stop place or quay carries.
fn sloid(keys: Option<&model::KeyList>) -> Option<&str> {
    keys.and_then(|keys| keys.get("SLOID"))
}

/// The identifier of the stop point a point of a pattern or a call names.
fn stop_point_ref(reference: Option<&StopPointRef>) -> Option<&str> {
    match reference? {
        StopPointRef::ScheduledStopPoint(r) | StopPointRef::FareScheduledStopPoint(r) => {
            r.r#ref.as_ref().map(|id| id.as_str())
        }
    }
}

/// A line's colour and text colour from its `Presentation`, which the model
/// carries through untyped: six hexadecimal digits each.
fn presentation(line: &model::Line) -> (Option<String>, Option<String>) {
    let Some(fragment) = line
        .unprocessed
        .iter()
        .find(|fragment| &*fragment.name == "Presentation")
    else {
        return (None, None);
    };
    let colour = |element: &str| {
        let start = fragment.xml.find(&format!("<{element}>"))? + element.len() + 2;
        let end = start + fragment.xml[start..].find('<')?;
        let value = fragment.xml[start..end].trim().trim_start_matches('#');
        (value.len() == 6 && value.bytes().all(|b| b.is_ascii_hexdigit()))
            .then(|| value.to_ascii_uppercase())
    };
    (colour("Colour"), colour("TextColour"))
}

fn position_of(position: usize) -> u32 {
    u32::try_from(position).expect("fewer than 2^32 entries")
}
