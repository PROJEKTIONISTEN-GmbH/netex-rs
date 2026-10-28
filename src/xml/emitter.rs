//! Writes a document.

use std::fmt::Write as _;

use super::{Attribute, Unprocessed};

/// Writes XML into a string, indenting as it goes.
///
/// Typed elements are written through it; fragments carried through untyped are
/// appended as they were read.
#[derive(Debug)]
pub struct Emitter {
    out: String,
    depth: usize,
    /// Whether the element being written has had a child written, which decides
    /// whether its end tag goes on a new line.
    open_has_children: Vec<bool>,
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
        }
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
        for (key, value) in attributes {
            let _ = write!(self.out, " {key}=\"{}\"", quick_xml::escape::escape(*value));
        }
        for attribute in extra {
            let _ = write!(
                self.out,
                " {}=\"{}\"",
                attribute.name,
                quick_xml::escape::escape(&*attribute.value)
            );
        }
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
    pub fn end(&mut self, name: &str) {
        self.depth -= 1;
        let had_children = self.open_has_children.pop().unwrap_or(false);
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
            self.out.push_str(&quick_xml::escape::escape(text));
            self.out.push_str("</");
            self.out.push_str(name);
            self.out.push('>');
        }
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
