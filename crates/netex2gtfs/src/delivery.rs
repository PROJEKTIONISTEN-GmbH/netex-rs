//! A delivery read once: the index, and what a feed needs beyond it.

use std::hash::Hash;

use netex_rs::dataset::Source;
use netex_rs::index::{Index, IndexBuilder, JourneyId, LineId};
use netex_rs::stream::{Kind, ReadOptions, Selection};
use rustc_hash::FxHashMap;

use crate::gather::{
    Direction, DirectionFacts, Display, DocumentFacts, Gather, JourneyFacts, LineFacts,
    LinkGeometry, Organisation, PatternFacts, PatternLinks, Point, ProjectionGeometry, TrainNumber,
};
use crate::names::{Name, Names};

/// What a delivery is read for, beyond the index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Reading {
    /// Keep what refers to what ([`IndexBuilder::with_referrers`]).
    pub referrers: bool,
    /// Read the service links with their geometry, and the links each
    /// journey pattern names, for the feed's shapes.
    pub geometry: bool,
}

/// What a selection reads for a feed: the routing entities, and the
/// organisations, data sources, destination displays, directions and train
/// numbers; the service links where their geometry is wanted.
pub fn selection(reading: Reading) -> Selection {
    let selection = [
        Kind::Operator,
        Kind::Authority,
        Kind::DataSource,
        Kind::DestinationDisplay,
        Kind::Direction,
        Kind::InterchangeRule,
        Kind::TrainNumber,
    ]
    .into_iter()
    .fold(Selection::routing(), Selection::with);
    if reading.geometry {
        selection.with(Kind::ServiceLink)
    } else {
        selection
    }
}

/// Definitions each document makes of its own, by identifier (or another
/// key): a reference resolves in its own document first, then in the
/// delivery, where the document whose name sorts first answers — as the index
/// resolves them.
#[derive(Debug)]
struct Scoped<K = Name> {
    /// Per identifier, the positions of its definitions, the document whose
    /// name sorts first first.
    by_id: FxHashMap<K, Vec<u32>>,
}

impl<K: Copy + Eq + Hash> Scoped<K> {
    /// The definitions of `records`, each by its document and key.
    fn of<T>(names: &Names, records: &[T], key: impl Fn(&T) -> (Name, K)) -> Scoped<K> {
        Self::of_those(names, records, |_| true, key)
    }

    /// The definitions of those of `records` that `keep` keeps.
    fn of_those<T>(
        names: &Names,
        records: &[T],
        keep: impl Fn(&T) -> bool,
        key: impl Fn(&T) -> (Name, K),
    ) -> Scoped<K> {
        let mut by_id: FxHashMap<K, Vec<u32>> = FxHashMap::default();
        for (position, record) in records.iter().enumerate() {
            if !keep(record) {
                continue;
            }
            let (_, id) = key(record);
            by_id
                .entry(id)
                .or_default()
                .push(u32::try_from(position).expect("fewer than 2^32 definitions"));
        }
        for positions in by_id.values_mut() {
            positions.sort_by(|&a, &b| {
                let document = |p: u32| names.text(key(&records[p as usize]).0);
                document(a).cmp(document(b)).then(a.cmp(&b))
            });
        }
        Scoped { by_id }
    }

    /// The definition of `id` as `document` sees it.
    fn get<T>(
        &self,
        records: &[T],
        document: Name,
        id: K,
        key: impl Fn(&T) -> (Name, K),
    ) -> Option<u32> {
        let positions = self.by_id.get(&id)?;
        positions
            .iter()
            .copied()
            .find(|&p| key(&records[p as usize]).0 == document)
            .or_else(|| positions.first().copied())
    }
}

/// A delivery read for a feed: its index, and what the feed needs beyond it.
#[derive(Debug)]
pub struct Delivery {
    index: Index,
    pub(crate) gather: Gather,
    organisations: Scoped,
    lines: Scoped,
    displays: Scoped,
    patterns: Scoped,
    journeys: Scoped,
    train_numbers: Scoped,
    directions: Scoped,
    pattern_links: Scoped,
    links: Scoped,
    /// The service links with a geometry, by the stop points they lead from
    /// and to.
    links_between: Scoped<(Name, Name)>,
    /// The link sequence projections standing on their own.
    projections: Scoped,
}

/// An organisation responsible for a line, as the feed names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Agency<'a> {
    /// The NeTEx identifier of the operator or authority.
    pub id: &'a str,
    /// Its name.
    pub name: &'a str,
    /// Its web address, if it states one.
    pub url: Option<&'a str>,
    /// Its telephone number, if it states one.
    pub phone: Option<&'a str>,
    /// The document defining it.
    pub(crate) document: Name,
}

impl Delivery {
    /// Reads `source` on every thread of the current rayon pool: the index,
    /// and what a feed needs beyond it, in one walk of each document.
    pub fn read(source: &Source, reading: Reading) -> netex_rs::Result<Delivery> {
        let options = ReadOptions::new(selection(reading));
        let workers = source.read_each_with(
            || {
                let builder = if reading.referrers {
                    IndexBuilder::new().with_referrers()
                } else {
                    IndexBuilder::new()
                };
                (builder, Gather::new(reading.geometry), None)
            },
            |(builder, gather, failure): &mut (IndexBuilder, Gather, Option<netex_rs::Error>),
             entry,
             text| {
                if failure.is_some() {
                    return;
                }
                gather.begin(&entry.name);
                let read = text
                    .and_then(|xml| builder.add_streamed_with(xml, &options, &entry.name, gather));
                if let Err(error) = read {
                    *failure = Some(error.in_document(&entry.name));
                }
            },
        )?;
        let mut builders = Vec::with_capacity(workers.len());
        let mut gathers = Vec::with_capacity(workers.len());
        for (builder, gather, failure) in workers {
            if let Some(error) = failure {
                return Err(error);
            }
            builders.push(builder);
            gathers.push(gather);
        }
        let index = IndexBuilder::merge_all(builders).finish();
        gathers.sort_by_key(|gather| std::cmp::Reverse(gather.names.len()));
        let mut gathers = gathers.into_iter();
        let mut gather = gathers.next().unwrap_or_default();
        for other in gathers {
            gather.merge(other);
        }
        Ok(Delivery::of(index, gather))
    }

    pub(crate) fn of(index: Index, gather: Gather) -> Delivery {
        let names = &gather.names;
        Delivery {
            organisations: Scoped::of(names, &gather.organisations, |o| (o.document, o.id)),
            lines: Scoped::of(names, &gather.lines, |l| (l.document, l.id)),
            displays: Scoped::of(names, &gather.displays, |d| (d.document, d.id)),
            patterns: Scoped::of(names, &gather.patterns, |p| (p.document, p.id)),
            journeys: Scoped::of(names, &gather.journeys, |j| (j.document, j.id)),
            train_numbers: Scoped::of(names, &gather.train_numbers, |t| (t.document, t.id)),
            directions: Scoped::of(names, &gather.directions, |d| (d.document, d.id)),
            pattern_links: Scoped::of(names, &gather.pattern_links, |p| (p.document, p.id)),
            links: Scoped::of(names, &gather.links, |l| (l.document, l.id)),
            links_between: Scoped::of_those(
                names,
                &gather.links,
                LinkGeometry::has_geometry,
                |l| (l.document, (l.from, l.to)),
            ),
            projections: Scoped::of(names, &gather.projections, |p| (p.document, p.id)),
            index,
            gather,
        }
    }

    /// The index.
    pub fn index(&self) -> &Index {
        &self.index
    }

    pub(crate) fn name(&self, text: &str) -> Option<Name> {
        self.gather.names.get(text)
    }

    pub(crate) fn text(&self, name: Name) -> &str {
        self.gather.names.text(name)
    }

    pub(crate) fn document_facts(&self, document: Name) -> DocumentFacts {
        self.gather
            .documents
            .get(&document)
            .copied()
            .unwrap_or_default()
    }

    pub(crate) fn organisation(&self, document: Name, id: Name) -> Option<&Organisation> {
        let records = &self.gather.organisations;
        let at = self
            .organisations
            .get(records, document, id, |o| (o.document, o.id))?;
        Some(&records[at as usize])
    }

    pub(crate) fn line_facts(&self, document: Name, id: Name) -> Option<&LineFacts> {
        let records = &self.gather.lines;
        let at = self
            .lines
            .get(records, document, id, |l| (l.document, l.id))?;
        Some(&records[at as usize])
    }

    pub(crate) fn display(&self, document: Name, id: Name) -> Option<&Display> {
        let records = &self.gather.displays;
        let at = self
            .displays
            .get(records, document, id, |d| (d.document, d.id))?;
        Some(&records[at as usize])
    }

    pub(crate) fn pattern_facts(&self, document: Name, id: Name) -> Option<&PatternFacts> {
        let records = &self.gather.patterns;
        let at = self
            .patterns
            .get(records, document, id, |p| (p.document, p.id))?;
        Some(&records[at as usize])
    }

    pub(crate) fn journey_facts(&self, document: Name, id: Name) -> Option<&JourneyFacts> {
        let records = &self.gather.journeys;
        let at = self
            .journeys
            .get(records, document, id, |j| (j.document, j.id))?;
        let found = &records[at as usize];
        // A journey's own facts are its own document's only.
        (found.document == document).then_some(found)
    }

    pub(crate) fn train_number(&self, document: Name, id: Name) -> Option<&TrainNumber> {
        let records = &self.gather.train_numbers;
        let at = self
            .train_numbers
            .get(records, document, id, |t| (t.document, t.id))?;
        Some(&records[at as usize])
    }

    pub(crate) fn direction(&self, document: Name, id: Name) -> Option<Direction> {
        let records: &[DirectionFacts] = &self.gather.directions;
        let at = self
            .directions
            .get(records, document, id, |d| (d.document, d.id))?;
        Some(records[at as usize].direction)
    }

    /// The service links a journey pattern names in sequence, as `document`
    /// sees the pattern; empty where it names none.
    pub(crate) fn pattern_links(&self, document: Name, pattern: Name) -> &[Name] {
        let records: &[PatternLinks] = &self.gather.pattern_links;
        self.pattern_links
            .get(records, document, pattern, |p| (p.document, p.id))
            .map_or(&[], |at| {
                let (start, end) = records[at as usize].links;
                &self.gather.link_refs[start as usize..end as usize]
            })
    }

    /// The service link `id` as `document` sees it: its position among the
    /// gather's links.
    pub(crate) fn link(&self, document: Name, id: Name) -> Option<u32> {
        let records: &[LinkGeometry] = &self.gather.links;
        self.links
            .get(records, document, id, |l| (l.document, l.id))
    }

    /// The link sequence projection `id` as `document` sees it: its position
    /// among the gather's projections.
    pub(crate) fn projection(&self, document: Name, id: Name) -> Option<u32> {
        let records: &[ProjectionGeometry] = &self.gather.projections;
        self.projections
            .get(records, document, id, |p| (p.document, p.id))
    }

    /// A service link with a geometry from stop point `from` to `to`, as
    /// `document` sees them: its position among the gather's links.
    pub(crate) fn link_between(&self, document: Name, from: Name, to: Name) -> Option<u32> {
        let records: &[LinkGeometry] = &self.gather.links;
        self.links_between.get(records, document, (from, to), |l| {
            (l.document, (l.from, l.to))
        })
    }

    pub(crate) fn points(&self, (start, end): (u32, u32)) -> &[Point] {
        &self.gather.points[start as usize..end as usize]
    }

    /// The organisation responsible for a line, as the line's document sees
    /// it: its operator, else its first additional operator, else its
    /// authority (the order a journey planner's importer uses,
    /// `docs/INTERPRETATION.md` § 10).
    pub(crate) fn agency_of_line_in(&self, line: LineId, document: Name) -> Option<&Organisation> {
        let id = self.name(self.index.line(line).id)?;
        let facts = self.line_facts(document, id)?;
        [facts.operator, facts.additional_operator, facts.authority]
            .into_iter()
            .flatten()
            .find_map(|organisation| self.organisation(facts.document, organisation))
    }

    /// The organisation running a journey: the one its line names, else the
    /// operator the journey names itself.
    pub fn agency_of_journey(&self, journey: JourneyId) -> Option<Agency<'_>> {
        let record = self.index.journey(journey);
        let document = self.name(record.document)?;
        let of_line = record
            .line
            .and_then(|line| self.agency_of_line_in(line, document));
        let organisation = of_line.or_else(|| {
            let facts = self.journey_facts(document, self.name(record.id)?)?;
            self.organisation(document, facts.operator?)
        })?;
        Some(self.agency(organisation))
    }

    pub(crate) fn agency(&self, organisation: &Organisation) -> Agency<'_> {
        Agency {
            id: self.text(organisation.id),
            name: self.text(organisation.name),
            url: organisation.url.map(|url| self.text(url)),
            phone: organisation.phone.map(|phone| self.text(phone)),
            document: organisation.document,
        }
    }
}

#[cfg(test)]
impl Delivery {
    /// The delivery of the documents `workers` read, each worker's by a
    /// builder and a gather of its own, merged in the order given — as
    /// [`Delivery::read`] merges what its threads read, in an order the
    /// documents' distribution over the threads decides.
    pub(crate) fn read_by_workers(workers: &[&[(&str, &str)]], reading: Reading) -> Delivery {
        let options = ReadOptions::new(selection(reading));
        let mut read = workers.iter().map(|documents| {
            let mut builder = IndexBuilder::new();
            let mut gather = Gather::new(reading.geometry);
            for &(name, xml) in documents.iter() {
                gather.begin(name);
                builder
                    .add_streamed_with(xml, &options, name, &mut gather)
                    .expect("the document reads");
            }
            (builder, gather)
        });
        let (mut builder, mut gather) = read.next().expect("at least one worker");
        for (other_builder, other_gather) in read {
            builder.merge(other_builder);
            gather.merge(other_gather);
        }
        Delivery::of(builder.finish(), gather)
    }
}
