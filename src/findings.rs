//! What a rewrite of deliveries reports — the merge ([`crate::merge`]), a cut of a delivery (the
//! `netex` command's `extract`) —: each class of finding with its count and first examples, and the
//! errors that stop it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io;
use std::path::PathBuf;

/// The examples kept of each class.
const EXAMPLES: usize = 5;

/// A class of findings.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Finding {
    /// The class, in kebab case.
    pub class: &'static str,
    /// What it means.
    pub says: &'static str,
    /// How often it was found.
    pub count: u64,
    /// The first examples, in the order of their text.
    pub examples: Vec<String>,
}

/// Findings as they are made, by class.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Findings {
    classes: BTreeMap<&'static str, (&'static str, u64, BTreeSet<String>)>,
}

impl Findings {
    /// Notes a finding of `class`, meaning `says`, about `example`.
    pub fn note(&mut self, class: &'static str, says: &'static str, example: impl Into<String>) {
        let (_, count, examples) = self
            .classes
            .entry(class)
            .or_insert((says, 0, BTreeSet::new()));
        *count += 1;
        examples.insert(example.into());
        if examples.len() > EXAMPLES {
            examples.pop_last();
        }
    }

    /// Adds the findings of `other`.
    pub fn absorb(&mut self, other: Findings) {
        for (class, (says, count, examples)) in other.classes {
            let (_, total, kept) = self
                .classes
                .entry(class)
                .or_insert((says, 0, BTreeSet::new()));
            *total += count;
            kept.extend(examples);
            while kept.len() > EXAMPLES {
                kept.pop_last();
            }
        }
    }

    /// The findings, by class.
    pub fn into_list(self) -> Vec<Finding> {
        self.classes
            .into_iter()
            .map(|(class, (says, count, examples))| Finding {
                class,
                says,
                count,
                examples: examples.into_iter().collect(),
            })
            .collect()
    }
}

/// The findings for a person, one line per class and one per example.
pub fn human(findings: &[Finding]) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    for finding in findings {
        let _ = writeln!(
            out,
            "{} × {}: {}",
            finding.count, finding.class, finding.says
        );
        for example in &finding.examples {
            let _ = writeln!(out, "    {example}");
        }
    }
    out
}

/// Why a part or a merge could not be written.
#[derive(Debug)]
pub enum Error {
    /// A delivery could not be read.
    Read(crate::Error),
    /// A document is no well-formed XML.
    Document {
        /// The document.
        document: String,
        /// What is wrong with it.
        message: String,
    },
    /// A document could not be written.
    Write {
        /// Where.
        path: PathBuf,
        /// Why.
        error: io::Error,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Read(error) => write!(f, "{error}"),
            Error::Document { document, message } => write!(f, "{document}: {message}"),
            Error::Write { path, error } => write!(f, "{}: {error}", path.display()),
        }
    }
}

impl std::error::Error for Error {}

impl From<crate::Error> for Error {
    fn from(error: crate::Error) -> Self {
        Error::Read(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_class_counts_everything_and_keeps_its_first_examples() {
        let mut findings = Findings::default();
        for example in ["g", "f", "e", "d", "c", "b"] {
            findings.note("cut", "cut", example);
        }
        let mut other = Findings::default();
        other.note("cut", "cut", "a");
        other.note("left-out", "left out", "x");
        findings.absorb(other);
        let list = findings.into_list();
        assert_eq!(list[0].count, 7);
        assert_eq!(list[0].examples, ["a", "b", "c", "d", "e"]);
        assert_eq!((list[1].class, list[1].count), ("left-out", 1));
    }
}
