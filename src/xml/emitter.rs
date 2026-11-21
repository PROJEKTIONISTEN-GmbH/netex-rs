//! Writes a document.

use super::{Attribute, Unprocessed, NETEX_NAMESPACE};

/// Writes XML into a string, indenting as it goes.
///
/// Typed elements are written through it; fragments carried through untyped are
/// appended as they were read. What it writes is XML 1.0 whatever the values
/// hold: a carriage return — and in an attribute a tab and a line feed —, which
/// a parser would fold, is written as a character reference, and a character
/// XML 1.0 forbids, which no document can hold, as U+FFFD, the replacement
/// character.
#[derive(Debug)]
pub struct Emitter {
    out: String,
    depth: usize,
    /// Whether the element being written has had a child written, which decides
    /// whether its end tag goes on a new line.
    open_has_children: Vec<bool>,
    /// Whether the next element written declares the NeTEx namespace where it
    /// does not itself.
    declare_netex: bool,
}

impl Emitter {
    /// An emitter with the XML declaration written.
    pub fn new() -> Self {
        let mut out = String::with_capacity(4096);
        out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        Emitter {
            out,
            depth: 0,
            open_has_children: Vec::with_capacity(32),
            declare_netex: false,
        }
    }

    /// The emitter, declaring the NeTEx namespace on the first element it
    /// writes where that element does not declare it itself: for an element
    /// written as a document of its own.
    pub fn declaring_netex(mut self) -> Self {
        self.declare_netex = true;
        self
    }

    /// An emitter with a buffer of the given capacity and the XML declaration written.
    pub fn with_capacity(capacity: usize) -> Self {
        let mut emitter = Self::new();
        emitter.out.reserve(capacity);
        emitter
    }

    /// The document written so far.
    pub fn finish(self) -> String {
        self.out
    }

    fn newline(&mut self) {
        self.out.push('\n');
        for _ in 0..self.depth {
            self.out.push_str("  ");
        }
    }

    fn note_child(&mut self) {
        if let Some(open) = self.open_has_children.last_mut() {
            *open = true;
        }
    }

    fn open_tag(&mut self, name: &str, attributes: &[(&str, &str)], extra: &[Attribute]) {
        self.out.push('<');
        self.out.push_str(name);
        if std::mem::take(&mut self.declare_netex)
            && !attributes.iter().any(|(key, _)| *key == "xmlns")
            && !extra.iter().any(|attribute| &*attribute.name == "xmlns")
        {
            self.attribute("xmlns", NETEX_NAMESPACE);
        }
        for (key, value) in attributes {
            self.attribute(key, value);
        }
        for attribute in extra {
            self.attribute(&attribute.name, &attribute.value);
        }
    }

    fn attribute(&mut self, key: &str, value: &str) {
        self.out.push(' ');
        self.out.push_str(key);
        self.out.push_str("=\"");
        escape(&mut self.out, value, Escaped::Attribute);
        self.out.push('"');
    }

    /// Starts an element that will hold children.
    pub fn start(&mut self, name: &str, attributes: &[(&str, &str)], extra: &[Attribute]) {
        self.note_child();
        if self.depth > 0 || !self.open_has_children.is_empty() {
            self.newline();
        }
        self.open_tag(name, attributes, extra);
        self.out.push('>');
        self.depth += 1;
        self.open_has_children.push(false);
    }

    /// Ends the element started last.
    ///
    /// # Panics
    ///
    /// If no element is open: every `end` follows its [`start`](Self::start).
    pub fn end(&mut self, name: &str) {
        let Some(had_children) = self.open_has_children.pop() else {
            panic!("Emitter::end(\"{name}\"): no element is open");
        };
        self.depth -= 1;
        if had_children {
            self.newline();
        }
        self.out.push_str("</");
        self.out.push_str(name);
        self.out.push('>');
    }

    /// Writes an element with character data and no children.
    pub fn leaf(
        &mut self,
        name: &str,
        attributes: &[(&str, &str)],
        extra: &[Attribute],
        text: &str,
    ) {
        self.note_child();
        self.newline();
        self.open_tag(name, attributes, extra);
        if text.is_empty() {
            self.out.push_str("/>");
        } else {
            self.out.push('>');
            escape(&mut self.out, text, Escaped::Text);
            self.out.push_str("</");
            self.out.push_str(name);
            self.out.push('>');
        }
    }

    /// Writes character data into the element started last, before its
    /// children: the text of an element of mixed content.
    pub fn text(&mut self, text: &str) {
        escape(&mut self.out, text, Escaped::Text);
    }

    /// Writes a fragment as it was read.
    pub fn raw(&mut self, fragment: &Unprocessed) {
        self.note_child();
        self.newline();
        self.out.push_str(&fragment.xml);
    }
}

impl Default for Emitter {
    fn default() -> Self {
        Self::new()
    }
}

/// Appends `value` to `out` as an attribute value between double quotes is
/// written ([`Emitter`]).
pub(crate) fn escape_attribute(out: &mut String, value: &str) {
    escape(out, value, Escaped::Attribute);
}

/// Where a value is written.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Escaped {
    Text,
    Attribute,
}

/// Appends `value` to `out` as XML writes it: the characters markup gives a
/// meaning as references; a carriage return as one, and in an attribute a
/// tab and a line feed too, which a parser reads as a line feed or a space;
/// a character XML 1.0 forbids as U+FFFD.
fn escape(out: &mut String, value: &str, place: Escaped) {
    let special = |character: char| match character {
        '&' | '<' | '>' | '"' | '\'' | '\r' => true,
        '\t' | '\n' => place == Escaped::Attribute,
        _ => !super::cursor::is_xml_char(character),
    };
    let mut rest = value;
    while let Some(at) = rest.find(special) {
        out.push_str(&rest[..at]);
        let character = rest[at..].chars().next().expect("a character is found");
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\r' => out.push_str("&#13;"),
            '\t' => out.push_str("&#9;"),
            '\n' => out.push_str("&#10;"),
            _ => out.push(char::REPLACEMENT_CHARACTER),
        }
        rest = &rest[at + character.len_utf8()..];
    }
    out.push_str(rest);
}
