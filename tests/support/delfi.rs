//! What an independent Python extractor, `extract_netex.py` (outside this
//! repository, as of 2026-08-08), lifts out of a DELFI line file, ported rule
//! for rule so that the two can be compared over the whole delivery.
//!
//! Its semantics are kept even where the crate could do better: references
//! resolve inside their own file only, a quay reference that does not resolve
//! makes the walk unknown rather than falling back to the stop place, a walk
//! between two identifiers of one DHID is dropped, and the accessibility axis
//! of a notice is recognised from its German wording, never from its code.

use std::collections::{BTreeSet, HashMap};
use std::sync::OnceLock;

use netex_rs::model::{
    ConnectionItem, Frame, NoticeOrRef, PointInSequenceItem, QuayItem, StopAssignmentItem,
    StopPlaceItem, StopPointRef,
};
use netex_rs::Document;
use regex::Regex;

/// A walk between two DHIDs.
pub type Footpath = (String, String, u64, bool);

/// An accessibility fact at a DHID: axis, comparator, value, unit, notice code, notice text.
pub type Accessibility = (String, String, String, String, String, String, String);

/// What one line file contributes.
#[derive(Debug, Default)]
pub struct Extract {
    pub footpaths: BTreeSet<Footpath>,
    pub accessibility: BTreeSet<Accessibility>,
    pub quay_levels: HashMap<String, String>,
    pub footpaths_unresolved: u64,
    pub footpaths_self: u64,
    pub notice_points_unassigned: u64,
}

impl Extract {
    pub fn merge(mut self, other: Extract) -> Extract {
        self.footpaths.extend(other.footpaths);
        self.accessibility.extend(other.accessibility);
        self.quay_levels.extend(other.quay_levels);
        self.footpaths_unresolved += other.footpaths_unresolved;
        self.footpaths_self += other.footpaths_self;
        self.notice_points_unassigned += other.notice_points_unassigned;
        self
    }
}

/// One accessibility fact read out of a notice's wording.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    pub axis: &'static str,
    pub comparator: &'static str,
    pub value: String,
    pub unit: String,
}

struct Patterns {
    axes: Vec<(&'static str, Regex, Option<Regex>)>,
    comparators: Vec<(&'static str, Regex)>,
    unit_then_value: Regex,
    no_steps: Regex,
    range: Regex,
}

fn patterns() -> &'static Patterns {
    static PATTERNS: OnceLock<Patterns> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        let re = |pattern: &str| Regex::new(&format!("(?i){pattern}")).expect("pattern compiles");
        Patterns {
            axes: vec![
                (
                    "step_height",
                    re(r"stufe\s+zwischen\s+bahnsteig\s+und\s+fahrzeug"),
                    None,
                ),
                (
                    "gap_width",
                    re(r"spalt\s+zwischen\s+bahnsteig\s+und\s+fahrzeug|einstiegsspalt"),
                    None,
                ),
                ("step_count", re(r"stufen\s+(am|im)\s+fahrzeug"), None),
                (
                    "door_width",
                    re(r"türbreite|lichte\s+breite\s+(der\s+)?fahrzeugtür"),
                    None,
                ),
                (
                    "floor_height",
                    re(r"fahrzeugbodenhöhe|höhe\s+des\s+fahrzeugbodens|einstiegsh(ö|oe)he"),
                    None,
                ),
                (
                    "movement_area_width",
                    re(r"breite\s+der\s+(minimalen\s+)?bewegungsfläche"),
                    None,
                ),
                (
                    "movement_area_length",
                    re(r"länge\s+der\s+(minimalen\s+)?bewegungsfläche"),
                    None,
                ),
                (
                    "ramp_capacity",
                    re(r"rampentragfähigkeit|tragfähigkeit\s+der\s+rampe|kg\s+tragfähigkeit"),
                    Some(re("rampe")),
                ),
                (
                    "ramp_slope",
                    re(r"rampenneigung|neigung"),
                    Some(re("rampe")),
                ),
                ("ramp_length", re(r"rampenlänge"), None),
                ("ramp_width", re(r"rampenbreite|breite\s+der\s+rampe"), None),
            ],
            comparators: vec![
                (
                    "le",
                    re(r"kleiner\s*/?\s*gleich|höchstens|maximal|max\.|bis\s+zu"),
                ),
                ("lt", re(r"\bunter\b")),
                ("ge", re(r"mindestens|min\.|größer\s*/?\s*gleich")),
                ("approx", re(r"\betwa\b|\bca\.\s|\bungefähr\b")),
            ],
            unit_then_value: re(r"\bin\s+(mm|cm|m|kg)\b[^0-9]{0,12}(\d+(?:[.,]\d+)?)"),
            no_steps: re(r"\bkeine\s+stufen\b"),
            range: re(r"\d+(?:[.,]\d+)?\s*[-–—]\s*\d+(?:[.,]\d+)?\s*(?:mm|cm|m|kg|%)"),
        }
    })
}

/// The Python `_VALUE_WITH_UNIT`: `(\d+(?:[.,]\d+)?)\s*(mm|cm|m|kg|%)(?![a-zäöüß])`,
/// case-insensitive. The negative lookahead has no equivalent in the `regex`
/// crate, so the search is written out: at each number, the units are tried in
/// the pattern's order and the first whose next character is not a letter wins.
fn value_with_unit(text: &str) -> Option<(String, String)> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        let mut end = i;
        if i + 1 < chars.len()
            && (chars[i] == '.' || chars[i] == ',')
            && chars[i + 1].is_ascii_digit()
        {
            let mut j = i + 1;
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            end = j;
        }
        // The pattern is greedy over the fraction, but a fraction that leaves no
        // unit behind it backtracks to the integer in Python; try both.
        for number_end in [end, i] {
            let mut k = number_end;
            while k < chars.len() && chars[k].is_whitespace() {
                k += 1;
            }
            for unit in ["mm", "cm", "m", "kg", "%"] {
                let unit_chars: Vec<char> = unit.chars().collect();
                let matches = chars.len() >= k + unit_chars.len()
                    && chars[k..k + unit_chars.len()]
                        .iter()
                        .zip(&unit_chars)
                        .all(|(a, b)| a.to_lowercase().eq(b.to_lowercase()));
                if !matches {
                    continue;
                }
                let next = chars.get(k + unit_chars.len()).copied();
                let letter = next.is_some_and(|c| {
                    let lower: String = c.to_lowercase().collect();
                    matches!(
                        lower.as_str(),
                        "a" | "b"
                            | "c"
                            | "d"
                            | "e"
                            | "f"
                            | "g"
                            | "h"
                            | "i"
                            | "j"
                            | "k"
                            | "l"
                            | "m"
                            | "n"
                            | "o"
                            | "p"
                            | "q"
                            | "r"
                            | "s"
                            | "t"
                            | "u"
                            | "v"
                            | "w"
                            | "x"
                            | "y"
                            | "z"
                            | "ä"
                            | "ö"
                            | "ü"
                            | "ß"
                    )
                });
                if letter {
                    continue;
                }
                let value: String = chars[start..number_end].iter().collect();
                return Some((value.replace(',', "."), unit.to_lowercase()));
            }
            if number_end == i {
                break;
            }
        }
        // `(?<![\d.,])` is implicit: the scan starts at the first digit of a run.
        while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.' || chars[i] == ',') {
            i += 1;
        }
    }
    None
}

/// The Python `_BARE_COUNT`: a run of digits not adjoining a digit, `.` or `,`.
fn bare_count(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_ascii_digit() {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && chars[i].is_ascii_digit() {
            i += 1;
        }
        let before = start > 0 && matches!(chars[start - 1], '.' | ',');
        let after = i < chars.len() && matches!(chars[i], '.' | ',');
        if !before && !after {
            return Some(chars[start..i].iter().collect());
        }
    }
    None
}

/// The Python `classify_notice`.
pub fn classify_notice(text: &str) -> Option<Fact> {
    let patterns = patterns();
    if text.is_empty() || patterns.range.is_match(text) {
        return None;
    }
    let axis = patterns
        .axes
        .iter()
        .find(|(_, pattern, requires)| {
            pattern.is_match(text) && requires.as_ref().is_none_or(|r| r.is_match(text))
        })
        .map(|(axis, _, _)| *axis)?;
    let comparator = patterns
        .comparators
        .iter()
        .find(|(_, pattern)| pattern.is_match(text))
        .map_or("eq", |(comparator, _)| *comparator);
    if axis == "step_count" && patterns.no_steps.is_match(text) {
        return Some(Fact {
            axis,
            comparator: "eq",
            value: "0".to_owned(),
            unit: String::new(),
        });
    }
    if let Some((value, unit)) = value_with_unit(text) {
        return Some(Fact {
            axis,
            comparator,
            value,
            unit,
        });
    }
    if let Some(captures) = patterns.unit_then_value.captures(text) {
        return Some(Fact {
            axis,
            comparator,
            value: captures[2].replace(',', "."),
            unit: captures[1].to_lowercase(),
        });
    }
    if axis == "step_count" {
        if let Some(count) = bare_count(text) {
            return Some(Fact {
                axis,
                comparator,
                value: count,
                unit: String::new(),
            });
        }
    }
    None
}

/// The Python `_clean`: one line, single spaces.
pub fn clean(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn id_of(id: &Option<netex_rs::model::Id>) -> &str {
    id.as_ref().map_or("", |id| id.as_str())
}

fn ref_of(reference: &Option<netex_rs::model::Ref>) -> &str {
    reference
        .as_ref()
        .and_then(|r| r.r#ref.as_ref())
        .map_or("", |id| id.as_str())
}

/// The Python `extract_netex_file`, over the typed model.
pub fn extract(document: &Document) -> Extract {
    let mut dhid_of: HashMap<&str, &str> = HashMap::new();
    let mut level_names: HashMap<&str, &str> = HashMap::new();
    let mut quay_level_refs: HashMap<&str, &str> = HashMap::new();
    let mut assignment: HashMap<&str, (&str, &str)> = HashMap::new();
    let mut notices: HashMap<&str, Fact> = HashMap::new();
    let mut notice_source: HashMap<&str, (&str, &str)> = HashMap::new();
    let mut connections: Vec<(&str, &str, &str, &str, u64, bool)> = Vec::new();
    let mut points: Vec<(&str, Vec<&str>)> = Vec::new();

    for frame in document.delivery.frames() {
        match frame {
            Frame::Site(site) => {
                for item in site.stop_places.iter().flat_map(|s| &s.items) {
                    let StopPlaceItem::StopPlace(place) = item else {
                        continue;
                    };
                    if let Some(dhid) = place.key_list.as_ref().and_then(|k| k.get("GlobalID")) {
                        let dhid = dhid.trim();
                        if !dhid.is_empty() {
                            dhid_of.insert(id_of(&place.id), dhid);
                        }
                    }
                    for level in place.levels.iter().flat_map(|l| &l.levels) {
                        let short = level.short_name.as_ref().map_or("", |n| n.as_str().trim());
                        if !short.is_empty() {
                            level_names.insert(id_of(&level.id), short);
                        }
                    }
                    for item in place.quays.iter().flat_map(|q| &q.items) {
                        let QuayItem::Quay(quay) = item else { continue };
                        let Some(dhid) = quay
                            .key_list
                            .as_ref()
                            .and_then(|k| k.get("GlobalID"))
                            .map(str::trim)
                        else {
                            continue;
                        };
                        if dhid.is_empty() {
                            continue;
                        }
                        dhid_of.insert(id_of(&quay.id), dhid);
                        let level_ref = ref_of(&quay.level_ref);
                        if !level_ref.is_empty() {
                            quay_level_refs.insert(dhid, level_ref);
                        }
                    }
                }
            }
            Frame::Service(service) => {
                for item in service.connections.iter().flat_map(|c| &c.items) {
                    let ConnectionItem::SiteConnection(connection) = item else {
                        continue;
                    };
                    let seconds = connection
                        .walk_transfer_duration
                        .as_ref()
                        .and_then(|w| w.default_duration.as_ref())
                        .and_then(|d| d.seconds());
                    let (Some(seconds), Some(from), Some(to)) =
                        (seconds, &connection.from, &connection.to)
                    else {
                        continue;
                    };
                    connections.push((
                        ref_of(&from.quay_ref),
                        ref_of(&from.stop_place_ref),
                        ref_of(&to.quay_ref),
                        ref_of(&to.stop_place_ref),
                        seconds,
                        connection.both_ways == Some(true),
                    ));
                }
                for item in service.stop_assignments.iter().flat_map(|s| &s.items) {
                    let StopAssignmentItem::PassengerStopAssignment(a) = item else {
                        continue;
                    };
                    let point = ref_of(&a.scheduled_stop_point_ref);
                    if !point.is_empty() {
                        assignment.insert(point, (ref_of(&a.quay_ref), ref_of(&a.stop_place_ref)));
                    }
                }
                for item in service.journey_patterns.iter().flat_map(|j| &j.items) {
                    let pattern = match item {
                        netex_rs::model::JourneyPatternItem::ServiceJourneyPattern(p)
                        | netex_rs::model::JourneyPatternItem::JourneyPattern(p) => p,
                        netex_rs::model::JourneyPatternItem::Unprocessed(_) => continue,
                    };
                    for item in pattern.points_in_sequence.iter().flat_map(|p| &p.items) {
                        let PointInSequenceItem::StopPointInJourneyPattern(point) = item else {
                            continue;
                        };
                        let mut ids = Vec::new();
                        for na in point
                            .notice_assignments
                            .iter()
                            .flat_map(|n| &n.notice_assignments)
                        {
                            match &na.notice {
                                Some(NoticeOrRef::Notice(notice)) => {
                                    let notice_id = id_of(&notice.id);
                                    let code = notice.public_code.as_deref().map_or("", str::trim);
                                    let text =
                                        notice.text.as_ref().map_or("", |t| t.as_str().trim());
                                    notice_source.insert(notice_id, (code, text));
                                    if let Some(fact) = classify_notice(text) {
                                        notices.insert(notice_id, fact);
                                    }
                                    ids.push(notice_id);
                                }
                                Some(NoticeOrRef::NoticeRef(reference)) => {
                                    let id = reference.r#ref.as_ref().map_or("", |id| id.as_str());
                                    if !id.is_empty() {
                                        ids.push(id);
                                    }
                                }
                                _ => {}
                            }
                        }
                        if !ids.is_empty() {
                            let stop_point = match &point.scheduled_stop_point_ref {
                                Some(
                                    StopPointRef::ScheduledStopPoint(r)
                                    | StopPointRef::FareScheduledStopPoint(r),
                                ) => r.r#ref.as_ref().map_or("", |id| id.as_str()),
                                None => "",
                            };
                            points.push((stop_point, ids));
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let endpoint = |quay_ref: &str, place_ref: &str| -> &str {
        if !quay_ref.is_empty() {
            dhid_of.get(quay_ref).copied().unwrap_or("")
        } else {
            dhid_of.get(place_ref).copied().unwrap_or("")
        }
    };
    let mut result = Extract::default();
    for (from_quay, from_place, to_quay, to_place, seconds, both_ways) in connections {
        let origin = endpoint(from_quay, from_place);
        let target = endpoint(to_quay, to_place);
        if origin.is_empty() || target.is_empty() {
            result.footpaths_unresolved += 1;
            continue;
        }
        if origin == target {
            result.footpaths_self += 1;
            continue;
        }
        result
            .footpaths
            .insert((origin.to_owned(), target.to_owned(), seconds, both_ways));
    }
    for (point, notice_ids) in points {
        let (quay_ref, place_ref) = assignment.get(point).copied().unwrap_or(("", ""));
        let dhid = dhid_of
            .get(quay_ref)
            .or_else(|| dhid_of.get(place_ref))
            .copied()
            .unwrap_or("");
        if dhid.is_empty() {
            result.notice_points_unassigned += 1;
            continue;
        }
        for notice_id in notice_ids {
            let Some(fact) = notices.get(notice_id) else {
                continue;
            };
            let (code, text) = notice_source[notice_id];
            result.accessibility.insert((
                dhid.to_owned(),
                fact.axis.to_owned(),
                fact.comparator.to_owned(),
                fact.value.clone(),
                fact.unit.clone(),
                clean(code),
                clean(text),
            ));
        }
    }
    for (quay_dhid, level_ref) in quay_level_refs {
        if let Some(name) = level_names.get(level_ref) {
            result
                .quay_levels
                .insert(quay_dhid.to_owned(), (*name).to_owned());
        }
    }
    result
}
