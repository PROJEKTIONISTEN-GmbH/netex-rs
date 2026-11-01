//! Who refers to what: the references of a delivery, read backwards.
//!
//! Every reference the index resolves joins two objects — the element it
//! stands in and the definition it resolved to, in the document it stands in
//! first, as every reference resolves ([`super::scope`]). Kept as a graph,
//! forward and inverse, the references answer what refers to an object across
//! the whole delivery, and which objects nothing refers to. A point a journey
//! passes through refers to its stop point as written, though it is no call.
//!
//! The graph costs memory and time and only a checker needs it, so it is
//! built only when asked for ([`IndexBuilder::with_referrers`](super::IndexBuilder::with_referrers)):
//! two adjacency lists over the references the index resolved, one forward and
//! one inverse.

use rustc_hash::FxHashMap;

use super::build::Pending;
use super::strings::{Strings, Sym};
use super::{Index, JourneyId, LineId, StopId, StopKind, StopPointId};

/// An object of a delivery as the graph of references numbers it: a
/// definition the index holds, or an element known only as one that refers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectId(pub u32);

/// An object of a delivery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Object<'a> {
    /// Its element name: `StopPlace`, `ServiceJourney`, `PassengerStopAssignment`, …
    pub element: &'static str,
    /// Its identifier.
    pub id: &'a str,
    /// The document defining it; none for what the index gathers once across
    /// documents — stop places, quays, stop points and levels.
    pub document: Option<&'a str>,
}

/// One reference: the object at its other end, and the element it is written as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reference {
    /// The object referring, for [`Referrers::of`]; the object referred to,
    /// for [`Referrers::from`].
    pub object: ObjectId,
    /// The reference's element name: `ScheduledStopPointRef`, `LineRef`, …
    pub via: &'static str,
}

/// The element names objects and references are written as, each stored as
/// its position here.
const ELEMENTS: &[&str] = &[
    "StopPlace",
    "Quay",
    "ScheduledStopPoint",
    "Level",
    "Route",
    "Line",
    "FlexibleLine",
    "ServiceJourneyPattern",
    "ServiceJourney",
    "DatedServiceJourney",
    "DayType",
    "OperatingPeriod",
    "UicOperatingPeriod",
    "AvailabilityCondition",
    "OperatingDay",
    "PassengerStopAssignment",
    "DayTypeAssignment",
    "ServiceLink",
    "SiteConnection",
    "ServiceJourneyInterchange",
    "LevelRef",
    "ScheduledStopPointRef",
    "QuayRef",
    "SiteRef",
    "StopPlaceRef",
    "RouteRef",
    "LineRef",
    "FlexibleLineRef",
    "JourneyPatternRef",
    "ServiceJourneyRef",
    "DayTypeRef",
    "AvailabilityConditionRef",
    "OperatingPeriodRef",
    "UicOperatingPeriodRef",
    "OperatingDayRef",
    "FromOperatingDayRef",
    "ToOperatingDayRef",
    "FromPointRef",
    "ToPointRef",
    "FromJourneyRef",
    "ToJourneyRef",
];

/// The position of an element name in [`ELEMENTS`].
///
/// # Panics
///
/// If the name is not there: every element the index gathers a reference in
/// or to is.
fn element_code(element: &str) -> u8 {
    let position = ELEMENTS
        .iter()
        .position(|&known| known == element)
        .unwrap_or_else(|| panic!("<{element}> is among the elements of the graph"));
    u8::try_from(position).expect("fewer than 256 element names")
}

/// The document of an object gathered once across documents.
const NO_DOCUMENT: Sym = Sym::MAX;

/// An object as the graph stores it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Node {
    element: u8,
    id: Sym,
    document: Sym,
}

/// The kinds of definition, in the order their objects are numbered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    Stop,
    StopPoint,
    Level,
    Route,
    Line,
    Pattern,
    Journey,
    DayType,
    Period,
    Condition,
    OperatingDay,
}

const KINDS: usize = 11;

/// The references as resolved, before they are laid out as a graph.
///
/// The definitions are numbered first, kind by kind in the order of [`Kind`];
/// the elements that refer are numbered as they come, by element, identifier
/// and document, and matched with the definitions they are once every
/// reference is in ([`finish`](Self::finish)).
#[derive(Debug, Default)]
pub(crate) struct Recorder {
    /// Where each kind's definitions start among the objects, and where the
    /// last ends.
    first: [u32; KINDS + 1],
    /// The kind of definition being numbered.
    kind: usize,
    nodes: Vec<Node>,
    /// The elements that refer, by element, identifier and document, each
    /// numbered once.
    referrers: Vec<(u8, Sym, Sym)>,
    referrer_of: FxHashMap<(u8, Sym, Sym), u32>,
    /// The number of the element that referred last: the references of one
    /// element come one after the other.
    last: Option<((u8, Sym, Sym), u32)>,
    /// Element codes by the address of the name, found once each.
    codes: FxHashMap<(usize, usize), u8>,
    /// Each reference: the element referring, by its number among the
    /// referrers; the element written; the object referred to.
    edges: Vec<(u32, u8, u32)>,
}

impl Recorder {
    /// Starts numbering the definitions of `kind`: those of every kind before
    /// it are in.
    pub(crate) fn begin(&mut self, kind: Kind) {
        self.kind = kind as usize;
        let next = position_of(self.nodes.len());
        for slot in &mut self.first[self.kind..] {
            *slot = next;
        }
    }

    /// Numbers the next definition of the kind begun last: its element, its
    /// identifier, and the document defining it — none for a kind gathered once.
    pub(crate) fn define(&mut self, element: &'static str, id: Sym, document: Option<Sym>) {
        let element = self.code(element);
        self.nodes.push(Node {
            element,
            id,
            document: document.unwrap_or(NO_DOCUMENT),
        });
        let next = position_of(self.nodes.len());
        for slot in &mut self.first[self.kind + 1..] {
            *slot = next;
        }
    }

    /// The code of an element name.
    fn code(&mut self, element: &'static str) -> u8 {
        *self
            .codes
            .entry((element.as_ptr() as usize, element.len()))
            .or_insert_with(|| element_code(element))
    }

    /// Records that `pending` resolved to the definition at `position` among
    /// those of `kind`.
    pub(crate) fn record(&mut self, pending: &Pending, kind: Kind, position: u32) {
        let target = self.first[kind as usize] + position;
        let via = self.code(pending.element);
        let key = (
            self.code(pending.referrer.element),
            pending.referrer.id,
            pending.origin,
        );
        let from = match self.last {
            Some((last, number)) if last == key => number,
            _ => {
                let next = position_of(self.referrers.len());
                let referrers = &mut self.referrers;
                let number = *self.referrer_of.entry(key).or_insert_with(|| {
                    referrers.push(key);
                    next
                });
                self.last = Some((key, number));
                number
            }
        };
        self.edges.push((from, via, target));
    }

    /// The graph of the references recorded. `definition` finds the
    /// definition an element that referred is — given its element name,
    /// identifier and document —, where it is one the index holds: a quay, a
    /// journey pattern, a journey; every other is an object of its own.
    pub(crate) fn finish(
        self,
        definition: impl Fn(&'static str, Sym, Sym) -> Option<(Kind, u32)>,
    ) -> Graph {
        let Recorder {
            first,
            mut nodes,
            referrers,
            mut edges,
            ..
        } = self;
        let objects: Vec<u32> = referrers
            .into_iter()
            .map(|(element, id, document)| {
                match definition(ELEMENTS[element as usize], id, document) {
                    Some((kind, position)) => first[kind as usize] + position,
                    None => {
                        nodes.push(Node {
                            element,
                            id,
                            document,
                        });
                        position_of(nodes.len() - 1)
                    }
                }
            })
            .collect();
        for edge in &mut edges {
            edge.0 = objects[edge.0 as usize];
        }
        edges.sort_unstable();
        edges.dedup();
        let count = nodes.len();
        let forward = Adjacency::of(count, edges.iter().copied());
        let mut inverse_edges: Vec<(u32, u8, u32)> = edges
            .into_iter()
            .map(|(from, via, to)| (to, via, from))
            .collect();
        inverse_edges.sort_unstable();
        let inverse = Adjacency::of(count, inverse_edges.into_iter());
        let mut by_id: Vec<u32> = (0..position_of(count)).collect();
        by_id.sort_unstable_by_key(|&object| nodes[object as usize].id);
        Graph {
            first,
            nodes,
            by_id,
            forward,
            inverse,
        }
    }
}

/// The references from each object, or to each: the offsets of each object's
/// run, and the object at the other end and the element written of each.
#[derive(Debug, Default, PartialEq)]
struct Adjacency {
    offsets: Vec<u32>,
    others: Vec<u32>,
    via: Vec<u8>,
}

impl Adjacency {
    /// The adjacency of `count` objects over `edges`, sorted by their first object.
    fn of(count: usize, edges: impl Iterator<Item = (u32, u8, u32)>) -> Adjacency {
        let mut adjacency = Adjacency {
            offsets: Vec::with_capacity(count + 1),
            ..Adjacency::default()
        };
        adjacency.offsets.push(0);
        for (object, via, other) in edges {
            while adjacency.offsets.len() <= object as usize {
                adjacency.offsets.push(position_of(adjacency.others.len()));
            }
            adjacency.others.push(other);
            adjacency.via.push(via);
        }
        while adjacency.offsets.len() <= count {
            adjacency.offsets.push(position_of(adjacency.others.len()));
        }
        adjacency
    }

    fn of_object(&self, object: u32) -> std::ops::Range<usize> {
        self.offsets[object as usize] as usize..self.offsets[object as usize + 1] as usize
    }
}

/// The graph of a delivery's references: its objects, and the references
/// from and to each.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Graph {
    first: [u32; KINDS + 1],
    nodes: Vec<Node>,
    /// Every object, by the number of its identifier among the index's strings.
    by_id: Vec<u32>,
    forward: Adjacency,
    inverse: Adjacency,
}

/// The references of an index read backwards: what refers to each object, and
/// what nothing refers to. See [`Index::referrers`].
#[derive(Debug, Clone, Copy)]
pub struct Referrers<'a> {
    index: &'a Index,
    graph: &'a Graph,
}

impl<'a> Referrers<'a> {
    pub(crate) fn new(index: &'a Index, graph: &'a Graph) -> Self {
        Referrers { index, graph }
    }

    fn strings(&self) -> &'a Strings {
        &self.index.strings
    }

    /// How many objects the graph holds: every definition of the index, and
    /// every element that refers to one.
    pub fn object_count(&self) -> usize {
        self.graph.nodes.len()
    }

    /// How many references the graph holds, each written once however often
    /// it was resolved.
    pub fn reference_count(&self) -> usize {
        self.graph.forward.others.len()
    }

    /// Every object.
    pub fn objects(&self) -> impl Iterator<Item = ObjectId> {
        (0..position_of(self.graph.nodes.len())).map(ObjectId)
    }

    /// The object.
    pub fn object(&self, object: ObjectId) -> Object<'a> {
        let node = self.graph.nodes[object.0 as usize];
        let strings = self.strings();
        Object {
            element: ELEMENTS[node.element as usize],
            id: strings.resolve(node.id),
            document: (node.document != NO_DOCUMENT).then(|| strings.resolve(node.document)),
        }
    }

    /// The objects of this element and identifier: in `document`, where one
    /// is given — a stop place, quay, stop point or level is in every
    /// document —, else in any document.
    pub fn find(
        &self,
        element: &str,
        id: &str,
        document: Option<&str>,
    ) -> impl Iterator<Item = ObjectId> + 'a {
        let strings = self.strings();
        let graph = self.graph;
        let element = ELEMENTS
            .iter()
            .position(|&known| known == element)
            .and_then(|position| u8::try_from(position).ok());
        let document = document.map(|d| strings.get(d));
        let run = match (strings.get(id), element) {
            (Some(id), Some(_)) => {
                let start = graph
                    .by_id
                    .partition_point(|&object| graph.nodes[object as usize].id < id);
                let end = graph
                    .by_id
                    .partition_point(|&object| graph.nodes[object as usize].id <= id);
                &graph.by_id[start..end]
            }
            _ => &[][..],
        };
        run.iter()
            .copied()
            .filter(move |&object| {
                let node = graph.nodes[object as usize];
                Some(node.element) == element
                    && document
                        .is_none_or(|d| node.document == NO_DOCUMENT || Some(node.document) == d)
            })
            .map(ObjectId)
    }

    /// What refers to the object, each reference once.
    pub fn of(&self, object: ObjectId) -> impl Iterator<Item = Reference> + 'a {
        references(&self.graph.inverse, object)
    }

    /// What the object refers to, each reference once.
    pub fn from(&self, object: ObjectId) -> impl Iterator<Item = Reference> + 'a {
        references(&self.graph.forward, object)
    }

    /// The object of a stop.
    pub fn stop(&self, stop: StopId) -> ObjectId {
        ObjectId(self.graph.first[Kind::Stop as usize] + stop.0)
    }

    /// The object of a stop point.
    pub fn stop_point(&self, stop_point: StopPointId) -> ObjectId {
        ObjectId(self.graph.first[Kind::StopPoint as usize] + stop_point.0)
    }

    /// The objects of a line: each definition the index took it from — one,
    /// or several that state it alike.
    pub fn line(&self, line: LineId) -> impl Iterator<Item = ObjectId> + 'a {
        let first = self.graph.first[Kind::Line as usize];
        self.index
            .line_of_position
            .iter()
            .enumerate()
            .filter(move |(_, &of)| of == line)
            .map(move |(position, _)| ObjectId(first + position_of(position)))
    }

    /// The objects of a journey: each definition the index took it from — one,
    /// or several that state it alike.
    pub fn journey(&self, journey: JourneyId) -> impl Iterator<Item = ObjectId> + 'a {
        let first = self.graph.first[Kind::Journey as usize];
        self.index
            .journey_of_position
            .iter()
            .enumerate()
            .filter(move |(_, &of)| of == Some(journey))
            .map(move |(position, _)| ObjectId(first + position_of(position)))
    }

    /// The definitions nothing refers to, in the order of their objects:
    /// stop places none of whose quays is referred to either, quays, stop
    /// points, levels, routes, lines, journey patterns, day types, operating
    /// periods, availability conditions and operating days. Journeys are not
    /// among them; nothing has to refer to a journey. A stop place naming its
    /// quay (`QuayRef`) and a quay naming its stop place (`SiteRef`) state
    /// what the stop is made of, not a use: neither counts.
    pub fn unreferenced(&self) -> impl Iterator<Item = ObjectId> + 'a {
        let graph = self.graph;
        let index = self.index;
        let stops = graph.first[Kind::Stop as usize];
        let stop_at = move |object: u32| index.stops.get(object.wrapping_sub(stops) as usize);
        // Whether `a` and `b` are a stop place and one of its quays.
        let parts_of_one = move |a: u32, b: u32| match (stop_at(a), stop_at(b)) {
            (Some(first), Some(second)) => {
                first.parent.map(|p| p.0 + stops) == Some(b)
                    || second.parent.map(|p| p.0 + stops) == Some(a)
            }
            _ => false,
        };
        let referred = move |object: u32| {
            graph
                .inverse
                .of_object(object)
                .any(|at| !parts_of_one(object, graph.inverse.others[at]))
        };
        // A stop place counts as referred to where one of its quays is.
        let mut place_used = vec![false; index.stops.len()];
        for (position, stop) in index.stops.iter().enumerate() {
            if let Some(parent) = stop.parent {
                if referred(stops + position_of(position)) {
                    place_used[parent.0 as usize] = true;
                }
            }
        }
        let journeys = graph.first[Kind::Journey as usize]..graph.first[Kind::Journey as usize + 1];
        let definitions = graph.first[KINDS];
        (0..definitions)
            .filter(move |object| !journeys.contains(object))
            .filter(move |&object| !referred(object))
            .filter(move |&object| {
                let position = object.wrapping_sub(stops) as usize;
                !(position < index.stops.len()
                    && index.stops[position].kind == StopKind::StopPlace
                    && place_used[position])
            })
            .map(ObjectId)
    }
}

fn references(adjacency: &Adjacency, object: ObjectId) -> impl Iterator<Item = Reference> + '_ {
    adjacency.of_object(object.0).map(move |at| Reference {
        object: ObjectId(adjacency.others[at]),
        via: ELEMENTS[adjacency.via[at] as usize],
    })
}

fn position_of(position: usize) -> u32 {
    u32::try_from(position).expect("fewer than 2^32 objects")
}
