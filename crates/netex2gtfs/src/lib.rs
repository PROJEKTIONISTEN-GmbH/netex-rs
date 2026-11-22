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
//! let mut options = Options::default();
//! options.shapes = true;
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

// What a caller cannot rely on, so that the crate may grow without breaking
// it, compiled — and failing to — with the documentation tests.
/// The profiles a delivery is read in grow with the deliveries converted, and
/// the choices of stop identifiers may: both are matched with a wildcard arm
/// outside the crate.
///
/// ```compile_fail,E0004
/// use netex2gtfs::Profile;
/// fn german(profile: Profile) -> bool {
///     match profile {
///         Profile::Delfi => true,
///         Profile::Austria | Profile::Switzerland | Profile::Epip | Profile::Other => false,
///     }
/// }
/// ```
///
/// ```compile_fail,E0004
/// use netex2gtfs::StopIds;
/// fn global(ids: StopIds) -> bool {
///     match ids {
///         StopIds::Global => true,
///         StopIds::Netex => false,
///     }
/// }
/// ```
#[cfg(doctest)]
struct InterfaceThatGrows;

mod convert;
mod country;
mod delivery;
mod gather;
mod names;
pub mod output;
mod report;

pub use convert::{convert, Error, Options, Publisher, StopIds};
pub use delivery::{selection, Agency, Delivery, Reading};
pub use report::{Decision, NotCarried, Profile, Report};
