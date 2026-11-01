//! `netex merge`: several deliveries into one, each written under its rank
//! and label (`1-oebb/…`, `2-vor/…`) so that netex-rs, resolving a reference
//! in the document of the delivery whose name sorts first, answers it as the
//! deliveries are ranked.
//!
//! Not the first delivery wins, but the rules a journey planner learned for
//! several timetables (`docs/INTERPRETATION.md` § 16):
//!
//! - **Stops pair by identifier, else by global identifier** (`GlobalID` —
//!   the DHID —, `SLOID`, the Dutch stop register's `QuayCode` and
//!   `StopPlaceCode`): a stop place of a later delivery whose every quay
//!   pairs with a quay of the stop place of an earlier one is that stop
//!   place; its references point there. What differs — the name, the
//!   position by more than 50 m — is a finding naming both. A stop place
//!   with a quay pairing with none is kept apart, under another identifier
//!   where its own is taken.
//! - **An object stated alike is kept once**: an object of a later delivery
//!   whose identifier an earlier one answers, written alike and referring to
//!   objects alike, is left out; its references find the earlier one.
//! - **An identifier taken with other content is renamed** into the
//!   delivery's codespace, `<label>:<identifier>`, with the references
//!   finding it.
//! - **A journey stated in two deliveries is kept once**: a journey of a later
//!   delivery calling at the same stops at the same times as one of an earlier
//!   one, on days the earlier one runs on too, is left out; one running on
//!   other days is kept and found.
//!
//! A place other than a stop place — a park and ride — under an earlier
//! delivery's identifier and global identifier is that place, stated
//! otherwise: a finding, the earlier one kept. An object renamed that only
//! objects left out needed is left out as well — not a place, which stands for
//! itself —, and so are the assignments of what is left out.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufWriter, Write as _};
use std::path::{Path, PathBuf};

use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::dataset::Source;
use crate::findings::{self, Error, Finding, Findings};
use crate::index::{Index, JourneyId};
use crate::stream::{ReadOptions, Selection};
use crate::written::edit::{attribute_value, Edits};
use crate::written::outline::{is_furniture, is_journey, is_place, subject_of, Object, Outline};

/// A delivery to merge, with the label naming it.
#[derive(Debug, Clone)]
pub struct Input {
    label: String,
    source: Source,
}

impl Input {
    /// A delivery labelled `label`: letters, digits, `-` and `_`.
    pub fn new(label: &str, source: Source) -> Result<Input, String> {
        let fits = |c: char| c.is_ascii_alphanumeric() || c == '-' || c == '_';
        if label.is_empty() || !label.chars().all(fits) {
            return Err(format!(
                "`{label}` is no label: letters, digits, `-` and `_`"
            ));
        }
        Ok(Input {
            label: label.to_owned(),
            source,
        })
    }

    /// The label.
    pub fn label(&self) -> &str {
        &self.label
    }
}

/// What became of one delivery.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct DeliveryReport {
    /// Its label.
    pub label: String,
    /// The source read.
    pub source: String,
    /// Its documents.
    pub documents_read: u64,
    /// Its documents written: those keeping an object.
    pub documents_written: u64,
    /// Its objects.
    pub objects: u64,
    /// Its objects left out: alike, paired, stated in an earlier delivery or
    /// no longer needed.
    pub objects_left_out: u64,
    /// Its objects renamed.
    pub objects_renamed: u64,
    /// Its journeys left out.
    pub journeys_left_out: u64,
    /// Its stop places paired with an earlier delivery's, each counted once
    /// however many documents state it.
    pub stop_places_paired: u64,
}

/// The report of a merge.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Report {
    /// Where the merge is written.
    pub to: String,
    /// Each delivery, in rank.
    pub deliveries: Vec<DeliveryReport>,
    /// What was found, by class.
    pub findings: Vec<Finding>,
}

impl Report {
    /// The report for a person.
    pub fn human(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        for (rank, delivery) in self.deliveries.iter().enumerate() {
            let _ = writeln!(
                out,
                "{}-{} ({}): {} of {} documents written, {} objects, {} left out, {} renamed, {} journeys left out",
                rank + 1,
                delivery.label,
                delivery.source,
                delivery.documents_written,
                delivery.documents_read,
                delivery.objects,
                delivery.objects_left_out,
                delivery.objects_renamed,
                delivery.journeys_left_out
            );
        }
        out + &findings::human(&self.findings)
    }
}

/// The global identifiers an object states of itself: key and value.
fn global_keys<'o, 'x>(object: &'o Object<'x>) -> impl Iterator<Item = (&'x str, &'x str)> + 'o {
    object
        .keys
        .iter()
        .filter(move |(owner, _, _)| *owner == object.id)
        .map(|&(_, key, value)| (key, value))
}

/// Two stops at most this far apart are placed alike, in metres.
const PLACED_ALIKE: f64 = 50.0;

/// An object: its delivery, document and number.
type At = (u32, u32, u32);

/// What becomes of an object.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Fate {
    /// Written as it is.
    Kept,
    /// Left out: an earlier delivery answers its identifier alike.
    Alike,
    /// A stop place left out: its stop place and quays are an earlier
    /// delivery's.
    Paired,
    /// A journey left out: the journey of this identifier of an earlier
    /// delivery runs as it does.
    InBoth(String),
    /// Written under another identifier.
    Renamed(String),
    /// Left out: renamed, and needed only by what is left out; or an
    /// assignment of what is left out.
    NoLongerNeeded,
}

impl Fate {
    fn is_written(&self) -> bool {
        matches!(self, Fate::Kept | Fate::Renamed(_))
    }
}

/// A document as read: its name, its text and its outline.
struct Document<'x> {
    name: &'x str,
    text: &'x str,
    outline: Outline<'x>,
}

/// A delivery as read: its documents, sorted by name, its index, and what
/// becomes of each object.
struct Read<'x> {
    label: &'x str,
    documents: Vec<Document<'x>>,
    index: Index,
    /// For each identifier, the object holding it in the document whose
    /// name sorts first.
    first: FxHashMap<&'x str, (u32, u32)>,
    /// The assignments of each object: those whose subject finds it.
    assigned: FxHashMap<(u32, u32), Vec<(u32, u32)>>,
    fates: Vec<Vec<Fate>>,
}

impl<'x> Read<'x> {
    fn new(label: &'x str, documents: Vec<Document<'x>>, index: Index) -> Read<'x> {
        let mut first = FxHashMap::default();
        for (number, document) in documents.iter().enumerate() {
            for id in document.outline.identifiers() {
                first
                    .entry(id)
                    .or_insert((number as u32, document.outline.holders(id)[0]));
            }
        }
        let fates = documents
            .iter()
            .map(|document| vec![Fate::Kept; document.outline.objects.len()])
            .collect();
        let mut read = Read {
            label,
            documents,
            index,
            first,
            assigned: FxHashMap::default(),
            fates,
        };
        let mut assigned: FxHashMap<(u32, u32), Vec<(u32, u32)>> = FxHashMap::default();
        for (number, document) in read.documents.iter().enumerate() {
            for (object, assignment) in document.outline.objects.iter().enumerate() {
                let Some(subject) = subject_of(assignment.element) else {
                    continue;
                };
                for reference in &assignment.references {
                    if reference.element == subject {
                        if let Some(found) = read.resolve(number as u32, reference.id) {
                            assigned
                                .entry(read.assignee(found))
                                .or_default()
                                .push((number as u32, object as u32));
                        }
                    }
                }
            }
        }
        read.assigned = assigned;
        read
    }

    fn object(&self, document: u32, object: u32) -> &Object<'x> {
        &self.documents[document as usize].outline.objects[object as usize]
    }

    /// The object the assignments of `at` are filed under: `at`, or for a
    /// stop place or stop point — one thing however many documents state it —
    /// the statement in the document whose name sorts first.
    fn assignee(&self, at: (u32, u32)) -> (u32, u32) {
        let record = self.object(at.0, at.1);
        if matches!(record.element, "StopPlace" | "ScheduledStopPoint") {
            self.first.get(record.id).copied().unwrap_or(at)
        } else {
            at
        }
    }

    /// The assignments of `at`.
    fn assignments(&self, at: (u32, u32)) -> &[(u32, u32)] {
        self.assigned
            .get(&self.assignee(at))
            .map_or(&[], Vec::as_slice)
    }

    fn text(&self, document: u32, object: u32) -> &'x str {
        let range = self.object(document, object).range.clone();
        &self.documents[document as usize].text[range]
    }

    fn fate(&self, (document, object): (u32, u32)) -> &Fate {
        &self.fates[document as usize][object as usize]
    }

    /// The object a reference to `id` in `document` finds: its own
    /// document's, else that of the document whose name sorts first.
    fn resolve(&self, document: u32, id: &str) -> Option<(u32, u32)> {
        match self.documents[document as usize]
            .outline
            .holders(id)
            .first()
        {
            Some(&object) => Some((document, object)),
            None => self.first.get(id).copied(),
        }
    }

    /// Every object, with its document and number.
    fn objects(&self) -> impl Iterator<Item = ((u32, u32), &Object<'x>)> + '_ {
        self.documents
            .iter()
            .enumerate()
            .flat_map(|(number, document)| {
                document
                    .outline
                    .objects
                    .iter()
                    .enumerate()
                    .map(move |(object, record)| ((number as u32, object as u32), record))
            })
    }
}

/// A stop place of a delivery, over every document stating it.
#[derive(Debug, Default)]
struct Place<'x> {
    /// Where it is stated.
    definitions: Vec<(u32, u32)>,
    /// Its quays, each once.
    quays: Vec<&'x str>,
    /// The global identifier of it and of each quay stating one.
    global: FxHashMap<&'x str, &'x str>,
}

/// The stop places of a delivery.
fn places<'x>(read: &Read<'x>) -> BTreeMap<&'x str, Place<'x>> {
    let mut places: BTreeMap<&'x str, Place<'x>> = BTreeMap::new();
    for (at, object) in read.objects() {
        if object.element != "StopPlace" {
            continue;
        }
        let place = places.entry(object.id).or_default();
        place.definitions.push(at);
        for nested in &object.nested {
            if nested.element == "Quay" && !place.quays.contains(&nested.id) {
                place.quays.push(nested.id);
            }
        }
        for &(owner, _, value) in &object.keys {
            place.global.entry(owner).or_insert(value);
        }
    }
    places
}

/// The stops of the deliveries merged so far: what pairs with a stop of the
/// next.
#[derive(Default)]
struct Stops<'x> {
    /// Each stop place: its delivery and quays.
    places: FxHashMap<&'x str, (u32, Vec<&'x str>)>,
    /// Stop places by global identifier.
    by_global: FxHashMap<&'x str, &'x str>,
    /// Quays by global identifier: the quay and its stop place.
    quays_by_global: FxHashMap<&'x str, (&'x str, &'x str)>,
}

/// What a journey does: the station and the arrival and departure of each
/// call, then its headways.
type JourneyKey = Vec<(u32, u32, u32)>;

/// What the deliveries merged so far hold.
#[derive(Default)]
struct Merged<'x> {
    /// The object answering each identifier: of the earliest delivery
    /// keeping one, in its document whose name sorts first.
    answers: FxHashMap<&'x str, At>,
    /// The object answering each identifier and version: of the earliest
    /// delivery keeping one, in its document whose name sorts first.
    versions: FxHashMap<(&'x str, Option<&'x str>), At>,
    /// Every object kept, by identifier.
    definitions: FxHashMap<&'x str, Vec<At>>,
    stops: Stops<'x>,
    /// The journeys kept, by the stations they call at with their times and
    /// by their headways: delivery, identifier and days.
    journeys: FxHashMap<JourneyKey, Vec<(u32, String, Vec<i64>)>>,
    /// The stations, numbered.
    stations: FxHashMap<String, u32>,
}

/// Merges `inputs`, in rank, into the directory `to`.
pub fn merge(inputs: &[Input], to: &Path) -> Result<Report, Error> {
    let texts: Vec<Vec<(String, String)>> = inputs
        .iter()
        .map(|input| load(&input.source))
        .collect::<Result<_, _>>()?;
    let mut reads: Vec<Read> = Vec::with_capacity(inputs.len());
    for (input, documents) in inputs.iter().zip(&texts) {
        let documents: Vec<Document> = documents
            .par_iter()
            .map(|(name, text)| {
                Outline::of(text)
                    .map(|outline| Document {
                        name,
                        text,
                        outline,
                    })
                    .map_err(|error| Error::Document {
                        document: name.clone(),
                        message: error.to_string(),
                    })
            })
            .collect::<Result<_, _>>()?;
        let index = Index::build(&input.source, &ReadOptions::new(Selection::routing()))?;
        reads.push(Read::new(&input.label, documents, index));
    }

    let mut findings = Findings::default();
    let mut merged = Merged::default();
    let mut reports = Vec::with_capacity(reads.len());
    for (number, input) in inputs.iter().enumerate() {
        let (earlier, rest) = reads.split_at_mut(number);
        let read = &mut rest[0];
        let places = places(read);
        let mut rewrite: FxHashMap<&str, String> = FxHashMap::default();
        let mut renamed_quays: Vec<((u32, u32), &str)> = Vec::new();
        pair_stops(
            read,
            earlier,
            &places,
            &merged.stops,
            &mut rewrite,
            &mut renamed_quays,
            &mut findings,
        );
        for (here, there) in alike(read, earlier, &merged, &rewrite) {
            read.fates[here.0 as usize][here.1 as usize] = Fate::Alike;
            let object = read.object(here.0, here.1);
            findings.note(
                "object-alike",
                "an object stated alike by an earlier delivery, referring to objects alike: left out",
                format!(
                    "{} {} ({}) = {}",
                    object.element, object.id, read.label, earlier[there.0 as usize].label
                ),
            );
        }
        pair_journeys(
            number as u32,
            read,
            earlier,
            &mut merged,
            &rewrite,
            &mut findings,
        );
        rename(read, earlier, &merged, &mut findings);
        let written = write(number, read, &rewrite, &renamed_quays, to)?;
        reports.push(DeliveryReport {
            label: read.label.to_owned(),
            source: input.source.path().display().to_string(),
            documents_read: read.documents.len() as u64,
            documents_written: written,
            objects: read.fates.iter().map(|fates| fates.len() as u64).sum(),
            objects_left_out: count(read, |fate| !fate.is_written()),
            objects_renamed: count(read, |fate| matches!(fate, Fate::Renamed(_))),
            journeys_left_out: count(read, |fate| matches!(fate, Fate::InBoth(_))),
            stop_places_paired: places
                .values()
                .filter(|place| {
                    place
                        .definitions
                        .first()
                        .is_some_and(|&at| *read.fate(at) == Fate::Paired)
                })
                .count() as u64,
        });
        register(number as u32, read, &places, &mut merged);
    }
    Ok(Report {
        to: to.display().to_string(),
        deliveries: reports,
        findings: findings.into_list(),
    })
}

/// The documents of a source, sorted by name.
fn load(source: &Source) -> Result<Vec<(String, String)>, Error> {
    let mut documents: Vec<(String, String)> = source
        .read_each_with(Vec::new, |documents, entry, text| {
            documents.push(text.map(|text| (entry.name.clone(), text.to_owned())));
        })?
        .into_iter()
        .flatten()
        .collect::<Result<_, _>>()?;
    documents.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(documents)
}

fn count(read: &Read, which: impl Fn(&Fate) -> bool) -> u64 {
    read.fates
        .iter()
        .flatten()
        .filter(|fate| which(fate))
        .count() as u64
}

/// Pairs the stop places of `read` with those merged so far: a stop place
/// whose every quay pairs is left out, its identifiers pointed at the
/// earlier ones; one with a quay pairing with none is kept apart.
fn pair_stops<'x>(
    read: &mut Read<'x>,
    earlier: &[Read<'x>],
    places: &BTreeMap<&'x str, Place<'x>>,
    stops: &Stops<'x>,
    rewrite: &mut FxHashMap<&'x str, String>,
    renamed_quays: &mut Vec<((u32, u32), &'x str)>,
    findings: &mut Findings,
) {
    for (&id, place) in places {
        let base = if stops.places.contains_key(id) {
            id
        } else {
            match place
                .global
                .get(id)
                .and_then(|global| stops.by_global.get(global))
            {
                Some(&base) => base,
                None => continue,
            }
        };
        let (base_rank, base_quays) = &stops.places[base];
        let base_read = &earlier[*base_rank as usize];
        let pairs: Vec<(&str, Option<&str>)> = place
            .quays
            .iter()
            .map(|&quay| {
                let pair = if base_quays.contains(&quay) {
                    Some(quay)
                } else {
                    place
                        .global
                        .get(quay)
                        .and_then(|global| stops.quays_by_global.get(global))
                        .filter(|(_, of)| *of == base)
                        .map(|(paired, _)| *paired)
                };
                (quay, pair)
            })
            .collect();
        if let Some(&(unpaired, _)) = pairs.iter().find(|(_, pair)| pair.is_none()) {
            findings.note(
                "stop-kept-apart",
                "a stop place of an earlier delivery's identifier or global identifier with a quay pairing with none of that one's: kept apart, renamed where its identifier is taken",
                format!("{id} ({}): quay {unpaired}", read.label),
            );
            if base == id {
                let new = format!("{}:{id}", read.label);
                for &at in &place.definitions {
                    read.fates[at.0 as usize][at.1 as usize] = Fate::Renamed(new.clone());
                }
                rewrite.insert(id, new);
                for &quay in place.quays.iter().filter(|quay| base_quays.contains(quay)) {
                    for &at in &place.definitions {
                        renamed_quays.push((at, quay));
                    }
                    rewrite.insert(quay, format!("{}:{quay}", read.label));
                }
            }
            continue;
        }
        let quays = pairs
            .iter()
            .map(|&(quay, pair)| (quay, pair.expect("every quay pairs")));
        for (here, there) in std::iter::once((id, base)).chain(quays) {
            let (class, says) = if here == there {
                (
                    "stop-paired-by-identifier",
                    "a stop place or quay with the identifier of an earlier delivery's: left out",
                )
            } else {
                (
                    "stop-paired-by-global-id",
                    "a stop place or quay with the global identifier of an earlier delivery's under another identifier: left out, its references pointed there",
                )
            };
            findings.note(
                class,
                says,
                format!("{here} ({}) = {there} ({})", read.label, base_read.label),
            );
            if here != there {
                rewrite.insert(here, there.to_owned());
            }
            compare_stops(read, here, base_read, there, findings);
        }
        for &at in &place.definitions {
            read.fates[at.0 as usize][at.1 as usize] = Fate::Paired;
        }
    }
}

/// Finds what differs between two paired stops: their names, their
/// positions.
fn compare_stops(read: &Read, here: &str, base: &Read, there: &str, findings: &mut Findings) {
    let (Some(a), Some(b)) = (read.index.stop_by_id(here), base.index.stop_by_id(there)) else {
        return;
    };
    let (a, b) = (read.index.stop(a), base.index.stop(b));
    if a.name != b.name {
        findings.note(
            "stop-named-otherwise",
            "a paired stop named otherwise: the earlier delivery's name kept",
            format!(
                "{here}: '{}' ({}), '{}' ({}) kept",
                a.name, read.label, b.name, base.label
            ),
        );
    }
    let (Some(la), Some(oa), Some(lb), Some(ob)) =
        (a.latitude, a.longitude, b.latitude, b.longitude)
    else {
        return;
    };
    let metres = distance((la, oa), (lb, ob));
    if metres > PLACED_ALIKE {
        findings.note(
            "stop-placed-otherwise",
            "a paired stop placed more than 50 m away: the earlier delivery's position kept",
            format!(
                "{here} ({}): {metres:.0} m from {there} ({})",
                read.label, base.label
            ),
        );
    }
}

/// The distance between two positions, latitude and longitude, in metres.
fn distance(a: (f64, f64), b: (f64, f64)) -> f64 {
    let (la, lb) = (a.0.to_radians(), b.0.to_radians());
    let (dl, dn) = (lb - la, (b.1 - a.1).to_radians());
    let h = (dl / 2.0).sin().powi(2) + la.cos() * lb.cos() * (dn / 2.0).sin().powi(2);
    2.0 * 6_371_008.8 * h.sqrt().asin()
}

/// The object a reference finding `at` of an earlier delivery finds in the
/// merge: the object an earlier delivery answers its identifier with, where
/// `at` is left out as alike or paired.
fn settled(earlier: &[Read], at: At, merged: &Merged) -> At {
    let (delivery, document, object) = at;
    let read = &earlier[delivery as usize];
    match read.fate((document, object)) {
        Fate::Alike | Fate::Paired => {
            let record = read.object(document, object);
            merged
                .versions
                .get(&(record.id, record.version))
                .or_else(|| merged.answers.get(record.id))
                .copied()
                .unwrap_or(at)
        }
        _ => at,
    }
}

/// The objects of `read` stated alike with the object an earlier delivery
/// answers their identifier with — written alike, white space aside,
/// referring to objects alike and assigned alike —, each with that object.
fn alike(
    read: &Read,
    earlier: &[Read],
    merged: &Merged,
    rewrite: &FxHashMap<&str, String>,
) -> Vec<((u32, u32), At)> {
    // Every object written alike with an object kept of its identifier.
    let mut candidates: FxHashSet<((u32, u32), At)> = FxHashSet::default();
    for (here, object) in read.objects() {
        if *read.fate(here) != Fate::Kept || is_furniture(object.element) {
            continue;
        }
        for &there in merged.definitions.get(object.id).into_iter().flatten() {
            let base = &earlier[there.0 as usize];
            if base.object(there.1, there.2).element == object.element
                && read
                    .text(here.0, here.1)
                    .split_ascii_whitespace()
                    .eq(base.text(there.1, there.2).split_ascii_whitespace())
            {
                candidates.insert((here, there));
            }
        }
    }
    // Keep those whose references and assignments are alike, until none fails.
    let finds_alike = |candidates: &FxHashSet<((u32, u32), At)>, here: (u32, u32), there: At| {
        let base = &earlier[there.0 as usize];
        let references = read
            .object(here.0, here.1)
            .references
            .iter()
            .all(|reference| {
                let mine = read.resolve(here.0, reference.id);
                let theirs = base
                    .resolve(there.1, reference.id)
                    .map(|(document, object)| {
                        settled(earlier, (there.0, document, object), merged)
                    });
                match (mine, theirs) {
                    (None, None) => true,
                    (Some(mine), Some(theirs)) if *read.fate(mine) == Fate::Paired => {
                        // A stop place is one however many documents state
                        // it: the earlier one is paired if it states what is.
                        let paired = rewrite
                            .get(reference.id)
                            .map_or(reference.id, String::as_str);
                        let place = earlier[theirs.0 as usize].object(theirs.1, theirs.2);
                        place.id == paired || place.nested.iter().any(|nested| nested.id == paired)
                    }
                    (Some(mine), Some(theirs)) => candidates.contains(&(mine, theirs)),
                    _ => false,
                }
            });
        let mine = read.assignments(here);
        let theirs = base.assignments((there.1, there.2));
        references
            && mine.len() == theirs.len()
            && mine.iter().all(|&assignment| {
                theirs.iter().any(|&(document, object)| {
                    candidates.contains(&(assignment, (there.0, document, object)))
                })
            })
    };
    loop {
        let failing: Vec<((u32, u32), At)> = candidates
            .iter()
            .filter(|&&(here, there)| !finds_alike(&candidates, here, there))
            .copied()
            .collect();
        if failing.is_empty() {
            break;
        }
        for pair in failing {
            candidates.remove(&pair);
        }
    }
    let mut alike: Vec<((u32, u32), At)> = candidates
        .into_iter()
        .filter(|&(here, there)| {
            let record = read.object(here.0, here.1);
            merged.versions.get(&(record.id, record.version)) == Some(&there)
        })
        .collect();
    alike.sort_unstable();
    alike
}

/// What a journey of `read` does: the stations it calls at with its times,
/// and its headways.
fn journey_key(
    read: &Read,
    journey: JourneyId,
    rewrite: &FxHashMap<&str, String>,
    stations: &mut FxHashMap<String, u32>,
) -> JourneyKey {
    let index = &read.index;
    let record = index.journey(journey);
    let mut key = Vec::with_capacity(record.stop_points.len() + record.headways.len());
    for (&point, time) in record.stop_points.iter().zip(record.passing_times) {
        let station = match index.stop_of_stop_point(point) {
            Some(stop) => {
                let place = index.stop(stop).parent.unwrap_or(stop);
                index.stop(place).id
            }
            None => index.stop_point(point).id,
        };
        let station = rewrite.get(station).map_or(station, String::as_str);
        let next = stations.len() as u32;
        let number = *stations.entry(station.to_owned()).or_insert(next);
        key.push((
            number,
            time.arrival.unwrap_or(u32::MAX),
            time.departure.unwrap_or(u32::MAX),
        ));
    }
    for headway in record.headways {
        key.push((u32::MAX, headway.first_departure, headway.interval));
        key.push((u32::MAX - 1, headway.last_departure, 0));
    }
    key
}

/// Leaves out the journeys of `read` an earlier delivery states with the
/// same calls and times on the same days or more, and registers the others.
fn pair_journeys(
    rank: u32,
    read: &mut Read,
    earlier: &[Read],
    merged: &mut Merged,
    rewrite: &FxHashMap<&str, String>,
    findings: &mut Findings,
) {
    let mut kept = Vec::new();
    let ats: Vec<(u32, u32)> = read
        .objects()
        .filter(|(_, object)| is_journey(object.element))
        .map(|(at, _)| at)
        .collect();
    for at in ats {
        if *read.fate(at) != Fate::Kept {
            continue;
        }
        let (id, name) = (
            read.object(at.0, at.1).id,
            read.documents[at.0 as usize].name,
        );
        let Some(journey) = read.index.journey_in_document(id, name) else {
            continue;
        };
        let key = journey_key(read, journey, rewrite, &mut merged.stations);
        let days: Vec<i64> = read.index.days_of(journey).collect();
        let same = merged.journeys.get(&key).into_iter().flatten();
        let mut stated = false;
        for (delivery, base, base_days) in same {
            stated = true;
            if days.iter().all(|day| base_days.binary_search(day).is_ok()) {
                findings.note(
                    "journey-in-both",
                    "a journey an earlier delivery states with the same calls and times on these days too: left out, its references pointed there",
                    format!("{id} ({}) = {base} ({})", read.label, earlier[*delivery as usize].label),
                );
                read.fates[at.0 as usize][at.1 as usize] = Fate::InBoth(base.clone());
                break;
            }
        }
        if *read.fate(at) == Fate::Kept {
            if stated {
                findings.note(
                    "journey-in-both-other-days",
                    "a journey an earlier delivery states with the same calls and times on other days: both kept",
                    format!("{id} ({})", read.label),
                );
            }
            kept.push((key, id, days));
        }
    }
    for (key, id, days) in kept {
        merged
            .journeys
            .entry(key)
            .or_default()
            .push((rank, id.to_owned(), days));
    }
}

/// Renames the objects of `read` whose identifier an earlier delivery
/// answers otherwise, and leaves out those renamed only objects left out
/// needed, and the assignments of what is left out.
fn rename(read: &mut Read, earlier: &[Read], merged: &Merged, findings: &mut Findings) {
    let taken: Vec<(u32, u32)> = read
        .objects()
        .filter(|(at, object)| {
            *read.fate(*at) == Fate::Kept
                && !is_furniture(object.element)
                && merged.answers.contains_key(object.id)
        })
        .map(|(at, _)| at)
        .collect();
    for at in taken {
        let object = read.object(at.0, at.1);
        let answer = merged.answers[object.id];
        let base = &earlier[answer.0 as usize];
        let same_place = global_keys(object)
            .any(|key| global_keys(base.object(answer.1, answer.2)).any(|other| other == key));
        if same_place {
            findings.note(
                "place-stated-otherwise",
                "a place under the identifier and global identifier of an earlier delivery's, stated otherwise: left out, the earlier one kept",
                format!("{} {} ({}), {} kept", object.element, object.id, read.label, base.label),
            );
            read.fates[at.0 as usize][at.1 as usize] = Fate::Paired;
        } else {
            let new = format!("{}:{}", read.label, object.id);
            read.fates[at.0 as usize][at.1 as usize] = Fate::Renamed(new);
        }
    }
    loop {
        let mut referred = FxHashSet::default();
        let mut used = FxHashSet::default();
        for (at, object) in read.objects() {
            // An assignment does not use what it assigns, only what it
            // assigns it to.
            let written = read.fate(at).is_written();
            let subject = subject_of(object.element);
            for reference in &object.references {
                if let Some(found) = read
                    .resolve(at.0, reference.id)
                    .filter(|found| *found != at)
                {
                    referred.insert(found);
                    if written && subject != Some(reference.element) {
                        used.insert(found);
                    }
                }
            }
        }
        let mut needless = Vec::new();
        for (at, object) in read.objects() {
            let fate = read.fate(at);
            let renamed_for_nothing = matches!(fate, Fate::Renamed(_))
                && !is_place(object.element)
                && referred.contains(&at)
                && !used.contains(&at);
            let assigns_nothing = fate.is_written()
                && subject_of(object.element).is_some_and(|subject| {
                    object.references.iter().any(|reference| {
                        reference.element == subject
                            && read.resolve(at.0, reference.id).is_some_and(|found| {
                                matches!(read.fate(found), Fate::InBoth(_) | Fate::NoLongerNeeded)
                            })
                    })
                });
            if renamed_for_nothing || assigns_nothing {
                needless.push(at);
            }
        }
        if needless.is_empty() {
            break;
        }
        for at in needless {
            read.fates[at.0 as usize][at.1 as usize] = Fate::NoLongerNeeded;
        }
    }
    for (at, object) in read.objects() {
        match read.fate(at) {
            Fate::Renamed(new) => findings.note(
                "identifier-renamed",
                "an object under an identifier an earlier delivery answers otherwise: renamed into the delivery's codespace, its references with it",
                format!("{} {} → {new}", object.element, object.id),
            ),
            Fate::NoLongerNeeded => findings.note(
                "object-no-longer-needed",
                "an object only what is left out needed, or an assignment of what is left out: left out",
                format!("{} {} ({})", object.element, object.id, read.label),
            ),
            _ => {}
        }
    }
}

/// The identifier a reference to `id` in `document` is written with: the
/// earlier delivery's for what is paired or stated there, the new one for
/// what is renamed.
fn rewritten(
    read: &Read,
    document: u32,
    id: &str,
    rewrite: &FxHashMap<&str, String>,
) -> Option<String> {
    let found = read.resolve(document, id)?;
    let own = read.object(found.0, found.1).id == id;
    match read.fate(found) {
        Fate::Paired => rewrite.get(id).cloned(),
        Fate::Renamed(new) if own => Some(new.clone()),
        Fate::Renamed(_) => rewrite.get(id).cloned(),
        Fate::InBoth(base) if own => Some(base.clone()),
        _ => None,
    }
}

/// Writes the documents of `read` keeping an object under
/// `<to>/<rank>-<label>/`; how many.
fn write(
    number: usize,
    read: &Read,
    rewrite: &FxHashMap<&str, String>,
    renamed_quays: &[((u32, u32), &str)],
    to: &Path,
) -> Result<u64, Error> {
    let directory = to.join(format!("{}-{}", number + 1, read.label));
    let mut written = 0;
    for (index, document) in read.documents.iter().enumerate() {
        let number = index as u32;
        let outline = &document.outline;
        let mut edits = Edits::default();
        let mut filled = vec![false; outline.containers.len()];
        let mut keeps = false;
        for (object, record) in outline.objects.iter().enumerate() {
            let fate = read.fate((number, object as u32));
            if !fate.is_written() && !is_furniture(record.element) {
                edits.remove(record.range.clone());
                continue;
            }
            filled[record.container as usize] = true;
            keeps |= !is_furniture(record.element);
            if let Fate::Renamed(new) = fate {
                edits.replace(record.id_at.clone(), attribute_value(new));
            }
            for reference in &record.references {
                if let Some(new) = rewritten(read, number, reference.id, rewrite) {
                    edits.replace(reference.at.clone(), attribute_value(&new));
                }
            }
        }
        if !keeps {
            continue;
        }
        for &((document, object), quay) in renamed_quays {
            if document != number {
                continue;
            }
            let record = read.object(document, object);
            for nested in record.nested.iter().filter(|nested| nested.id == quay) {
                edits.replace(nested.id_at.clone(), attribute_value(&rewrite[quay]));
            }
        }
        for reference in &outline.header {
            if let Some(new) = rewritten(read, number, reference.id, rewrite) {
                edits.replace(reference.at.clone(), attribute_value(&new));
            }
        }
        for (container, filled) in outline.containers.iter().zip(filled) {
            if !filled && !container.holds_others {
                edits.remove(container.range.clone());
            }
        }
        let path: PathBuf = directory.join(document.name);
        let failed = |error| Error::Write {
            path: path.clone(),
            error,
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(failed)?;
        }
        let mut out = BufWriter::new(File::create(&path).map_err(failed)?);
        edits.write(document.text, &mut out).map_err(failed)?;
        out.flush().map_err(failed)?;
        written += 1;
    }
    Ok(written)
}

/// Adds what `read` keeps to what is merged.
fn register<'x>(
    rank: u32,
    read: &Read<'x>,
    places: &BTreeMap<&'x str, Place<'x>>,
    merged: &mut Merged<'x>,
) {
    for ((document, object), record) in read.objects() {
        if *read.fate((document, object)) != Fate::Kept {
            continue;
        }
        let at = (rank, document, object);
        merged.answers.entry(record.id).or_insert(at);
        merged
            .versions
            .entry((record.id, record.version))
            .or_insert(at);
        for nested in &record.nested {
            merged.answers.entry(nested.id).or_insert(at);
        }
        merged.definitions.entry(record.id).or_default().push(at);
    }
    for (&id, place) in places {
        if place
            .definitions
            .first()
            .is_none_or(|&at| *read.fate(at) != Fate::Kept)
        {
            continue;
        }
        merged
            .stops
            .places
            .entry(id)
            .or_insert_with(|| (rank, place.quays.clone()));
        if let Some(&global) = place.global.get(id) {
            merged.stops.by_global.entry(global).or_insert(id);
        }
        for &quay in &place.quays {
            if let Some(&global) = place.global.get(quay) {
                merged
                    .stops
                    .quays_by_global
                    .entry(global)
                    .or_insert((quay, id));
            }
        }
    }
}
