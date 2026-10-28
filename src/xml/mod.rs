//! The XML layer: a cursor over the tokens of a document, an emitter that writes
//! one, and the fragment type that carries content through untouched.
//!
//! The reader is streaming and typed by hand, not by serde: for each element it
//! decides, name by name, what becomes a typed value and what is carried through
//! as it was written. That decision is what keeps a document the crate does not
//! fully model from losing content.

mod cursor;
mod emitter;

pub use cursor::{Child, Content, Cursor, NETEX_NAMESPACE};
pub use emitter::Emitter;

/// The document without its leading byte-order marks.
///
/// The tokenizer skips one mark but reports offsets as if it were one byte
/// long, so every slice taken by offset would then fall inside it; the
/// scanners and the cursor therefore work on the text behind the marks — all
/// of them, so that none is left for the tokenizer to skip.
pub(crate) fn without_bom(mut xml: &str) -> &str {
    while let Some(rest) = xml.strip_prefix('\u{feff}') {
        xml = rest;
    }
    xml
}

/// Limits a document must stay within to be read.
///
/// A document is read from memory, so its total size is bounded by the caller;
/// these bound what the reader itself does with it: how deep it recurses, how
/// much character data it gathers into one value, and how large a fragment it
/// carries through whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Deepest element nesting accepted. The reader recurses once per typed
    /// element, so this bounds its stack. NeTEx documents are about fifteen deep.
    pub max_depth: usize,
    /// Longest run of character data accepted in one element, in bytes.
    pub max_text: usize,
    /// Most attributes accepted on one element.
    pub max_attributes: usize,
    /// Largest fragment carried through untyped, in bytes.
    pub max_fragment: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_depth: 128,
            max_text: 16 * 1024 * 1024,
            max_attributes: 256,
            max_fragment: 1024 * 1024 * 1024,
        }
    }
}

/// An attribute the crate does not model, kept as it was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    /// The qualified name as written, prefix included: `xmlns:xsi`, `xsi:schemaLocation`.
    pub name: Box<str>,
    /// The value, unescaped.
    pub value: Box<str>,
}

/// An element the crate does not model, carried through as it was written.
///
/// The fragment is the element's source text, start tag to end tag, and is
/// written back byte for byte. Its position among the siblings it was read
/// with is kept so that the element is written back where it stood.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unprocessed {
    /// The element's local name, for reports.
    pub name: Box<str>,
    /// The element's index among its parent's children when it was read.
    pub position: u32,
    /// The element's source text, start tag to end tag.
    pub xml: Box<str>,
}

impl Unprocessed {
    /// A fragment to be written after every typed sibling.
    ///
    /// For content added to a document rather than read from one: the position
    /// is past any child a typed element can have.
    pub fn new(name: impl Into<Box<str>>, xml: impl Into<Box<str>>) -> Self {
        Unprocessed {
            name: name.into(),
            position: u32::MAX,
            xml: xml.into(),
        }
    }
}
