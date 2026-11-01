//! The objects of a document as written: each object of a frame — a stop
//! place, a line, a journey, a day type —, where it stands, what it refers to
//! and what it holds, and the containers holding the objects. `extract` keeps
//! some objects and leaves the others out; `merge` leaves out or renames
//! objects and points references elsewhere — both by these positions, every
//! other byte of the document as it was.

use std::ops::Range;

use rustc_hash::FxHashMap;

use super::scan::{self, Close, Open, ScanError, Visitor};

/// A reference as written: an element's `ref` attribute or an attribute such
/// as `responsibilitySetRef`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference<'x> {
    /// The element (`QuayRef`) or the attribute (`responsibilitySetRef`).
    pub element: &'x str,
    /// The identifier referred to.
    pub id: &'x str,
    /// Where the identifier stands.
    pub at: Range<usize>,
    /// Where the version the reference names stands, if it names one.
    pub version: Option<Range<usize>>,
}

/// Where a call of a journey is: a point of its pattern (a passing time) or a
/// stop point (a call the Swiss way).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Called<'x> {
    /// `StopPointInJourneyPatternRef`.
    PatternPoint(&'x str),
    /// `ScheduledStopPointRef`.
    StopPoint(&'x str),
}

/// A passing time or a call of a journey.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Call<'x> {
    /// The element as written.
    pub range: Range<usize>,
    /// Where it calls, if it says.
    pub at: Option<Called<'x>>,
}

/// An element with an identifier within an object: a quay of a stop place,
/// a point of a pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Nested<'x> {
    /// The element.
    pub element: &'x str,
    /// Its identifier.
    pub id: &'x str,
    /// Where its identifier stands.
    pub id_at: Range<usize>,
}

/// An object of a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Object<'x> {
    /// The element: `StopPlace`, `ServiceJourney`, …
    pub element: &'x str,
    /// Its identifier.
    pub id: &'x str,
    /// Where its identifier stands.
    pub id_at: Range<usize>,
    /// Its version, if it states one.
    pub version: Option<&'x str>,
    /// The element as written.
    pub range: Range<usize>,
    /// The container holding it, by number.
    pub container: u32,
    /// Its references, its own attributes' and those within it.
    pub references: Vec<Reference<'x>>,
    /// The elements with an identifier within it.
    pub nested: Vec<Nested<'x>>,
    /// Its passing times or calls, for a journey.
    pub calls: Vec<Call<'x>>,
    /// The stop point of each point of its pattern, for a journey pattern:
    /// point and stop point.
    pub points: Vec<(&'x str, &'x str)>,
    /// The `keyList` pairs of it and of elements within it naming a stop
    /// globally — `GlobalID`, `SLOID` —, and the codes the Dutch stop
    /// register gives them (`privateCodes`, `QuayCode`, `StopPlaceCode`), the
    /// key the German delivery names those stops by: the identifier of the
    /// element, the key or the code's type, and the value.
    pub keys: Vec<(&'x str, &'x str, &'x str)>,
}

/// A container of a frame: `stopPlaces`, `vehicleJourneys`, `members`, …
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Container<'x> {
    /// The element.
    pub element: &'x str,
    /// The element as written.
    pub range: Range<usize>,
    /// Whether it holds anything besides objects.
    pub holds_others: bool,
}

/// The objects of a document.
#[derive(Debug, Default)]
pub struct Outline<'x> {
    /// The objects, in the order written.
    pub objects: Vec<Object<'x>>,
    /// The containers, in the order they open.
    pub containers: Vec<Container<'x>>,
    /// The references standing outside every object: in a frame's defaults,
    /// say.
    pub header: Vec<Reference<'x>>,
    /// The objects defining or holding each identifier.
    holders: FxHashMap<&'x str, Vec<u32>>,
}

impl<'x> Outline<'x> {
    /// The outline of `xml`.
    pub fn of(xml: &'x str) -> Result<Outline<'x>, ScanError> {
        let mut builder = Builder::default();
        scan::walk(xml, &mut builder)?;
        Ok(builder.outline)
    }

    /// The objects defining `id` or holding an element it identifies.
    pub fn holders(&self, id: &str) -> &[u32] {
        self.holders.get(id).map_or(&[], Vec::as_slice)
    }

    /// Every identifier the document defines, of an object or within one.
    pub fn identifiers(&self) -> impl Iterator<Item = &'x str> + '_ {
        self.holders.keys().copied()
    }
}

/// A journey, as an element.
pub fn is_journey(element: &str) -> bool {
    matches!(
        element,
        "ServiceJourney" | "TemplateServiceJourney" | "DatedServiceJourney" | "DeadRun"
    )
}

/// An object kept in every document written: the codespaces.
pub fn is_furniture(element: &str) -> bool {
    element == "Codespace"
}

/// The reference naming what an assignment assigns, for the assignments
/// taken with what they assign.
pub fn subject_of(element: &str) -> Option<&'static str> {
    match element {
        "PassengerStopAssignment" | "TrainStopAssignment" | "DynamicStopAssignment" => {
            Some("ScheduledStopPointRef")
        }
        "DayTypeAssignment" => Some("DayTypeRef"),
        "NoticeAssignment" => Some("NoticedObjectRef"),
        _ => None,
    }
}

/// A place: something in the world that stands for itself, needed or not.
pub fn is_place(element: &str) -> bool {
    element.ends_with("Place")
        || matches!(
            element,
            "Quay" | "Parking" | "PointOfInterest" | "StopPlaceEntrance" | "Entrance"
        )
}

/// The keys naming a stop globally.
const GLOBAL_KEYS: [&str; 2] = ["GlobalID", "SLOID"];

/// The types of private code naming a stop globally: the Dutch stop
/// register's.
const GLOBAL_CODES: [&str; 2] = ["QuayCode", "StopPlaceCode"];

/// What an open element is to the outline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    /// A frame.
    Frame,
    /// A container of a frame.
    Container(u32),
    /// An object of a container.
    Object(u32),
    /// An element within an object.
    Within,
    /// Anything else.
    Other,
}

#[derive(Default)]
struct Builder<'x> {
    outline: Outline<'x>,
    /// The open elements: name and role.
    open: Vec<(&'x str, Role)>,
    /// The identifiers of the open elements within the current object, with
    /// its own first.
    ids: Vec<(&'x str, usize)>,
    /// The call open, and its depth.
    call: Option<usize>,
    /// The point of a pattern open: its identifier and depth.
    point: Option<(&'x str, usize)>,
    /// The text of the last `Key` of a `keyList`.
    key: Option<&'x str>,
    /// The type of the private code open, where it names a stop globally.
    code: Option<&'x str>,
}

impl<'x> Builder<'x> {
    fn current(&self) -> Option<u32> {
        self.open.iter().rev().find_map(|(_, role)| match role {
            Role::Object(object) => Some(*object),
            _ => None,
        })
    }

    fn references(element: &Open<'x>) -> Vec<Reference<'x>> {
        let version = element
            .attributes
            .iter()
            .find(|(name, _, _)| *name == "version")
            .map(|(_, _, at)| at.clone());
        element
            .attributes
            .iter()
            .filter_map(|(name, value, at)| {
                if value.is_empty() {
                    None
                } else if *name == "ref" {
                    Some(Reference {
                        element: element.name,
                        id: value,
                        at: at.clone(),
                        version: version.clone(),
                    })
                } else if name.ends_with("Ref") && *name != "versionRef" {
                    Some(Reference {
                        element: name,
                        id: value,
                        at: at.clone(),
                        version: None,
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

impl<'x> Visitor<'x> for Builder<'x> {
    fn open(&mut self, element: Open<'x>) {
        let parent = self.open.last().map(|(_, role)| *role);
        let id = element
            .attributes
            .iter()
            .find(|(name, value, _)| *name == "id" && !value.is_empty())
            .map(|(_, value, at)| (*value, at.clone()));
        let is_frame = element.name.ends_with("Frame");
        let role = match parent {
            Some(Role::Container(container)) if id.is_some() && !is_frame => {
                Role::Object(container)
            }
            Some(Role::Object(_) | Role::Within) => Role::Within,
            _ if is_frame => {
                if let Some(Role::Container(container)) = parent {
                    self.outline.containers[container as usize].holds_others = true;
                }
                Role::Frame
            }
            Some(Role::Frame) if element.name.starts_with(|c: char| c.is_ascii_lowercase()) => {
                let number = self.outline.containers.len() as u32;
                self.outline.containers.push(Container {
                    element: element.name,
                    range: element.start..element.start,
                    holds_others: false,
                });
                Role::Container(number)
            }
            Some(Role::Container(container)) => {
                self.outline.containers[container as usize].holds_others = true;
                Role::Other
            }
            _ => Role::Other,
        };
        let references = Self::references(&element);
        match role {
            Role::Object(container) => {
                let number = self.outline.objects.len() as u32;
                let (id, id_at) = id.expect("an object has an identifier");
                self.outline.objects.push(Object {
                    element: element.name,
                    id,
                    id_at,
                    version: element.attribute("version"),
                    range: element.start..element.start,
                    container,
                    references,
                    nested: Vec::new(),
                    calls: Vec::new(),
                    points: Vec::new(),
                    keys: Vec::new(),
                });
                self.outline.holders.entry(id).or_default().push(number);
                self.ids.clear();
                self.ids.push((id, element.depth));
                self.open.push((element.name, Role::Object(number)));
                return;
            }
            Role::Within => {
                let number = self.current().expect("within an object");
                let parent_name = self.open.last().map_or("", |(name, _)| *name);
                let object = &mut self.outline.objects[number as usize];
                if element.name == "PrivateCode" {
                    self.code = element
                        .attribute("type")
                        .filter(|kind| GLOBAL_CODES.contains(kind));
                }
                if let Some((id, id_at)) = id.clone() {
                    object.nested.push(Nested {
                        element: element.name,
                        id,
                        id_at,
                    });
                    self.ids.push((id, element.depth));
                    let holders = self.outline.holders.entry(id).or_default();
                    if holders.last() != Some(&number) {
                        holders.push(number);
                    }
                }
                let depth = element.depth;
                if matches!(
                    (parent_name, element.name),
                    ("passingTimes", "TimetabledPassingTime") | ("calls", "Call")
                ) {
                    self.call = Some(depth);
                    object.calls.push(Call {
                        range: element.start..element.start,
                        at: None,
                    });
                }
                if let (Some((id, _)), true) = (
                    id,
                    matches!(
                        element.name,
                        "StopPointInJourneyPattern" | "PointInJourneyPattern"
                    ),
                ) {
                    self.point = Some((id, depth));
                }
                if let Some(reference) = references.first() {
                    if self.call == Some(depth - 1) {
                        let at = match element.name {
                            "StopPointInJourneyPatternRef" => {
                                Some(Called::PatternPoint(reference.id))
                            }
                            "ScheduledStopPointRef" => Some(Called::StopPoint(reference.id)),
                            _ => None,
                        };
                        if let (Some(at), Some(call)) = (at, object.calls.last_mut()) {
                            call.at = Some(at);
                        }
                    }
                    if let Some((point, point_depth)) = self.point {
                        if point_depth + 1 == depth && element.name == "ScheduledStopPointRef" {
                            object.points.push((point, reference.id));
                        }
                    }
                }
                object.references.extend(references);
            }
            Role::Frame | Role::Container(_) | Role::Other => {
                self.outline.header.extend(references);
            }
        }
        self.open.push((element.name, role));
    }

    fn close(&mut self, element: Close<'x>) {
        let (_, role) = self.open.pop().expect("an element closes that opened");
        match role {
            Role::Object(number) => {
                self.outline.objects[number as usize].range = element.range;
                self.ids.clear();
                self.call = None;
                self.point = None;
                self.key = None;
                self.code = None;
            }
            Role::Container(number) => {
                self.outline.containers[number as usize].range = element.range;
            }
            Role::Within => {
                if self
                    .ids
                    .last()
                    .is_some_and(|(_, depth)| *depth == element.depth)
                {
                    self.ids.pop();
                }
                if self.call == Some(element.depth) {
                    self.call = None;
                    let number = self.current().expect("within an object");
                    if let Some(call) = self.outline.objects[number as usize].calls.last_mut() {
                        call.range = element.range.clone();
                    }
                }
                if self.point.is_some_and(|(_, depth)| depth == element.depth) {
                    self.point = None;
                }
                match element.name {
                    "Key" => self.key = element.text,
                    "Value" => {
                        if let (Some(key), Some(value), Some(&(owner, _))) =
                            (self.key.take(), element.text, self.ids.last())
                        {
                            if GLOBAL_KEYS.contains(&key) {
                                let number = self.current().expect("within an object");
                                self.outline.objects[number as usize].keys.push((
                                    owner,
                                    key,
                                    value.trim(),
                                ));
                            }
                        }
                    }
                    "PrivateCode" => {
                        if let (Some(kind), Some(value), Some(&(owner, _))) =
                            (self.code.take(), element.text, self.ids.last())
                        {
                            let number = self.current().expect("within an object");
                            self.outline.objects[number as usize].keys.push((
                                owner,
                                kind,
                                value.trim(),
                            ));
                        }
                    }
                    _ => {}
                }
            }
            Role::Frame | Role::Other => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOCUMENT: &str = r#"<PublicationDelivery version="1.1">
  <dataObjects>
    <CompositeFrame id="f" version="1">
      <FrameDefaults><DefaultCodespaceRef ref="cs"/></FrameDefaults>
      <frames>
        <SiteFrame id="sf" version="1">
          <stopPlaces>
            <StopPlace id="sp" version="1">
              <keyList><KeyValue><Key>GlobalID</Key><Value>de:1:2</Value></KeyValue></keyList>
              <quays>
                <Quay id="q" version="1">
                  <keyList><KeyValue><Key>SLOID</Key><Value>ch:1:sloid:2:1</Value></KeyValue></keyList>
                </Quay>
              </quays>
            </StopPlace>
          </stopPlaces>
        </SiteFrame>
        <TimetableFrame id="tf" version="1">
          <vehicleJourneys>
            <ServiceJourney id="j" version="1" responsibilitySetRef="rs">
              <JourneyPatternRef ref="p" version="1"/>
              <passingTimes>
                <TimetabledPassingTime id="t1" version="1">
                  <StopPointInJourneyPatternRef ref="pp1" version="1"/>
                </TimetabledPassingTime>
              </passingTimes>
            </ServiceJourney>
          </vehicleJourneys>
        </TimetableFrame>
      </frames>
    </CompositeFrame>
  </dataObjects>
</PublicationDelivery>"#;

    #[test]
    fn objects_stand_in_containers_with_their_references_and_what_they_hold() {
        let outline = Outline::of(DOCUMENT).expect("an outline");
        let elements: Vec<(&str, &str)> = outline
            .objects
            .iter()
            .map(|object| (object.element, object.id))
            .collect();
        assert_eq!(elements, [("StopPlace", "sp"), ("ServiceJourney", "j")]);
        let containers: Vec<&str> = outline.containers.iter().map(|c| c.element).collect();
        assert_eq!(containers, ["frames", "stopPlaces", "vehicleJourneys"]);
        assert!(outline.containers[0].holds_others);

        let place = &outline.objects[0];
        let nested: Vec<(&str, &str, &str)> = place
            .nested
            .iter()
            .map(|nested| (nested.element, nested.id, &DOCUMENT[nested.id_at.clone()]))
            .collect();
        assert_eq!(nested, [("Quay", "q", "q")]);
        assert_eq!(
            place.keys,
            [
                ("sp", "GlobalID", "de:1:2"),
                ("q", "SLOID", "ch:1:sloid:2:1")
            ]
        );
        assert_eq!(outline.holders("q"), [0]);
        assert!(DOCUMENT[place.range.clone()].starts_with("<StopPlace id=\"sp\""));
        assert!(DOCUMENT[place.range.clone()].ends_with("</StopPlace>"));

        let journey = &outline.objects[1];
        let references: Vec<(&str, &str)> = journey
            .references
            .iter()
            .map(|reference| (reference.element, reference.id))
            .collect();
        assert_eq!(
            references,
            [
                ("responsibilitySetRef", "rs"),
                ("JourneyPatternRef", "p"),
                ("StopPointInJourneyPatternRef", "pp1")
            ]
        );
        assert_eq!(&DOCUMENT[journey.references[1].at.clone()], "p");
        assert_eq!(journey.calls.len(), 1);
        assert_eq!(journey.calls[0].at, Some(Called::PatternPoint("pp1")));
        assert!(DOCUMENT[journey.calls[0].range.clone()].starts_with("<TimetabledPassingTime"));
        assert_eq!(&DOCUMENT[journey.id_at.clone()], "j");

        let header: Vec<&str> = outline.header.iter().map(|r| r.id).collect();
        assert_eq!(header, ["cs"]);
    }
}
