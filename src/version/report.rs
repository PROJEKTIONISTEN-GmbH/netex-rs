//! What a document loses when written for a release.

use std::collections::BTreeMap;
use std::fmt;

use super::Release;

/// One thing a release does not know.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Item {
    /// A child element under a parent.
    Element {
        /// The parent's local name.
        parent: String,
        /// The child's local name.
        child: String,
    },
    /// An attribute on an element.
    Attribute {
        /// The element's local name.
        element: String,
        /// The attribute's name.
        attribute: String,
    },
    /// An enumeration token as the content of an element.
    Token {
        /// The element's local name.
        element: String,
        /// The token.
        token: String,
    },
}

impl fmt::Display for Item {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Item::Element { parent, child } => write!(f, "<{child}> under <{parent}>"),
            Item::Attribute { element, attribute } => write!(f, "@{attribute} on <{element}>"),
            Item::Token { element, token } => write!(f, "the token {token:?} in <{element}>"),
        }
    }
}

/// One thing left out of a document, and how often.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Loss {
    /// What was left out.
    pub item: Item,
    /// The release that does not know it.
    pub unknown_to: Release,
    /// How many times it was left out.
    pub occurrences: usize,
    /// The line of the first occurrence, in the document as it was written
    /// before anything was left out.
    pub first_line: u32,
}

impl fmt::Display for Loss {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} is not in NeTEx {}: left out {} time{}, first at line {}",
            self.item,
            self.unknown_to,
            self.occurrences,
            if self.occurrences == 1 { "" } else { "s" },
            self.first_line
        )
    }
}

/// The account of one write: everything the target release did not know,
/// keyed by what it was, counted, and placed.
///
/// The pattern is the loss report of the VDV→SIRI bridge in `vdv453-rs`: a
/// list of a hundred stops with a 2.0-only attribute each is one entry, counted
/// a hundred times, so that the report stays readable and nothing is dropped
/// without a line in it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    losses: BTreeMap<Item, Loss>,
}

impl Report {
    /// An empty report.
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether everything the document held reached the written form.
    pub fn is_lossless(&self) -> bool {
        self.losses.is_empty()
    }

    /// Everything left out, in a stable order.
    pub fn losses(&self) -> impl Iterator<Item = &Loss> {
        self.losses.values()
    }

    /// How many things were left out, occurrences summed.
    pub fn occurrences(&self) -> usize {
        self.losses.values().map(|loss| loss.occurrences).sum()
    }

    pub(crate) fn record(&mut self, item: Item, unknown_to: Release, line: u32) {
        self.losses
            .entry(item.clone())
            .and_modify(|loss| loss.occurrences += 1)
            .or_insert(Loss {
                item,
                unknown_to,
                occurrences: 1,
                first_line: line,
            });
    }
}
