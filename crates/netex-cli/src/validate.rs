//! `netex validate`: the checking report of a delivery, for the people who
//! deliver it.
//!
//! Every finding of the index ([`netex_rs::index::Concern`]) and every check
//! of the report's own — journeys without travel time, times going back,
//! stop points without a place, definitions nothing refers to — is a class
//! with its count, examples naming the document and the entity, and the
//! journeys it affects: found backwards through the references of the
//! delivery ([`netex_rs::index::Referrers`]), from the entity a finding is
//! about to the journeys that depend on it. A [`Policy`] says which classes
//! refuse the delivery, and how many of each it tolerates.

use std::collections::BTreeMap;

use netex_rs::index::{Concern, Index, JourneyId, ObjectId, Policy, Referrers, StopPointId};
use serde::Serialize;

/// The version of the report's layout, `schema/validate-report.schema.json`.
pub const REPORT_VERSION: u32 = 1;

/// How many examples a class lists at most, unless told otherwise.
pub const EXAMPLES: usize = 5;

/// The checks of the report's own, beyond the index's findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Check {
    /// A journey whose calls are all at one time: it takes no time from its
    /// first stop to its last.
    JourneyWithoutTravelTime,
    /// A journey whose passing times go back in time from one call to the next.
    TimeGoesBack,
    /// A stop point a journey calls at that no assignment places at a stop
    /// place or quay.
    StopPointWithoutPlace,
    /// A stop point a journey calls at with no position: neither its own,
    /// nor its stop's, nor that stop's stop place's.
    StopPointWithoutPosition,
    /// A definition nothing refers to: a stop place none of whose quays is
    /// used either, a stop point no journey calls at, a line, a pattern, a
    /// day type, an operating period or day, an availability condition.
    Unreferenced,
}

impl Check {
    /// Every check, in order.
    pub const ALL: &'static [Check] = &[
        Check::JourneyWithoutTravelTime,
        Check::TimeGoesBack,
        Check::StopPointWithoutPlace,
        Check::StopPointWithoutPosition,
        Check::Unreferenced,
    ];

    /// The check's name in reports and on the command line.
    pub fn code(self) -> &'static str {
        match self {
            Check::JourneyWithoutTravelTime => "journey-without-travel-time",
            Check::TimeGoesBack => "time-goes-back",
            Check::StopPointWithoutPlace => "stop-point-without-place",
            Check::StopPointWithoutPosition => "stop-point-without-position",
            Check::Unreferenced => "unreferenced",
        }
    }

    /// The check named `code`.
    pub fn from_code(code: &str) -> Option<Check> {
        Check::ALL
            .iter()
            .copied()
            .find(|check| check.code() == code)
    }

    fn says(self) -> &'static str {
        match self {
            Check::JourneyWithoutTravelTime => {
                "journey whose calls are all at one time: no travel time from its first stop to its last"
            }
            Check::TimeGoesBack => "journey whose passing times go back in time along its calls",
            Check::StopPointWithoutPlace => {
                "stop point called at that no assignment places at a stop place or quay"
            }
            Check::StopPointWithoutPosition => {
                "stop point called at without a position: neither its own nor its stop's"
            }
            Check::Unreferenced => "definition nothing refers to",
        }
    }
}

/// A class of the report: an index concern, or a check of the report's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Class {
    /// A concern of the index.
    Concern(Concern),
    /// A check of the report.
    Check(Check),
}

impl Class {
    /// The class's name: the concern's or the check's code.
    pub fn code(self) -> &'static str {
        match self {
            Class::Concern(concern) => concern.code(),
            Class::Check(check) => check.code(),
        }
    }

    /// The class named `code`.
    pub fn from_code(code: &str) -> Option<Class> {
        Concern::from_code(code)
            .map(Class::Concern)
            .or_else(|| Check::from_code(code).map(Class::Check))
    }

    /// Every class: the index's concerns, then the report's checks.
    pub fn all() -> impl Iterator<Item = Class> {
        Concern::ALL
            .iter()
            .copied()
            .map(Class::Concern)
            .chain(Check::ALL.iter().copied().map(Class::Check))
    }

    fn says(self) -> String {
        match self {
            Class::Concern(concern) => concern.to_string(),
            Class::Check(check) => check.says().to_owned(),
        }
    }

    /// Whether the class is about entities journeys depend on, whose
    /// affected journeys are found through the references; not where it is
    /// about journeys, their calls or their links themselves.
    fn follows_references(self) -> bool {
        match self {
            Class::Concern(concern) => matches!(
                concern,
                Concern::Unresolved
                    | Concern::RedefinedOtherwise
                    | Concern::PeriodWithoutFirstDay
                    | Concern::DayBitsLengthDiffers
                    | Concern::ResolvedIgnoringVersion
                    | Concern::ResolvedAsOtherClass
                    | Concern::UnknownReferenceSystem
            ),
            Class::Check(check) => matches!(
                check,
                Check::StopPointWithoutPlace | Check::StopPointWithoutPosition
            ),
        }
    }
}

/// What the report tolerates of each class; the rest in any number.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Limits {
    limits: BTreeMap<Class, u64>,
}

impl Limits {
    /// Tolerates at most `limit` findings of `class`.
    pub fn at_most(mut self, class: Class, limit: u64) -> Limits {
        self.limits.insert(class, limit);
        self
    }

    /// Refuses every finding of every class.
    pub fn strict() -> Limits {
        Class::all().fold(Limits::default(), |limits, class| limits.at_most(class, 0))
    }

    /// The limit of the index's concerns as a policy of netex-rs.
    fn index_policy(&self) -> Policy {
        self.limits
            .iter()
            .fold(Policy::tolerant(), |policy, (&class, &limit)| match class {
                Class::Concern(concern) => policy.at_most(concern, limit),
                Class::Check(_) => policy,
            })
    }
}

/// One entity a finding is about, with the journeys it affects.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Example {
    /// The document it stands in; none for what the index gathers once.
    pub document: Option<String>,
    /// Its element name.
    pub element: String,
    /// Its identifier.
    pub entity: String,
    /// What else the finding names.
    pub about: Option<String>,
    /// How many journeys depend on it.
    pub affected_journeys: u64,
    /// The first of those journeys, by identifier.
    pub affected_examples: Vec<String>,
}

/// One class of the report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClassReport {
    /// The class's name.
    pub class: String,
    /// `index` for a concern of the index, `check` for one of the report's own.
    pub kind: &'static str,
    /// What a finding of the class says.
    pub says: String,
    /// How many findings.
    pub count: u64,
    /// How many distinct journeys depend on the entities found, where the
    /// references were followed back to them; none where the class is about
    /// journeys themselves.
    pub affected_journeys: Option<u64>,
    /// The first findings, in the order the index lists them.
    pub examples: Vec<Example>,
    /// Up to how many the policy tolerates; none where it tolerates any number.
    pub limit: Option<u64>,
    /// Whether the count exceeds the limit.
    pub refused: bool,
}

/// What the delivery holds, counted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Counts {
    /// Documents read.
    pub documents: u32,
    /// Stop places and quays.
    pub stops: usize,
    /// Scheduled stop points.
    pub stop_points: usize,
    /// Lines.
    pub lines: usize,
    /// Journeys the index keeps.
    pub journeys: usize,
    /// Passing times of those journeys.
    pub passing_times: usize,
    /// Distinct calendars they run on.
    pub calendars: usize,
    /// Walks between stops.
    pub transfers: usize,
    /// Planned interchanges.
    pub interchanges: usize,
    /// Objects of the reference graph: definitions and elements that refer.
    pub objects: usize,
    /// References resolved, each counted once.
    pub references: usize,
}

/// The checking report.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    /// The version of the report's layout.
    pub report_version: u32,
    /// The source read: its file or directory name.
    pub source: String,
    /// What the delivery holds.
    pub counts: Counts,
    /// Whether the policy refuses the delivery.
    pub refused: bool,
    /// Every class with findings, and every class the policy limits.
    pub classes: Vec<ClassReport>,
}

impl Report {
    /// The report in a few lines for a person: the counts, each class with
    /// findings and its first examples, and the verdict.
    pub fn human(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        let c = &self.counts;
        let _ = writeln!(
            out,
            "{}: {} documents, {} stops, {} stop points, {} lines, {} journeys, {} calendars, {} references",
            self.source, c.documents, c.stops, c.stop_points, c.lines, c.journeys, c.calendars, c.references
        );
        let journeys = |count: u64| match count {
            1 => "1 journey".to_owned(),
            count => format!("{count} journeys"),
        };
        for class in &self.classes {
            let affected = class
                .affected_journeys
                .map_or(String::new(), |n| format!(", {} affected", journeys(n)));
            let limit = class
                .limit
                .map_or(String::new(), |l| format!(" (at most {l} tolerated)"));
            let mark = if class.refused { "REFUSED " } else { "" };
            let _ = writeln!(
                out,
                "{mark}{} × {} — {}{affected}{limit}",
                class.count, class.class, class.says
            );
            for example in &class.examples {
                let document = example.document.as_deref().unwrap_or("the delivery");
                let about = example
                    .about
                    .as_ref()
                    .map_or(String::new(), |a| format!(" ({a})"));
                let _ = writeln!(
                    out,
                    "    {} {} in {document}{about}{}",
                    example.element,
                    example.entity,
                    if example.affected_journeys > 0 {
                        format!(" — {}", journeys(example.affected_journeys))
                    } else {
                        String::new()
                    }
                );
            }
        }
        let _ = writeln!(out, "{}", if self.refused { "refused" } else { "accepted" });
        out
    }
}

/// A finding before it is reported: its document (none for what is gathered
/// once), element, entity and what else it names; and the object of the
/// reference graph to follow back, where there is one.
struct Found {
    document: Option<String>,
    element: String,
    entity: String,
    about: Option<String>,
    start: Option<ObjectId>,
}

/// Checks the delivery whose index is `index`, built with its references
/// kept ([`netex_rs::index::IndexBuilder::with_referrers`]): every class, its
/// examples — `examples` at most — and the journeys affected, under `limits`.
///
/// # Panics
///
/// If the index was built without its references.
pub fn check(index: &Index, source: &str, limits: &Limits, examples: usize) -> Report {
    let referrers = index
        .referrers()
        .expect("the index is built with its references");
    let affected = Affected::new(index, &referrers);
    let mut classes = Vec::new();
    for class in Class::all() {
        let found = find(index, &referrers, class);
        let limit = limits.limits.get(&class).copied();
        let count = match class {
            Class::Concern(concern) => index.finding_count(concern),
            Class::Check(_) => found.len() as u64,
        };
        if count == 0 && limit.is_none() {
            continue;
        }
        let follows = class.follows_references();
        let affected_journeys =
            follows.then(|| affected.count(found.iter().filter_map(|f| f.start)) as u64);
        let examples = found
            .into_iter()
            .take(examples)
            .map(|found| {
                let journeys = match (follows, found.start) {
                    (true, Some(start)) => affected.journeys(std::iter::once(start)),
                    _ => Vec::new(),
                };
                Example {
                    document: found.document,
                    element: found.element,
                    entity: found.entity,
                    about: found.about,
                    affected_journeys: journeys.len() as u64,
                    affected_examples: journeys
                        .iter()
                        .take(EXAMPLES)
                        .map(|&journey| index.journey(journey).id.to_owned())
                        .collect(),
                }
            })
            .collect();
        classes.push(ClassReport {
            class: class.code().to_owned(),
            kind: match class {
                Class::Concern(_) => "index",
                Class::Check(_) => "check",
            },
            says: class.says(),
            count,
            affected_journeys,
            examples,
            limit,
            refused: limit.is_some_and(|limit| count > limit),
        });
    }
    let refused =
        classes.iter().any(|class| class.refused) || limits.index_policy().check(index).is_err();
    Report {
        report_version: REPORT_VERSION,
        source: source.to_owned(),
        counts: Counts {
            documents: index.document_count(),
            stops: index.stop_count(),
            stop_points: index.stop_point_count(),
            lines: index.line_count(),
            journeys: index.journey_count(),
            passing_times: index.passing_time_count(),
            calendars: index.calendar_count(),
            transfers: index.transfer_count(),
            interchanges: index.interchange_count(),
            objects: referrers.object_count(),
            references: referrers.reference_count(),
        },
        refused,
        classes,
    }
}

/// The findings of `class`, in the index's order.
fn find(index: &Index, referrers: &Referrers<'_>, class: Class) -> Vec<Found> {
    let object = |element: &str, id: &str, document: Option<&str>| {
        referrers.find(element, id, document).next()
    };
    match class {
        Class::Concern(Concern::Unresolved) => index
            .unresolved()
            .iter()
            .map(|unresolved| {
                // The referrer is written `element identifier`.
                let (element, id) = unresolved
                    .referrer
                    .split_once(' ')
                    .unwrap_or((unresolved.referrer.as_str(), ""));
                Found {
                    document: Some(unresolved.origin.clone()),
                    element: element.to_owned(),
                    entity: id.to_owned(),
                    about: Some(format!("<{}> {}", unresolved.element, unresolved.id)),
                    start: object(element, id, Some(&unresolved.origin)),
                }
            })
            .collect(),
        Class::Concern(concern) => index
            .findings()
            .filter(|finding| finding.concern == concern)
            .map(|finding| {
                // A finding about how a reference resolved names the
                // reference's element and the identifier it refers to.
                let target =
                    match concern {
                        Concern::ResolvedAsOtherClass => finding.about,
                        Concern::ResolvedIgnoringVersion => finding
                            .element
                            .strip_suffix("Ref")
                            .map(|element| match element {
                                "JourneyPattern" => "ServiceJourneyPattern",
                                other => other,
                            }),
                        _ => Some(finding.element),
                    };
                Found {
                    document: Some(finding.document.to_owned()),
                    element: finding.element.to_owned(),
                    entity: finding.entity.to_owned(),
                    about: finding.about.map(str::to_owned),
                    start: target.and_then(|element| {
                        object(element, finding.entity, Some(finding.document))
                    }),
                }
            })
            .collect(),
        Class::Check(Check::JourneyWithoutTravelTime) => journeys_where(index, |times| {
            let mut stated = times
                .iter()
                .flat_map(|time| [time.arrival, time.departure])
                .flatten();
            let first = stated.next();
            first.is_some() && stated.all(|time| Some(time) == first)
        }),
        Class::Check(Check::TimeGoesBack) => journeys_where(index, |times| {
            let stated: Vec<u32> = times
                .iter()
                .flat_map(|time| [time.arrival, time.departure])
                .flatten()
                .collect();
            stated.windows(2).any(|pair| pair[1] < pair[0])
        }),
        Class::Check(Check::StopPointWithoutPlace) => called_stop_points(index)
            .into_iter()
            .filter(|&point| index.stop_of_stop_point(point).is_none())
            .map(|point| stop_point_found(index, referrers, point))
            .collect(),
        Class::Check(Check::StopPointWithoutPosition) => called_stop_points(index)
            .into_iter()
            .filter(|&point| position_of(index, point).is_none())
            .map(|point| stop_point_found(index, referrers, point))
            .collect(),
        Class::Check(Check::Unreferenced) => referrers
            .unreferenced()
            .map(|id| {
                let object = referrers.object(id);
                Found {
                    document: object.document.map(str::to_owned),
                    element: object.element.to_owned(),
                    entity: object.id.to_owned(),
                    about: None,
                    start: None,
                }
            })
            .collect(),
    }
}

/// The journeys whose passing times `matches`, as findings about themselves.
fn journeys_where(
    index: &Index,
    matches: impl Fn(&[netex_rs::index::PassingTime]) -> bool,
) -> Vec<Found> {
    (0..index.journey_count() as u32)
        .map(JourneyId)
        .filter(|&journey| matches(index.journey(journey).passing_times))
        .map(|journey| {
            let record = index.journey(journey);
            Found {
                document: Some(record.document.to_owned()),
                element: "ServiceJourney".to_owned(),
                entity: record.id.to_owned(),
                about: record.line.map(|line| index.line(line).id.to_owned()),
                start: None,
            }
        })
        .collect()
}

/// The stop points journeys call at, each once, in the order of the index.
fn called_stop_points(index: &Index) -> Vec<StopPointId> {
    let mut called = vec![false; index.stop_point_count()];
    for journey in (0..index.journey_count() as u32).map(JourneyId) {
        for point in index.journey(journey).stop_points {
            called[point.0 as usize] = true;
        }
    }
    (0..index.stop_point_count() as u32)
        .map(StopPointId)
        .filter(|point| called[point.0 as usize])
        .collect()
}

fn stop_point_found(index: &Index, referrers: &Referrers<'_>, point: StopPointId) -> Found {
    Found {
        document: None,
        element: "ScheduledStopPoint".to_owned(),
        entity: index.stop_point(point).id.to_owned(),
        about: None,
        start: Some(referrers.stop_point(point)),
    }
}

/// Where a stop point is: its own position, else its stop's, else that
/// stop's stop place's.
fn position_of(index: &Index, point: StopPointId) -> Option<(f64, f64)> {
    let record = index.stop_point(point);
    record.latitude.zip(record.longitude).or_else(|| {
        let stop = index.stop(record.stop?);
        stop.latitude.zip(stop.longitude).or_else(|| {
            let place = index.stop(stop.parent?);
            place.latitude.zip(place.longitude)
        })
    })
}

/// The journeys depending on objects of the delivery: what refers to them,
/// and on, to the journeys — through an assignment to what it assigns, as a
/// stop point depends on the assignment placing it.
struct Affected<'a> {
    referrers: &'a Referrers<'a>,
    /// The journey each object is, where it is one the index keeps.
    journey_of: Vec<Option<JourneyId>>,
}

impl<'a> Affected<'a> {
    fn new(index: &Index, referrers: &'a Referrers<'a>) -> Affected<'a> {
        let mut journey_of = vec![None; referrers.object_count()];
        for journey in (0..index.journey_count() as u32).map(JourneyId) {
            for object in referrers.journey(journey) {
                journey_of[object.0 as usize] = Some(journey);
            }
        }
        Affected {
            referrers,
            journey_of,
        }
    }

    /// The journeys depending on any of `starts`, each once, in the order of the index.
    fn journeys(&self, starts: impl Iterator<Item = ObjectId>) -> Vec<JourneyId> {
        let mut seen = vec![false; self.referrers.object_count()];
        let mut queue: Vec<ObjectId> = Vec::new();
        let mut visit = |object: ObjectId, queue: &mut Vec<ObjectId>| {
            if !std::mem::replace(&mut seen[object.0 as usize], true) {
                queue.push(object);
            }
        };
        for start in starts {
            visit(start, &mut queue);
        }
        let mut journeys = Vec::new();
        while let Some(object) = queue.pop() {
            if let Some(journey) = self.journey_of[object.0 as usize] {
                journeys.push(journey);
            }
            let element = self.referrers.object(object).element;
            if matches!(element, "PassengerStopAssignment" | "DayTypeAssignment") {
                for reference in self.referrers.from(object) {
                    visit(reference.object, &mut queue);
                }
            }
            for reference in self.referrers.of(object) {
                visit(reference.object, &mut queue);
            }
        }
        journeys.sort_unstable();
        journeys.dedup();
        journeys
    }

    fn count(&self, starts: impl Iterator<Item = ObjectId>) -> usize {
        self.journeys(starts).len()
    }
}
