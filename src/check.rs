//! Checks the schemas express as identity constraints, made on the crate's own.
//!
//! The 2.0 `should-fail` examples each repeat an identifier at one version in
//! one document, which the publication schema's keys reject. A reader that has
//! no validator to hand can ask for the same finding here.

use std::collections::HashMap;
use std::fmt;

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::Document;

/// An identifier that occurs more than once at one version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateId {
    /// The element carrying the second occurrence.
    pub element: String,
    /// The identifier.
    pub id: String,
    /// The version, if the element states one.
    pub version: Option<String>,
    /// The line of the first occurrence, in the document as written.
    pub first_line: u32,
    /// The line of the second occurrence.
    pub line: u32,
}

impl fmt::Display for DuplicateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}> repeats id {}", self.element, self.id)?;
        if let Some(version) = &self.version {
            write!(f, " at version {version}")?;
        }
        write!(
            f,
            ": first at line {}, again at line {}",
            self.first_line, self.line
        )
    }
}

/// Every identifier the document states more than once at the same version.
///
/// The document is written and its tokens scanned, so that elements the crate
/// types and elements it carries through are treated alike. Each further
/// occurrence is one finding.
pub fn duplicate_ids(document: &Document) -> Vec<DuplicateId> {
    let xml = document.write();
    duplicate_ids_in(&xml)
}

/// Every identifier `xml` states more than once at the same version.
pub fn duplicate_ids_in(xml: &str) -> Vec<DuplicateId> {
    let xml = crate::xml::without_bom(xml);
    let mut reader = Reader::from_str(xml);
    let mut seen: HashMap<(String, Option<String>), u32> = HashMap::new();
    let mut findings = Vec::new();
    let (mut counted_to, mut line) = (0usize, 1u32);
    loop {
        let offset = usize::try_from(reader.buffer_position()).unwrap_or(usize::MAX);
        let Ok(event) = reader.read_event() else {
            break;
        };
        match event {
            Event::Start(start) | Event::Empty(start) => {
                let mut id = None;
                let mut version = None;
                for attribute in start.attributes().flatten() {
                    match attribute.key.as_ref() {
                        "id" => id = Some(attribute.value.into_owned()),
                        "version" => version = Some(attribute.value.into_owned()),
                        _ => {}
                    }
                }
                let Some(id) = id else { continue };
                let offset = offset.min(xml.len());
                if offset > counted_to {
                    let newlines =
                        memchr::memchr_iter(b'\n', &xml.as_bytes()[counted_to..offset]).count();
                    line = line.saturating_add(u32::try_from(newlines).unwrap_or(u32::MAX));
                    counted_to = offset;
                }
                match seen.get(&(id.clone(), version.clone())) {
                    Some(&first_line) => findings.push(DuplicateId {
                        element: start.local_name().as_ref().to_owned(),
                        id,
                        version,
                        first_line,
                        line,
                    }),
                    None => {
                        seen.insert((id, version), line);
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    findings
}
