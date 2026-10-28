//! The two schema releases: telling them apart, and writing for either.
//!
//! Both releases share one namespace, and the `version` attribute of a
//! publication is not to be trusted — most of the official 2.0 examples still
//! declare `1.0`. What tells the releases apart is what they know: the
//! elements, attributes and enumeration tokens one release declares and the
//! other does not, derived from the two schema sets by `tools/schema_table.py`
//! into [`table`]. Detection looks for those; writing for a release drops
//! them and reports each drop.

mod report;
mod scan;
pub mod table;

pub use report::{Item, Loss, Report};
pub use scan::{detect, rewrite, Detection, Evidence};

/// One of the two schema releases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Release {
    /// The 1.x line: releases 1.0 to 1.3, whose publication schema is versioned
    /// `1.1`, and which the German national delivery declares as `ntx:1.1`.
    V1,
    /// Release 2.0.
    V2,
}

impl Release {
    /// The other release.
    pub fn other(self) -> Release {
        match self {
            Release::V1 => Release::V2,
            Release::V2 => Release::V1,
        }
    }

    /// The release as it is named in prose: `1.x` or `2.0`.
    pub fn label(self) -> &'static str {
        match self {
            Release::V1 => "1.x",
            Release::V2 => "2.0",
        }
    }

    /// The value the `version` attribute of a publication takes for this release.
    pub fn declared_version(self) -> &'static str {
        match self {
            Release::V1 => "1.1",
            Release::V2 => "2.0",
        }
    }

    /// The release a publication's `version` attribute names, if it names one.
    ///
    /// `1.0`, `1.1`, `1.2.2`, `ntx:1.1` and `1.04:NO-NeTEx-stops:1.0` all name
    /// the 1.x line; `2.0` names 2.0. Anything else names neither.
    pub fn declared(version: &str) -> Option<Release> {
        let version = version.trim();
        let version = version.strip_prefix("ntx:").unwrap_or(version);
        match version.split(['.', ':']).next() {
            Some("1") => Some(Release::V1),
            Some("2") => Some(Release::V2),
            _ => None,
        }
    }
}

impl std::fmt::Display for Release {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

/// A document written for a release, with what it lost on the way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Written {
    /// The document.
    pub xml: String,
    /// What the release did not know, and was left out.
    pub report: Report,
}
