//! CEN NeTEx — *Network Timetable Exchange*, CEN/TS 16614 — in Rust.
//!
//! Reads and writes NeTEx documents of both schema releases, 1.x and 2.0, into
//! one model; tells the releases apart; writes a document for either release
//! and reports what the release does not know; and, over a dataset of many
//! files, resolves references and builds the indexes a routing engine needs.
//!
//! # Reading and writing
//!
//! ```
//! let xml = r#"<PublicationDelivery xmlns="http://www.netex.org.uk/netex" version="ntx:1.1">
//!   <PublicationTimestamp>2026-09-19T15:04:00</PublicationTimestamp>
//!   <ParticipantRef>DE</ParticipantRef>
//!   <dataObjects>
//!     <SiteFrame id="DE::SiteFrame:1" version="1">
//!       <stopPlaces>
//!         <StopPlace id="DE::StopPlace:32901_vgm_d::" version="1789823040">
//!           <keyList><KeyValue><Key>GlobalID</Key><Value>de:05978:32901</Value></KeyValue></keyList>
//!           <Name>Fröndenberg, Mitte</Name>
//!         </StopPlace>
//!       </stopPlaces>
//!     </SiteFrame>
//!   </dataObjects>
//! </PublicationDelivery>"#;
//!
//! let document = netex_rs::read(xml)?;
//! let stop_place = document
//!     .delivery
//!     .frames()
//!     .find_map(|frame| match frame {
//!         netex_rs::model::Frame::Site(site) => site.stop_places.as_ref(),
//!         _ => None,
//!     })
//!     .and_then(|places| places.items.first())
//!     .and_then(|item| match item {
//!         netex_rs::model::StopPlaceItem::StopPlace(place) => Some(place),
//!         _ => None,
//!     })
//!     .expect("the document holds a stop place");
//! assert_eq!(stop_place.name.as_ref().map(|n| n.as_str()), Some("Fröndenberg, Mitte"));
//! assert_eq!(stop_place.key_list.as_ref().and_then(|k| k.get("GlobalID")), Some("de:05978:32901"));
//!
//! let written = document.write();
//! assert!(written.contains("<StopPlace id=\"DE::StopPlace:32901_vgm_d::\""));
//!
//! let for_20 = document.write_for(netex_rs::Release::V2);
//! assert!(for_20.report.is_lossless());
//! assert!(for_20.xml.contains("version=\"2.0\""));
//! # Ok::<(), netex_rs::Error>(())
//! ```
//!
//! What the crate does not type is not lost: every element and attribute the
//! model does not name is carried through as it was written, at the position
//! it was read, and written back there. See [`xml::Unprocessed`].
//!
//! # Reading lean
//!
//! An importer that wants the stops and the journeys of a delivery, not a
//! model of every document, reads entity by entity instead: [`stream::read`]
//! types only the kinds of entity a [`stream::Selection`] names and skips
//! everything else at the tokenizer, and [`index::Index::build`] builds the
//! index of a whole dataset that way, on every core, without a document model
//! in memory. The full model stays as it is; the lean path is in addition.
//!
//! # Merging deliveries
//!
//! [`merge::merge`] writes several deliveries as one, each under its rank and label, so that a
//! reference resolves as they are ranked: a stop place pairs with an earlier delivery's by
//! identifier, else by global identifier (`GlobalID`, `SLOID`); an object stated alike is kept once;
//! an identifier taken with other content is renamed into the delivery's codespace; a journey stated
//! in two deliveries is kept once. The documents are rewritten as they are written ([`written`]),
//! every other byte kept, and what was found is reported by class ([`findings`]).
//!
//! ```
//! use netex_rs::dataset::Source;
//! use netex_rs::merge::{self, Input};
//!
//! # let own = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/own/at");
//! # if !own.is_dir() { return Ok(()); }
//! # let to = tempfile::tempdir()?;
//! // The crate's own Austrian-style example, merged with itself: the second delivery's stop places
//! // pair with the first's, every object of it is stated alike and left out.
//! let inputs = [Input::new("oebb", Source::open(&own)?)?, Input::new("vor", Source::open(&own)?)?];
//! let report = merge::merge(&inputs, to.path())?;
//! assert_eq!(report.deliveries[1].objects_left_out, report.deliveries[1].objects);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Safety against foreign input
//!
//! No `unsafe` code. Entities are never expanded and DTDs never fetched; a
//! document that relies on either is refused. Nesting depth, the length of a
//! run of character data, the number of attributes on an element and the size
//! of a carried-through fragment are bounded by [`Limits`].

#![forbid(unsafe_code)]

pub mod calendar;
pub mod check;
#[cfg(feature = "crs")]
pub mod crs;
pub mod dataset;
pub mod error;
pub mod findings;
pub mod geometry;
pub mod index;
pub mod merge;
pub mod model;
pub mod stream;
pub mod version;
pub mod written;
pub mod xml;

// The README's examples, compiled — and run, where they read the crate's own
// examples — with the documentation tests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeExamples;

pub use error::{Error, Kind, Result};
pub use version::{Release, Report, Written};
pub use xml::{Attribute, Limits, Unprocessed};

use model::{Element, PublicationDelivery, Walk};
use xml::{Cursor, Emitter, NETEX_NAMESPACE};

/// A NeTEx document: a `<PublicationDelivery>` and the frames it delivers.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Document {
    /// The publication.
    pub delivery: PublicationDelivery,
}

/// Reads a document.
///
/// The document must be a `<PublicationDelivery>` in the NeTEx namespace, of
/// either release. [`Limits::default`] applies; see [`read_with`].
pub fn read(xml: &str) -> Result<Document> {
    read_with(xml, &Limits::default())
}

/// Reads a document within `limits`.
pub fn read_with(xml: &str, limits: &Limits) -> Result<Document> {
    let mut cursor = Cursor::new(xml, *limits);
    let root = cursor.root()?;
    if !(root.netex && root.local == "PublicationDelivery") {
        return Err(cursor.error_at(
            Kind::NotNetex {
                root: root.local.to_owned(),
            },
            root.offset,
        ));
    }
    let delivery = PublicationDelivery::read(&mut cursor, &root)?;
    cursor.finish()?;
    Ok(Document { delivery })
}

/// Reads a document rooted at any typed element: a `<StopPlace>` published on
/// its own, say.
pub fn read_element<T: Element>(xml: &str, name: &str, limits: &Limits) -> Result<T> {
    let mut cursor = Cursor::new(xml, *limits);
    let root = cursor.root()?;
    if !(root.netex && root.local == name) {
        return Err(cursor.error_at(
            Kind::NotNetex {
                root: root.local.to_owned(),
            },
            root.offset,
        ));
    }
    let element = T::read(&mut cursor, &root)?;
    cursor.finish()?;
    Ok(element)
}

/// Writes a typed element as a document of its own, under `name`.
pub fn write_element<T: Element>(element: &T, name: &str) -> String {
    let mut out = Emitter::new();
    element.write(name, &mut out);
    out.finish()
}

impl Document {
    /// A document delivering `delivery`.
    ///
    /// The publication is given the NeTEx namespace if it does not declare it.
    pub fn new(mut delivery: PublicationDelivery) -> Self {
        if !delivery
            .extra_attributes
            .iter()
            .any(|a| &*a.name == "xmlns")
        {
            delivery.extra_attributes.insert(
                0,
                Attribute {
                    name: "xmlns".into(),
                    value: NETEX_NAMESPACE.into(),
                },
            );
        }
        Document { delivery }
    }

    /// Writes the document as it is: whatever release it was read in, with
    /// everything it carried.
    pub fn write(&self) -> String {
        let mut out = Emitter::with_capacity(64 * 1024);
        self.delivery.write("PublicationDelivery", &mut out);
        out.finish()
    }

    /// Writes the document for `release`, leaving out what that release does
    /// not know and reporting each thing left out.
    pub fn write_for(&self, release: Release) -> Written {
        version::rewrite(&self.write(), release)
    }

    /// Calls `visit` with each fragment the typed model carried through
    /// untyped, under the name of the element it was read in.
    pub fn for_each_unprocessed(&self, visit: &mut dyn FnMut(&str, &Unprocessed)) {
        self.delivery.walk_unprocessed("PublicationDelivery", visit);
    }
}

/// A document rooted at any NeTEx element, carried through as it was written.
///
/// The official examples include documents that publish a single element —
/// a `<StopPlace>`, a `<Locale>`, a fare product. This carries such a
/// document whole; [`read_element`] reads one into a typed element instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fragment {
    /// The root element, as written.
    pub root: Unprocessed,
}

impl Fragment {
    /// Reads a fragment document within [`Limits::default`].
    pub fn read(xml: &str) -> Result<Fragment> {
        Self::read_with(xml, &Limits::default())
    }

    /// Reads a fragment document within `limits`.
    pub fn read_with(xml: &str, limits: &Limits) -> Result<Fragment> {
        let mut cursor = Cursor::new(xml, *limits);
        let root = cursor.root()?;
        if !root.netex {
            return Err(cursor.error_at(
                Kind::NotNetex {
                    root: root.local.to_owned(),
                },
                root.offset,
            ));
        }
        let root = cursor.skip(&root, 0)?;
        cursor.finish()?;
        Ok(Fragment { root })
    }

    /// Writes the fragment as it is.
    pub fn write(&self) -> String {
        let mut out = Emitter::with_capacity(self.root.xml.len() + 64);
        out.raw(&self.root);
        out.finish()
    }

    /// Writes the fragment for `release`, leaving out what that release does
    /// not know and reporting each thing left out.
    pub fn write_for(&self, release: Release) -> Written {
        version::rewrite(&self.write(), release)
    }
}
