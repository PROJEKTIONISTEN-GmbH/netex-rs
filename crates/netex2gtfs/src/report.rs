//! What a conversion did: the rows it wrote, every decision it took, and what
//! of the delivery the feed does not carry.

use std::collections::BTreeMap;

use serde::Serialize;

/// The profile a delivery is read as: which national or European profile of
/// NeTEx its identifiers and frames show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Profile {
    /// The German national delivery (DELFI): identifiers `DE::…`.
    Delfi,
    /// An Austrian delivery: identifiers `at:…`.
    Austria,
    /// The Swiss delivery: identifiers `ch:…`.
    Switzerland,
    /// The European Passenger Information Profile: frames typed `epip:…`.
    Epip,
    /// None of these.
    Other,
}

/// A decision the conversion took, counted: what it is (`code`), what it
/// means, how often it was taken, and one example.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Decision {
    /// The decision's name: `call-without-place`, `trip-without-two-calls`, …
    pub code: &'static str,
    /// What the decision is, in words.
    pub what: &'static str,
    /// How often it was taken.
    pub count: u64,
    /// The first entity it was taken for, as the order of the conversion
    /// meets them.
    pub example: Option<String>,
}

/// Elements of the delivery the feed does not carry, by element name.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NotCarried {
    /// The element: `FareFrame`, `Notice`, `FlexibleLine`, …
    pub element: String,
    /// How many the delivery holds.
    pub count: u64,
    /// The document whose name sorts first among those holding one.
    pub first_document: String,
}

/// The report of a conversion.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Report {
    /// The profile the delivery was read as.
    pub profile: Profile,
    /// How many documents were read.
    pub documents: u32,
    /// The rows written, by file.
    pub rows: BTreeMap<String, u64>,
    /// The first and last day of the feed's calendar, `YYYYMMDD`.
    pub period: Option<(String, String)>,
    /// What the index found about the delivery, by concern code
    /// (`netex_rs::index::Concern::code`).
    pub index_findings: BTreeMap<String, u64>,
    /// The decisions the conversion took, in the order of their codes.
    pub decisions: Vec<Decision>,
    /// What the feed does not carry.
    pub not_carried: Vec<NotCarried>,
}

/// The decisions, counted as they are taken.
#[derive(Debug, Default)]
pub(crate) struct Decisions {
    taken: BTreeMap<&'static str, (&'static str, u64, Option<String>)>,
}

impl Decisions {
    /// Counts a decision about `example`.
    pub(crate) fn take(
        &mut self,
        code: &'static str,
        what: &'static str,
        example: impl FnOnce() -> String,
    ) {
        let entry = self.taken.entry(code).or_insert((what, 0, None));
        entry.1 += 1;
        if entry.2.is_none() {
            entry.2 = Some(example());
        }
    }

    pub(crate) fn into_list(self) -> Vec<Decision> {
        self.taken
            .into_iter()
            .map(|(code, (what, count, example))| Decision {
                code,
                what,
                count,
                example,
            })
            .collect()
    }
}
