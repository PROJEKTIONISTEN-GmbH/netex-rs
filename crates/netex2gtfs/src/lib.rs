//! NeTEx to GTFS: a NeTEx delivery, read and resolved by [`netex_rs`], written
//! as a GTFS feed — with a report of every decision the conversion took and of
//! everything in the delivery the feed does not carry.
//!
//! ```no_run
//! use netex_rs::dataset::Source;
//! use netex2gtfs::{convert, output, Delivery, Options, Reading};
//!
//! let source = Source::open("netex.zip")?;
//! let delivery = Delivery::read(&source, Reading { geometry: true, ..Reading::default() })?;
//! let options = Options { shapes: true, ..Options::default() };
//! let report = convert(&delivery, &options, output::at("gtfs.zip")?)?;
//! for decision in &report.decisions {
//!     eprintln!("{} × {}: {}", decision.count, decision.code, decision.what);
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! The delivery is read once, on every core: the index of [`netex_rs::index`]
//! — stops, lines, journeys with their calls and days, walks, interchanges, each
//! reference resolved in its own document first — and, in the same walk, what
//! a feed needs beyond it: the organisations running the lines, destination
//! displays, directions, train numbers, the accessibility, platform codes and
//! levels of stops, the geometry of service links. `README.md` says how each
//! file of the feed is made, and which decisions the report counts.

// The README's example, compiled with the documentation tests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeExample;

mod convert;
mod delivery;
mod gather;
mod names;
pub mod output;
mod report;

pub use convert::{convert, Error, Options, Publisher, StopIds};
pub use delivery::{selection, Agency, Delivery, Reading};
pub use report::{Decision, NotCarried, Profile, Report};
