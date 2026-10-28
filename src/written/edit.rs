//! Changes to a document, applied as it is written out: elements left out,
//! values replaced — every other byte as it was.

use std::io::{self, Write};
use std::ops::Range;

/// The changes to one document.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Edits {
    /// Each change: where, and the text replacing it — none for an element
    /// left out, together with the white space before it.
    changes: Vec<(Range<usize>, Option<String>)>,
}

impl Edits {
    /// Leaves out the element at `range` and the white space before it.
    pub fn remove(&mut self, range: Range<usize>) {
        self.changes.push((range, None));
    }

    /// Replaces the text at `range` — an attribute's value — by `text`.
    pub fn replace(&mut self, range: Range<usize>, text: String) {
        self.changes.push((range, Some(text)));
    }

    /// Whether there is no change.
    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    /// Writes `xml` changed. A change within an element left out is dropped
    /// with it.
    pub fn write(mut self, xml: &str, out: &mut impl Write) -> io::Result<()> {
        self.changes
            .sort_by(|(a, _), (b, _)| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));
        let bytes = xml.as_bytes();
        let mut written = 0;
        for (range, text) in self.changes {
            if range.start < written {
                continue;
            }
            let start = match text {
                Some(_) => range.start,
                None => {
                    let mut start = range.start;
                    while start > written && bytes[start - 1].is_ascii_whitespace() {
                        start -= 1;
                    }
                    start
                }
            };
            out.write_all(&bytes[written..start])?;
            if let Some(text) = text {
                out.write_all(text.as_bytes())?;
            }
            written = range.end;
        }
        out.write_all(&bytes[written..])
    }
}

/// `text` as an attribute value: the characters XML gives a meaning there
/// written as references.
pub fn attribute_value(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&apos;"),
            _ => escaped.push(c),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    fn written(xml: &str, edits: Edits) -> String {
        let mut out = Vec::new();
        edits.write(xml, &mut out).expect("written");
        String::from_utf8(out).expect("UTF-8")
    }

    #[test]
    fn an_element_goes_with_the_white_space_before_it_and_values_are_replaced() {
        let xml = "<a>\n  <b ref=\"x\"/>\n  <c/>\n</a>";
        let mut edits = Edits::default();
        edits.remove(21..25);
        edits.replace(14..15, attribute_value("y&z"));
        assert_eq!(written(xml, edits), "<a>\n  <b ref=\"y&amp;z\"/>\n</a>");
    }

    #[test]
    fn a_change_within_an_element_left_out_goes_with_it() {
        let xml = "<a><b ref=\"x\"/></a>";
        let mut edits = Edits::default();
        edits.replace(11..12, "y".to_owned());
        edits.remove(3..15);
        assert_eq!(written(xml, edits), "<a></a>");
        assert_eq!(written(xml, Edits::default()), xml);
    }
}
