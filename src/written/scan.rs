//! The elements of a document as written: where each starts and ends, its
//! attributes and where their values stand — what `extract` and `merge` need
//! to cut a document and rewrite references in it without touching anything
//! else. A walk hands each element to a [`Visitor`] when it opens and when it
//! closes, holding nothing of the document itself.

use std::ops::Range;

use quick_xml::events::Event;
use quick_xml::Reader;

/// An attribute: its local name, its value as written, and where the value
/// stands in the document, without its quotes.
pub type Attribute<'x> = (&'x str, &'x str, Range<usize>);

/// An element as it opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Open<'x> {
    /// The local name, without a prefix.
    pub name: &'x str,
    /// Where its start tag starts.
    pub start: usize,
    /// How deep it stands: the root is 0.
    pub depth: usize,
    /// Its attributes.
    pub attributes: Vec<Attribute<'x>>,
}

impl<'x> Open<'x> {
    /// The value of the attribute `name`, as written.
    pub fn attribute(&self, name: &str) -> Option<&'x str> {
        self.attributes
            .iter()
            .find(|(attribute, _, _)| *attribute == name)
            .map(|(_, value, _)| *value)
    }
}

/// An element as it closes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Close<'x> {
    /// The local name, without a prefix.
    pub name: &'x str,
    /// From the `<` of its start tag to the `>` of its end tag.
    pub range: Range<usize>,
    /// How deep it stands: the root is 0.
    pub depth: usize,
    /// The character data it holds, where it holds no element: as written,
    /// entity references not resolved.
    pub text: Option<&'x str>,
}

/// What a walk hands each element to.
pub trait Visitor<'x> {
    /// An element opens.
    fn open(&mut self, element: Open<'x>);
    /// An element closes, every element within it closed before.
    fn close(&mut self, element: Close<'x>);
}

/// Why a document cannot be walked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanError {
    /// Where in the document.
    pub offset: usize,
    /// What went wrong.
    pub message: String,
}

impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "at byte {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for ScanError {}

/// An element open in the walk: its name, where it starts, and the range of
/// the character data it holds and whether it holds an element.
struct Opened<'x> {
    name: &'x str,
    start: usize,
    text: Option<Range<usize>>,
    holds_elements: bool,
}

/// Walks `xml`, handing every element to `visitor` as it opens and closes. A
/// document declaring a DTD or using an entity other than the five XML
/// predefines is refused, as netex-rs refuses it.
pub fn walk<'x>(xml: &'x str, visitor: &mut impl Visitor<'x>) -> Result<(), ScanError> {
    // The reader skips a byte-order mark and counts from after it — one
    // mark: a second is a character of the document.
    let mark = if xml.starts_with('\u{feff}') { 3 } else { 0 };
    let mut reader = Reader::from_str(xml);
    reader.config_mut().check_end_names = true;
    let position = |reader: &Reader<&[u8]>| {
        mark + usize::try_from(reader.buffer_position()).expect("a document in memory")
    };
    let mut open: Vec<Opened<'x>> = Vec::new();
    loop {
        let start = position(&reader);
        let event = reader.read_event().map_err(|error| ScanError {
            offset: start,
            message: error.to_string(),
        })?;
        let end = position(&reader);
        match event {
            Event::Start(_) | Event::Empty(_) => {
                let (name, attributes) = parse_tag(&xml[start..end], start)?;
                if let Some(parent) = open.last_mut() {
                    parent.holds_elements = true;
                }
                visitor.open(Open {
                    name,
                    start,
                    depth: open.len(),
                    attributes,
                });
                if matches!(event, Event::Empty(_)) {
                    visitor.close(Close {
                        name,
                        range: start..end,
                        depth: open.len(),
                        text: None,
                    });
                } else {
                    open.push(Opened {
                        name,
                        start,
                        text: None,
                        holds_elements: false,
                    });
                }
            }
            Event::End(_) => {
                let opened = open.pop().ok_or_else(|| ScanError {
                    offset: start,
                    message: "an end tag without a start tag".to_owned(),
                })?;
                let text = opened
                    .text
                    .filter(|_| !opened.holds_elements)
                    .map(|range| &xml[range]);
                visitor.close(Close {
                    name: opened.name,
                    range: opened.start..end,
                    depth: open.len(),
                    text,
                });
            }
            Event::GeneralRef(reference) => {
                let known = reference.resolve_char_ref().is_ok_and(|c| c.is_some())
                    || matches!(reference.as_ref(), "amp" | "lt" | "gt" | "quot" | "apos");
                if !known {
                    return Err(ScanError {
                        offset: start,
                        message: format!("the entity &{}; is not expanded", reference.as_ref()),
                    });
                }
                extend_text(&mut open, start..end);
            }
            Event::Text(_) | Event::CData(_) => extend_text(&mut open, start..end),
            Event::DocType(_) => {
                return Err(ScanError {
                    offset: start,
                    message: "a document type declaration is not read".to_owned(),
                })
            }
            Event::Eof => break,
            Event::Decl(_) | Event::PI(_) | Event::Comment(_) => {}
        }
    }
    match open.last() {
        Some(opened) => Err(ScanError {
            offset: opened.start,
            message: format!("<{}> is never closed", opened.name),
        }),
        None => Ok(()),
    }
}

/// Adds `range` to the character data of the element open last.
fn extend_text(open: &mut [Opened<'_>], range: Range<usize>) {
    if let Some(parent) = open.last_mut() {
        let text = parent.text.get_or_insert(range.start..range.start);
        text.end = range.end;
    }
}

/// The local name and the attributes of a start tag `tag`, which starts at
/// `offset` in the document.
fn parse_tag(tag: &str, offset: usize) -> Result<(&str, Vec<Attribute<'_>>), ScanError> {
    let bytes = tag.as_bytes();
    let error = |at: usize, message: &str| ScanError {
        offset: offset + at,
        message: message.to_owned(),
    };
    let name_end = tag[1..]
        .find(|c: char| c.is_ascii_whitespace() || c == '>' || c == '/')
        .map_or(tag.len(), |at| at + 1);
    let name = local(&tag[1..name_end]);
    let mut attributes = Vec::new();
    let mut at = name_end;
    loop {
        while at < bytes.len() && bytes[at].is_ascii_whitespace() {
            at += 1;
        }
        if at >= bytes.len() || bytes[at] == b'>' || bytes[at] == b'/' {
            break;
        }
        let key_start = at;
        while at < bytes.len() && bytes[at] != b'=' && !bytes[at].is_ascii_whitespace() {
            at += 1;
        }
        let key = &tag[key_start..at];
        while at < bytes.len() && (bytes[at].is_ascii_whitespace() || bytes[at] == b'=') {
            at += 1;
        }
        let quote = *bytes
            .get(at)
            .ok_or_else(|| error(at, "an attribute without a value"))?;
        if quote != b'"' && quote != b'\'' {
            return Err(error(at, "an attribute value without quotes"));
        }
        let value_start = at + 1;
        let value_end = tag[value_start..]
            .find(quote as char)
            .map(|length| value_start + length)
            .ok_or_else(|| error(at, "an attribute value never closed"))?;
        attributes.push((
            local(key),
            &tag[value_start..value_end],
            offset + value_start..offset + value_end,
        ));
        at = value_end + 1;
    }
    Ok((name, attributes))
}

/// A qualified name without its prefix.
fn local(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every event as text.
    #[derive(Default)]
    struct Events(Vec<String>);

    impl<'x> Visitor<'x> for Events {
        fn open(&mut self, element: Open<'x>) {
            let attributes: Vec<String> = element
                .attributes
                .iter()
                .map(|(name, value, range)| format!("{name}={value}@{}", range.start))
                .collect();
            self.0.push(format!(
                "open {} at {} depth {} [{}]",
                element.name,
                element.start,
                element.depth,
                attributes.join(" ")
            ));
        }

        fn close(&mut self, element: Close<'x>) {
            self.0.push(format!(
                "close {} {:?} depth {} {:?}",
                element.name, element.range, element.depth, element.text
            ));
        }
    }

    #[test]
    fn each_element_opens_and_closes_with_its_range_its_attributes_and_its_text() {
        let xml = r#"<root xmlns:gml="g"><gml:pos srsName='EPSG:2056'>1 2</gml:pos><Ref ref="a&amp;b" version="1"/></root>"#;
        let mut events = Events::default();
        walk(xml, &mut events).expect("walks");
        assert_eq!(
            events.0,
            [
                "open root at 0 depth 0 [gml=g@17]",
                "open pos at 20 depth 1 [srsName=EPSG:2056@38]",
                "close pos 20..62 depth 1 Some(\"1 2\")",
                "open Ref at 62 depth 1 [ref=a&amp;b@72 version=1@90]",
                "close Ref 62..94 depth 1 None",
                "close root 0..101 depth 0 None",
            ]
        );
        assert_eq!(&xml[72..79], "a&amp;b");
    }

    #[test]
    fn positions_count_a_byte_order_mark() {
        let xml = "\u{feff}<?xml version=\"1.0\"?>\r\n<a b=\"c\"/>";
        let mut events = Events::default();
        walk(xml, &mut events).expect("walks");
        assert_eq!(events.0[0], "open a at 26 depth 0 [b=c@32]");
        assert_eq!(&xml[32..33], "c");
        // A second mark is a character before the root, not a mark.
        let _ = walk("\u{feff}\u{feff}<a/>", &mut Events::default());
    }

    #[test]
    fn what_netex_rs_refuses_the_walk_refuses() {
        let mut events = Events::default();
        assert!(walk("<!DOCTYPE a><a/>", &mut events).is_err());
        assert!(walk("<a>&custom;</a>", &mut events).is_err());
        assert!(walk("<a><b></a>", &mut events).is_err());
        assert!(walk("<a>", &mut events).is_err());
    }
}
