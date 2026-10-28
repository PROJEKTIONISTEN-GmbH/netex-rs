//! `netex extract`: a closed part of a delivery — the journeys chosen by
//! operator, line, area and period, and everything they refer to —, written
//! document by document as the source wrote it: the same release, the same
//! bytes, the objects not taken left out.
//!
//! The journeys are chosen on the index. What else is taken is found on the
//! documents as written ([`Outline`]): what a kept object refers to, resolved
//! as netex-rs resolves it — in the object's own document first, else in the
//! document whose name sorts first among those defining it —, and the
//! assignments giving a kept object its meaning: the stop assignment of a kept
//! stop point, the day type assignment of a kept day type, the notice
//! assignment of a kept object — those standing in the kept object's own
//! document. An interchange or connection is kept where everything it joins is.

use std::collections::hash_map::Entry;
use std::fs::File;
use std::io::{BufWriter, Write as _};
use std::path::{Path, PathBuf};

use netex2gtfs::Delivery;
use netex_rs::dataset::Source;
use netex_rs::findings::{self, Error, Finding, Findings};
use netex_rs::index::{Index, JourneyId, StopPointId};
use netex_rs::model::Date;
use netex_rs::written::edit::Edits;
use netex_rs::written::outline::{is_furniture, is_journey, subject_of, Called, Object, Outline};
use rustc_hash::{FxHashMap, FxHashSet};
use serde::Serialize;

pub use crate::area::Area;

/// What becomes of a journey that leaves the area.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Leaving {
    /// It is kept whole, every call outside the area too.
    #[default]
    Whole,
    /// Its calls before its first and after its last in the area are left
    /// out; one calling there only once is left out.
    Cut,
}

/// Which journeys a part holds: those meeting every criterion given.
#[derive(Debug, Clone, Default)]
pub struct Selection {
    /// Run by one of these operators — the line's operator, else its first
    /// additional operator, else its authority, else the journey's operator.
    pub operators: Vec<String>,
    /// Of one of these lines.
    pub lines: Vec<String>,
    /// Calling in this area.
    pub area: Option<Area>,
    /// Running on a day of this period, first and last day; the calendar is
    /// kept whole.
    pub period: Option<(Date, Date)>,
    /// What becomes of a journey leaving the area.
    pub leaving: Leaving,
}

/// What a part holds.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct Counts {
    /// The documents of the source.
    pub documents_read: u64,
    /// The documents written: those holding something taken.
    pub documents_written: u64,
    /// The journeys chosen.
    pub journeys_selected: u64,
    /// The journeys written: those chosen and those a kept object refers to.
    pub journeys_written: u64,
    /// The objects written.
    pub objects_written: u64,
}

/// The report of a part.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Report {
    /// The source read.
    pub source: String,
    /// Where the part is written.
    pub to: String,
    /// What became of journeys leaving the area: `whole` or `cut`.
    pub leaving: &'static str,
    /// What the part holds.
    pub counts: Counts,
    /// What was found, by class.
    pub findings: Vec<Finding>,
}

impl Report {
    /// The report for a person.
    pub fn human(&self) -> String {
        let counts = &self.counts;
        format!(
            "{} → {}: {} of {} documents, {} journeys chosen, {} written, {} objects\n{}",
            self.source,
            self.to,
            counts.documents_written,
            counts.documents_read,
            counts.journeys_selected,
            counts.journeys_written,
            counts.objects_written,
            findings::human(&self.findings)
        )
    }
}

/// The objects taken from each document, by number.
type Taken = FxHashMap<String, FxHashSet<u32>>;

/// The identifiers referred to across documents, each with the document
/// answering it once found.
type Wanted = FxHashMap<String, Option<String>>;

/// Writes the part of `delivery`, read from `source`, that `selection`
/// chooses into the directory `to`, each document under its name.
pub fn extract(
    source: &Source,
    delivery: &Delivery,
    selection: &Selection,
    to: &Path,
) -> Result<Report, Error> {
    let mut findings = Findings::default();
    let (chosen, journeys_selected) = choose(delivery, selection, &mut findings);
    let index = delivery.index();
    let inside = |point: &str| {
        selection.area.as_ref().is_none_or(|area| {
            index
                .stop_point_by_id(point)
                .is_some_and(|point| is_in(index, area, point))
        })
    };

    // Take what the chosen journeys refer to, document by document, until
    // nothing more is taken.
    let mut taken = Taken::default();
    let mut wanted = Wanted::default();
    let documents_read = loop {
        let answers: Vec<Result<Answer, Error>> = source
            .read_each_with(Vec::new, |answers, entry, text| {
                answers.push(
                    text.map_err(Error::Read)
                        .and_then(|text| take(&entry.name, text, &chosen, &taken, &wanted)),
                );
            })?
            .into_iter()
            .flatten()
            .collect();
        let documents = answers.len() as u64;
        let mut changed = false;
        let mut definers: FxHashMap<String, String> = FxHashMap::default();
        for answer in answers {
            let answer = answer?;
            for id in answer.external {
                if let Entry::Vacant(slot) = wanted.entry(id) {
                    slot.insert(None);
                    changed = true;
                }
            }
            for id in answer.holds {
                let first = definers
                    .entry(id)
                    .or_insert_with(|| answer.document.clone());
                if answer.document < *first {
                    first.clone_from(&answer.document);
                }
            }
            let known = taken.entry(answer.document).or_default();
            let before = known.len();
            known.extend(answer.kept);
            changed |= known.len() != before;
        }
        for (id, document) in definers {
            let answer = wanted
                .get_mut(&id)
                .expect("only wanted identifiers are looked for");
            if answer.is_none() {
                *answer = Some(document);
                changed = true;
            }
        }
        if !changed {
            break documents;
        }
    };
    for (id, answer) in &wanted {
        if answer.is_none() {
            findings.note(
                "reference-to-nothing",
                "a reference of an object taken to an identifier no document defines: written as it was",
                id.clone(),
            );
        }
    }
    taken.retain(|_, objects| !objects.is_empty());

    // Write the documents holding something taken.
    let cut = selection.area.is_some() && selection.leaving == Leaving::Cut;
    let written: Vec<Result<Written, Error>> = source
        .read_each_with(Vec::new, |written, entry, text| {
            let Some(kept) = taken.get(&entry.name) else {
                return;
            };
            written.push(text.map_err(Error::Read).and_then(|text| {
                let chosen = chosen.get(&entry.name);
                write(
                    &entry.name,
                    text,
                    kept,
                    chosen,
                    cut.then_some(&inside as &dyn Fn(&str) -> bool),
                    to,
                )
            }));
        })?
        .into_iter()
        .flatten()
        .collect();
    let mut counts = Counts {
        documents_read,
        journeys_selected,
        ..Counts::default()
    };
    for document in written {
        let document = document?;
        counts.documents_written += 1;
        counts.journeys_written += document.journeys;
        counts.objects_written += document.objects;
        findings.absorb(document.findings);
    }
    Ok(Report {
        source: source.path().display().to_string(),
        to: to.display().to_string(),
        leaving: match selection.leaving {
            Leaving::Whole => "whole",
            Leaving::Cut => "cut",
        },
        counts,
        findings: findings.into_list(),
    })
}

/// The journeys `selection` chooses, by document, and how many.
fn choose(
    delivery: &Delivery,
    selection: &Selection,
    findings: &mut Findings,
) -> (FxHashMap<String, FxHashSet<String>>, u64) {
    let index = delivery.index();
    let period = selection.period.as_ref().map(|(first, last)| {
        (
            first.days_since_epoch().unwrap_or(i64::MIN),
            last.days_since_epoch().unwrap_or(i64::MAX),
        )
    });
    let mut chosen: FxHashMap<String, FxHashSet<String>> = FxHashMap::default();
    let mut count = 0;
    for journey in (0..index.journey_count() as u32).map(JourneyId) {
        let record = index.journey(journey);
        if !selection.operators.is_empty()
            && !delivery
                .agency_of_journey(journey)
                .is_some_and(|agency| selection.operators.iter().any(|id| id == agency.id))
        {
            continue;
        }
        if !selection.lines.is_empty()
            && !record.line.is_some_and(|line| {
                let id = index.line(line).id;
                selection.lines.iter().any(|wanted| wanted == id)
            })
        {
            continue;
        }
        if let Some((first, last)) = period {
            if !index
                .days_of(journey)
                .any(|day| first <= day && day <= last)
            {
                continue;
            }
        }
        if let Some(area) = &selection.area {
            let inside = record
                .stop_points
                .iter()
                .filter(|&&point| is_in(index, area, point))
                .count();
            let leaves = inside < record.stop_points.len();
            match (selection.leaving, inside) {
                (_, 0) => continue,
                (Leaving::Cut, 1) => {
                    findings.note(
                        "journey-touches-area",
                        "a journey calling in the area only once: left out",
                        record.id,
                    );
                    continue;
                }
                (Leaving::Whole, _) if leaves => findings.note(
                    "journey-leaves-area",
                    "a journey calling outside the area too: kept whole",
                    record.id,
                ),
                _ => {}
            }
        }
        chosen
            .entry(record.document.to_owned())
            .or_default()
            .insert(record.id.to_owned());
        count += 1;
    }
    (chosen, count)
}

/// Whether the stop point is in the area: its stop's position, else its own.
fn is_in(index: &Index, area: &Area, point: StopPointId) -> bool {
    let of_stop = index.stop_of_stop_point(point).and_then(|stop| {
        let stop = index.stop(stop);
        stop.longitude.zip(stop.latitude)
    });
    let record = index.stop_point(point);
    of_stop
        .or(record.longitude.zip(record.latitude))
        .is_some_and(|(longitude, latitude)| area.contains(longitude, latitude))
}

/// What a pass found in one document.
struct Answer {
    document: String,
    /// The objects taken, by number.
    kept: Vec<u32>,
    /// The identifiers referred to that the document does not define.
    external: Vec<String>,
    /// The identifiers looked for that the document defines.
    holds: Vec<String>,
}

/// One pass over a document: the objects taken in it so far, the chosen
/// journeys, the objects answering identifiers wanted from elsewhere, and all
/// they refer to within the document.
fn take(
    document: &str,
    text: &str,
    chosen: &FxHashMap<String, FxHashSet<String>>,
    taken: &Taken,
    wanted: &Wanted,
) -> Result<Answer, Error> {
    let outline = Outline::of(text).map_err(|error| Error::Document {
        document: document.to_owned(),
        message: error.to_string(),
    })?;
    let mut kept: FxHashSet<u32> = taken.get(document).cloned().unwrap_or_default();
    if let Some(journeys) = chosen.get(document) {
        for (number, object) in outline.objects.iter().enumerate() {
            if is_journey(object.element) && journeys.contains(object.id) {
                kept.insert(number as u32);
            }
        }
    }
    let mut holds = Vec::new();
    for id in outline.identifiers() {
        match wanted.get(id) {
            Some(Some(answering)) if answering == document => {
                kept.extend(outline.holders(id));
            }
            Some(None) => holds.push(id.to_owned()),
            _ => {}
        }
    }
    let mut external = FxHashSet::default();
    close(&outline, &mut kept, &mut external, wanted);
    Ok(Answer {
        document: document.to_owned(),
        kept: kept.into_iter().collect(),
        external: external.into_iter().map(str::to_owned).collect(),
        holds,
    })
}

/// Takes what the objects in `kept` refer to within the document, and the
/// assignments, interchanges and connections that belong with them; the
/// identifiers referred to that the document does not define go to
/// `external`.
fn close<'x>(
    outline: &Outline<'x>,
    kept: &mut FxHashSet<u32>,
    external: &mut FxHashSet<&'x str>,
    wanted: &Wanted,
) {
    let mut queue: Vec<u32> = kept.iter().copied().collect();
    let mut header = false;
    loop {
        while let Some(number) = queue.pop() {
            for reference in &outline.objects[number as usize].references {
                follow(outline, reference.id, kept, &mut queue, external);
            }
        }
        if !header
            && kept
                .iter()
                .any(|&number| !is_furniture(outline.objects[number as usize].element))
        {
            header = true;
            for reference in &outline.header {
                follow(outline, reference.id, kept, &mut queue, external);
            }
        }
        for (number, object) in outline.objects.iter().enumerate() {
            let number = number as u32;
            if !kept.contains(&number) && belongs(outline, object, kept, wanted) {
                kept.insert(number);
                queue.push(number);
            }
        }
        if queue.is_empty() {
            break;
        }
    }
}

fn follow<'x>(
    outline: &Outline<'x>,
    id: &'x str,
    kept: &mut FxHashSet<u32>,
    queue: &mut Vec<u32>,
    external: &mut FxHashSet<&'x str>,
) {
    let holders = outline.holders(id);
    if holders.is_empty() {
        external.insert(id);
    }
    for &holder in holders {
        if kept.insert(holder) {
            queue.push(holder);
        }
    }
}

/// Whether an object not taken belongs with those taken: an assignment of a
/// taken object, an interchange or connection between taken objects.
fn belongs(outline: &Outline, object: &Object, kept: &FxHashSet<u32>, wanted: &Wanted) -> bool {
    let is_kept = |id: &str| {
        outline
            .holders(id)
            .iter()
            .any(|holder| kept.contains(holder))
    };
    if let Some(subject) = subject_of(object.element) {
        return object
            .references
            .iter()
            .any(|reference| reference.element == subject && is_kept(reference.id));
    }
    let joins = ["Assignment", "Interchange", "Connection"]
        .iter()
        .any(|kind| object.element.ends_with(kind));
    joins
        && object.references.iter().any(|r| is_kept(r.id))
        && object.references.iter().all(|reference| {
            if outline.holders(reference.id).is_empty() {
                matches!(wanted.get(reference.id), Some(Some(_)))
            } else {
                is_kept(reference.id)
            }
        })
}

/// What writing one document wrote.
struct Written {
    journeys: u64,
    objects: u64,
    findings: Findings,
}

/// Writes the document `name` with the objects `kept`, the chosen journeys
/// cut to the area where `inside` is given.
fn write(
    name: &str,
    text: &str,
    kept: &FxHashSet<u32>,
    chosen: Option<&FxHashSet<String>>,
    inside: Option<&dyn Fn(&str) -> bool>,
    to: &Path,
) -> Result<Written, Error> {
    let outline = Outline::of(text).map_err(|error| Error::Document {
        document: name.to_owned(),
        message: error.to_string(),
    })?;
    let mut findings = Findings::default();
    let mut edits = Edits::default();
    let mut filled = vec![false; outline.containers.len()];
    let points: FxHashMap<&str, &str> = outline
        .objects
        .iter()
        .flat_map(|object| object.points.iter().copied())
        .collect();
    let (mut journeys, mut objects) = (0, 0);
    for (number, object) in outline.objects.iter().enumerate() {
        if !kept.contains(&(number as u32)) && !is_furniture(object.element) {
            if object.element.ends_with("Interchange") {
                findings.note(
                    "interchange-left-out",
                    "an interchange with a journey not taken: left out",
                    object.id,
                );
            }
            edits.remove(object.range.clone());
            continue;
        }
        filled[object.container as usize] = true;
        objects += 1;
        if !is_journey(object.element) {
            continue;
        }
        journeys += 1;
        let is_chosen = chosen.is_some_and(|chosen| chosen.contains(object.id));
        if !is_chosen {
            findings.note(
                "journey-referenced",
                "a journey not chosen that an object taken refers to: kept",
                object.id,
            );
        } else if let Some(inside) = inside {
            cut(object, &points, inside, &mut edits, &mut findings);
        }
    }
    for (container, filled) in outline.containers.iter().zip(filled) {
        if !filled && !container.holds_others {
            edits.remove(container.range.clone());
        }
    }
    let path: PathBuf = to.join(name);
    let failed = |error| Error::Write {
        path: path.clone(),
        error,
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(failed)?;
    }
    let mut out = BufWriter::new(File::create(&path).map_err(failed)?);
    edits.write(text, &mut out).map_err(failed)?;
    out.flush().map_err(failed)?;
    Ok(Written {
        journeys,
        objects,
        findings,
    })
}

/// Leaves out the calls of a journey before its first and after its last in
/// the area.
fn cut(
    journey: &Object,
    points: &FxHashMap<&str, &str>,
    inside: &dyn Fn(&str) -> bool,
    edits: &mut Edits,
    findings: &mut Findings,
) {
    let calls: Option<Vec<bool>> = journey
        .calls
        .iter()
        .map(|call| match call.at? {
            Called::StopPoint(point) => Some(inside(point)),
            Called::PatternPoint(point) => points.get(point).map(|point| inside(point)),
        })
        .collect();
    let Some(calls) = calls else {
        findings.note(
            "journey-not-cut",
            "a journey whose calls are not all placed within its document: kept whole",
            journey.id,
        );
        return;
    };
    let (Some(first), Some(last)) = (
        calls.iter().position(|&inside| inside),
        calls.iter().rposition(|&inside| inside),
    ) else {
        return;
    };
    if first > 0 || last + 1 < calls.len() {
        findings.note(
            "journey-cut",
            "a journey leaving the area: its calls before the first and after the last in it left out",
            journey.id,
        );
        for (number, call) in journey.calls.iter().enumerate() {
            if number < first || number > last {
                edits.remove(call.range.clone());
            }
        }
    }
    if calls[first..=last].contains(&false) {
        findings.note(
            "journey-leaves-and-returns",
            "a journey leaving the area and coming back: its calls outside in between kept",
            journey.id,
        );
    }
}
