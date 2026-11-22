//! Checks the schemas express as identity constraints, made on the crate's own.
//!
//! The 2.0 `should-fail` examples each repeat an identifier at one version in
//! one document, which the publication schema's keys reject. A reader that has
//! no validator to hand can ask for the same finding here.
//!
//! The schemas key the elements of each type apart
//! (`StopPlace_UniqueBy_Id_Version`: the `StopPlace` elements by `@id` and
//! `@version`), and as `xsd:unique` constraints leave out an element lacking
//! either field; the check does the same, by the element's name in the NeTEx
//! namespace. Of the 600 such constraints of the 2.0 publication schema, 19
//! select several types together — a `DayType` with a `FareDayType` —; the
//! check keys those type by type too.

use std::collections::HashMap;
use std::fmt;

use quick_xml::events::Event;
use quick_xml::name::{Namespace, ResolveResult};
use quick_xml::{NsReader, XmlVersion};

use crate::error::{Error, Kind, Location, Result};
use crate::xml::NETEX_NAMESPACE;
use crate::Document;

/// An identifier that occurs more than once at one version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateId {
    /// The element carrying the second occurrence.
    pub element: String,
    /// The identifier.
    pub id: String,
    /// The version.
    pub version: String,
    /// The line of the first occurrence, in the document as written.
    pub first_line: u32,
    /// The line of the second occurrence.
    pub line: u32,
}

impl fmt::Display for DuplicateId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<{}> repeats id {} at version {}: first at line {}, again at line {}",
            self.element, self.id, self.version, self.first_line, self.line
        )
    }
}

/// Every identifier the document states more than once at the same version,
/// by elements of one type.
///
/// The document is written and its tokens scanned, so that elements the crate
/// types and elements it carries through are treated alike. Each further
/// occurrence is one finding.
pub fn duplicate_ids(document: &Document) -> Result<Vec<DuplicateId>> {
    duplicate_ids_in(&document.write())
}

/// Every identifier `xml` states more than once at the same version, by
/// elements of one type in the NeTEx namespace; identifiers and versions are
/// compared as read, their references resolved (`a&amp;b` is `a&#38;b`). A
/// document that is not well-formed is an error, where it stops being so.
pub fn duplicate_ids_in(xml: &str) -> Result<Vec<DuplicateId>> {
    let xml = crate::xml::without_bom(xml);
    let mut reader = NsReader::from_str(xml);
    reader.config_mut().check_end_names = true;
    let mut path: Vec<String> = Vec::new();
    // The line of the first occurrence of each element name, identifier and version.
    let mut seen: HashMap<(String, String, String), u32> = HashMap::new();
    let mut findings = Vec::new();
    let (mut counted_to, mut line) = (0usize, 1u32);
    loop {
        let offset = usize::try_from(reader.buffer_position()).unwrap_or(usize::MAX);
        let failed = |message: String, path: &[String]| {
            Error::at(
                Kind::Xml(message),
                Location::at(xml, offset, path.join("/")),
            )
        };
        let (namespace, event) = match reader.read_resolved_event() {
            Ok(read) => read,
            Err(error) => return Err(failed(error.to_string(), &path)),
        };
        let netex =
            matches!(namespace, ResolveResult::Bound(Namespace(ns)) if ns == NETEX_NAMESPACE);
        let start = match event {
            Event::Start(start) => {
                path.push(start.local_name().as_ref().to_owned());
                start
            }
            Event::Empty(start) => start,
            Event::End(_) => {
                path.pop();
                continue;
            }
            Event::Eof => break,
            _ => continue,
        };
        if !netex {
            continue;
        }
        let mut id = None;
        let mut version = None;
        for attribute in start.attributes() {
            let attribute = attribute.map_err(|error| failed(error.to_string(), &path))?;
            let slot = match attribute.key.as_ref() {
                "id" => &mut id,
                "version" => &mut version,
                _ => continue,
            };
            let value = attribute
                .normalized_value(XmlVersion::Explicit1_0)
                .map_err(|error| failed(error.to_string(), &path))?;
            *slot = Some(value.into_owned());
        }
        let (Some(id), Some(version)) = (id, version) else {
            continue;
        };
        let offset = offset.min(xml.len());
        if offset > counted_to {
            let newlines = memchr::memchr_iter(b'\n', &xml.as_bytes()[counted_to..offset]).count();
            line = line.saturating_add(u32::try_from(newlines).unwrap_or(u32::MAX));
            counted_to = offset;
        }
        let element = start.local_name().as_ref().to_owned();
        let key = (element, id, version);
        match seen.get(&key) {
            Some(&first_line) => {
                let (element, id, version) = key;
                findings.push(DuplicateId {
                    element,
                    id,
                    version,
                    first_line,
                    line,
                });
            }
            None => {
                seen.insert(key, line);
            }
        }
    }
    Ok(findings)
}
