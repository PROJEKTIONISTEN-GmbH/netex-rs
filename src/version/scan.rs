//! Scanning a document's tokens for what a release knows: detection, and the
//! rewrite that leaves out what a release does not know.

use std::fmt;
use std::sync::OnceLock;

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use super::report::{Item, Report};
use super::{table, Release, Written};
use crate::xml::NETEX_NAMESPACE;

/// A piece of evidence that a document is in one release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    /// What was found.
    pub item: Item,
    /// The release that alone knows it.
    pub release: Release,
    /// The line it was found on.
    pub line: u32,
}

impl fmt::Display for Evidence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({} only, line {})",
            self.item, self.release, self.line
        )
    }
}

/// Which release a document is in, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detection {
    /// The publication's `version` attribute, as written.
    pub declared_version: Option<String>,
    /// The release that attribute names.
    pub declared: Option<Release>,
    /// The release the document's own content proves: the one release that
    /// alone knows something the document uses. `None` when the document uses
    /// nothing either release lacks — such a document is valid in both — or
    /// when it uses things from both, which no release accepts whole.
    pub structural: Option<Release>,
    /// What the content proves, at most [`Detection::EVIDENCE_KEPT`] pieces
    /// per release; [`Detection::evidence_count`] counts them all.
    pub evidence: Vec<Evidence>,
    /// How many pieces of evidence were found, per release.
    pub evidence_count: [usize; 2],
}

impl Detection {
    /// How many pieces of evidence per release are kept in full.
    pub const EVIDENCE_KEPT: usize = 16;

    /// The release to read the document as: what its content proves, else
    /// what it declares.
    pub fn release(&self) -> Option<Release> {
        self.structural.or(self.declared)
    }

    /// Whether the content proves both releases at once.
    pub fn is_conflicting(&self) -> bool {
        self.evidence_count.iter().all(|&count| count > 0)
    }
}

/// What one release knows and the other does not, as sorted tables.
struct Only {
    element_names: &'static [&'static str],
    children: &'static [(&'static str, &'static str)],
    attribute_names: &'static [&'static str],
    attributes: &'static [(&'static str, &'static str)],
    tokens: &'static [(&'static str, &'static str)],
}

const ONLY_1X: Only = Only {
    element_names: table::ELEMENT_NAMES_ONLY_IN_1X,
    children: table::CHILDREN_ONLY_IN_1X,
    attribute_names: table::ATTRIBUTE_NAMES_ONLY_IN_1X,
    attributes: table::ATTRIBUTES_ONLY_IN_1X,
    tokens: table::TOKENS_ONLY_IN_1X,
};

const ONLY_20: Only = Only {
    element_names: table::ELEMENT_NAMES_ONLY_IN_20,
    children: table::CHILDREN_ONLY_IN_20,
    attribute_names: table::ATTRIBUTE_NAMES_ONLY_IN_20,
    attributes: table::ATTRIBUTES_ONLY_IN_20,
    tokens: table::TOKENS_ONLY_IN_20,
};

/// What `release` alone knows.
fn only(release: Release) -> &'static Only {
    match release {
        Release::V1 => &ONLY_1X,
        Release::V2 => &ONLY_20,
    }
}

impl Only {
    fn knows_element(&self, parent: &str, child: &str) -> bool {
        self.element_names.binary_search(&child).is_ok()
            || self
                .children
                .binary_search_by(|(p, c)| (*p, *c).cmp(&(parent, child)))
                .is_ok()
    }

    fn knows_attribute(&self, element: &str, attribute: &str) -> bool {
        self.attribute_names.binary_search(&attribute).is_ok()
            || self
                .attributes
                .binary_search_by(|(e, a)| (*e, *a).cmp(&(element, attribute)))
                .is_ok()
    }

    fn knows_token(&self, element: &str, token: &str) -> bool {
        self.tokens
            .binary_search_by(|(e, t)| (*e, *t).cmp(&(element, token)))
            .is_ok()
    }
}

/// The elements whose content is an enumeration token that one release alone
/// knows: the only elements whose character data has to be looked at.
fn token_elements() -> &'static [&'static str] {
    static ELEMENTS: OnceLock<Vec<&'static str>> = OnceLock::new();
    ELEMENTS.get_or_init(|| {
        let mut elements: Vec<&'static str> = table::TOKENS_ONLY_IN_1X
            .iter()
            .chain(table::TOKENS_ONLY_IN_20)
            .map(|(element, _)| *element)
            .collect();
        elements.sort_unstable();
        elements.dedup();
        elements
    })
}

fn is_token_element(local: &str) -> bool {
    token_elements().binary_search(&local).is_ok()
}

/// Which elements are NeTEx elements, by the root's namespace declarations.
struct Namespaces<'i> {
    default_is_netex: bool,
    netex_prefix: Option<&'i str>,
}

impl<'i> Namespaces<'i> {
    fn of_root(input: &'i str, root: &BytesStart<'_>) -> Self {
        let mut namespaces = Namespaces {
            default_is_netex: false,
            netex_prefix: None,
        };
        for attribute in root.attributes().flatten() {
            let key = attribute.key.as_ref();
            if key == "xmlns" {
                namespaces.default_is_netex = attribute.value.as_ref() == NETEX_NAMESPACE;
            } else if let Some(prefix) = key.strip_prefix("xmlns:") {
                if attribute.value.as_ref() == NETEX_NAMESPACE {
                    namespaces.netex_prefix = Some(reborrow(input, prefix));
                }
            }
        }
        namespaces
    }

    /// The local name of a NeTEx element, or `None` for an element in another namespace.
    fn netex_local<'e>(&self, start: &'e BytesStart<'_>) -> Option<&'e str> {
        let (local, prefix) = start.name().decompose();
        let netex = match prefix {
            None => self.default_is_netex,
            Some(prefix) => self.netex_prefix == Some(prefix.into_inner()),
        };
        netex.then(|| local.into_inner())
    }
}

/// The same text, as a slice of `input`.
///
/// Names taken from an event borrow the event; the event borrows the input.
/// Re-slicing the input at the same place gives the name the input's lifetime.
fn reborrow<'i>(input: &'i str, text: &str) -> &'i str {
    let base = input.as_ptr() as usize;
    let here = text.as_ptr() as usize;
    debug_assert!(here >= base && here + text.len() <= base + input.len());
    let at = here - base;
    &input[at..at + text.len()]
}

/// Counts lines as the scan advances, so that each byte is looked at once.
struct Lines<'i> {
    input: &'i str,
    offset: usize,
    line: u32,
}

impl<'i> Lines<'i> {
    fn new(input: &'i str) -> Self {
        Lines {
            input,
            offset: 0,
            line: 1,
        }
    }

    /// The line `offset` is on; offsets must not go backwards.
    fn at(&mut self, offset: usize) -> u32 {
        let offset = offset.min(self.input.len());
        if offset > self.offset {
            let newlines =
                memchr::memchr_iter(b'\n', &self.input.as_bytes()[self.offset..offset]).count();
            self.line = self
                .line
                .saturating_add(u32::try_from(newlines).unwrap_or(u32::MAX));
            self.offset = offset;
        }
        self.line
    }
}

/// Tells which release a document is in.
///
/// The document's tokens are scanned for what one release alone knows; a
/// document that is not well-formed is scanned as far as it goes. Any root
/// element is accepted, so a fragment document can be told apart too.
pub fn detect(xml: &str) -> Detection {
    let xml = crate::xml::without_bom(xml);
    let mut reader = Reader::from_str(xml);
    let mut stack: Vec<&str> = Vec::with_capacity(32);
    let mut namespaces = Namespaces {
        default_is_netex: false,
        netex_prefix: None,
    };
    let mut detection = Detection {
        declared_version: None,
        declared: None,
        structural: None,
        evidence: Vec::new(),
        evidence_count: [0, 0],
    };
    let mut token_text: Option<(&str, String)> = None;
    let mut lines = Lines::new(xml);

    let note = |detection: &mut Detection, item: Item, release: Release, line: u32| {
        let slot = match release {
            Release::V1 => 0,
            Release::V2 => 1,
        };
        detection.evidence_count[slot] += 1;
        if detection.evidence_count[slot] <= Detection::EVIDENCE_KEPT {
            detection.evidence.push(Evidence {
                item,
                release,
                line,
            });
        }
    };

    loop {
        let offset = usize::try_from(reader.buffer_position()).unwrap_or(usize::MAX);
        let Ok(event) = reader.read_event() else {
            break;
        };
        match event {
            Event::Start(ref start) | Event::Empty(ref start) => {
                let empty = matches!(event, Event::Empty(_));
                if stack.is_empty() {
                    namespaces = Namespaces::of_root(xml, start);
                    if let Some(version) = start
                        .attributes()
                        .flatten()
                        .find(|attribute| attribute.key.as_ref() == "version")
                    {
                        let version = version.value.into_owned();
                        detection.declared = Release::declared(&version);
                        detection.declared_version = Some(version);
                    }
                }
                let parent = stack.last().copied().unwrap_or("");
                if let Some(local) = namespaces.netex_local(start) {
                    let local = reborrow(xml, local);
                    let line = lines.at(offset);
                    for release in [Release::V1, Release::V2] {
                        if only(release).knows_element(parent, local) {
                            note(
                                &mut detection,
                                Item::Element {
                                    parent: parent.to_owned(),
                                    child: local.to_owned(),
                                },
                                release,
                                line,
                            );
                        }
                        for attribute in start.attributes().flatten() {
                            let key = attribute.key.as_ref();
                            if key.contains(':') || key == "xmlns" {
                                continue;
                            }
                            if only(release).knows_attribute(local, key) {
                                note(
                                    &mut detection,
                                    Item::Attribute {
                                        element: local.to_owned(),
                                        attribute: key.to_owned(),
                                    },
                                    release,
                                    line,
                                );
                            }
                        }
                    }
                    if !empty && is_token_element(local) {
                        token_text = Some((local, String::new()));
                    }
                    if !empty {
                        stack.push(local);
                    }
                } else if !empty {
                    stack.push("");
                }
            }
            Event::Text(text) => {
                if let Some((_, gathered)) = &mut token_text {
                    gathered.push_str(text.xml10_content().as_ref());
                }
            }
            Event::End(_) => {
                if let Some((element, gathered)) = token_text.take() {
                    let token = gathered.trim();
                    let line = lines.at(offset);
                    for release in [Release::V1, Release::V2] {
                        if only(release).knows_token(element, token) {
                            note(
                                &mut detection,
                                Item::Token {
                                    element: element.to_owned(),
                                    token: token.to_owned(),
                                },
                                release,
                                line,
                            );
                        }
                    }
                }
                stack.pop();
            }
            Event::Eof => break,
            _ => {}
        }
    }

    detection.structural = match detection.evidence_count {
        [0, 0] => None,
        [_, 0] => Some(Release::V1),
        [0, _] => Some(Release::V2),
        _ => None,
    };
    detection
}

/// Writes `xml` for `release`, leaving out every element, attribute and
/// enumeration token that release does not know, and reporting each.
///
/// The publication's `version` attribute is set to the release's. Elements in
/// other namespaces, prefixed attributes and namespace declarations are left
/// as they are. Nothing else changes: the document is copied token by token.
pub fn rewrite(xml: &str, release: Release) -> Written {
    let xml = crate::xml::without_bom(xml);
    let unknown = only(release.other());
    let mut reader = Reader::from_str(xml);
    let mut out = String::with_capacity(xml.len());
    let mut report = Report::new();
    let mut stack: Vec<&str> = Vec::with_capacity(32);
    let mut namespaces = Namespaces {
        default_is_netex: false,
        netex_prefix: None,
    };
    // Depth of the stack at which an element being left out started.
    let mut dropping: Option<usize> = None;
    // An enumerated element being held back until its token is known: the
    // element's name, its start tag, its text so far.
    let mut held: Option<(&str, String, String)> = None;
    let mut is_root = true;
    let mut lines = Lines::new(xml);

    loop {
        let offset = usize::try_from(reader.buffer_position()).unwrap_or(usize::MAX);
        let Ok(event) = reader.read_event() else {
            break;
        };
        let end = usize::try_from(reader.buffer_position()).unwrap_or(usize::MAX);
        let source = &xml[offset.min(xml.len())..end.min(xml.len())];

        match event {
            Event::Start(ref start) | Event::Empty(ref start) => {
                let empty = matches!(event, Event::Empty(_));
                if is_root {
                    namespaces = Namespaces::of_root(xml, start);
                }
                if dropping.is_some() {
                    if !empty {
                        stack.push("");
                    }
                    continue;
                }
                let parent = stack.last().copied().unwrap_or("");
                let Some(local) = namespaces.netex_local(start) else {
                    out.push_str(source);
                    if !empty {
                        stack.push("");
                    }
                    is_root = false;
                    continue;
                };
                let local = reborrow(xml, local);
                let line = lines.at(offset);
                if !is_root && unknown.knows_element(parent, local) {
                    report.record(
                        Item::Element {
                            parent: parent.to_owned(),
                            child: local.to_owned(),
                        },
                        release,
                        line,
                    );
                    if !empty {
                        dropping = Some(stack.len());
                        stack.push(local);
                    }
                    continue;
                }
                let tag = start_tag(
                    start,
                    local,
                    empty,
                    is_root,
                    release,
                    unknown,
                    &mut report,
                    line,
                );
                if !empty && is_token_element(local) {
                    held = Some((local, tag, String::new()));
                } else {
                    out.push_str(&tag);
                }
                if !empty {
                    stack.push(local);
                }
                is_root = false;
            }
            Event::End(_) => {
                let local = stack.pop().unwrap_or("");
                if let Some(depth) = dropping {
                    if stack.len() == depth {
                        dropping = None;
                    }
                    continue;
                }
                if let Some((element, tag, text)) = held.take() {
                    let token = text.trim();
                    if unknown.knows_token(element, token) {
                        report.record(
                            Item::Token {
                                element: element.to_owned(),
                                token: token.to_owned(),
                            },
                            release,
                            lines.at(offset),
                        );
                        continue;
                    }
                    out.push_str(&tag);
                    out.push_str(&text);
                }
                let _ = local;
                out.push_str(source);
            }
            Event::Text(ref text) => {
                if dropping.is_some() {
                    continue;
                }
                match &mut held {
                    Some((_, _, gathered)) => gathered.push_str(text.xml10_content().as_ref()),
                    None => out.push_str(source),
                }
            }
            Event::Eof => break,
            _ => {
                if dropping.is_none() {
                    match &mut held {
                        Some((_, _, gathered)) => gathered.push_str(source),
                        None => out.push_str(source),
                    }
                }
            }
        }
    }

    Written { xml: out, report }
}

/// The start tag of a kept element, with the attributes the release does not
/// know left out and, on a `PublicationDelivery` root, the `version` set to
/// the release's. A document rooted at any other element — a `<Locale>`
/// published on its own — keeps its root as it is: there, `version` is the
/// entity's version, or is not allowed at all.
#[allow(clippy::too_many_arguments)]
fn start_tag(
    start: &BytesStart<'_>,
    local: &str,
    empty: bool,
    is_root: bool,
    release: Release,
    unknown: &Only,
    report: &mut Report,
    line: u32,
) -> String {
    let mut tag = String::with_capacity(start.len() + 4);
    tag.push('<');
    tag.push_str(start.name().as_ref());
    let is_publication = is_root && local == "PublicationDelivery";
    let mut version_written = false;
    for attribute in start.attributes().flatten() {
        let key = attribute.key.as_ref();
        if is_publication && key == "version" {
            tag.push_str(" version=\"");
            tag.push_str(release.declared_version());
            tag.push('"');
            version_written = true;
            continue;
        }
        if !key.contains(':') && key != "xmlns" && unknown.knows_attribute(local, key) {
            report.record(
                Item::Attribute {
                    element: local.to_owned(),
                    attribute: key.to_owned(),
                },
                release,
                line,
            );
            continue;
        }
        tag.push(' ');
        tag.push_str(key);
        tag.push_str("=\"");
        tag.push_str(attribute.value.as_ref());
        tag.push('"');
    }
    if is_publication && !version_written {
        tag.push_str(" version=\"");
        tag.push_str(release.declared_version());
        tag.push('"');
    }
    tag.push_str(if empty { "/>" } else { ">" });
    tag
}
