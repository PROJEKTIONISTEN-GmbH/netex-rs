//! The fingerprints of a delivery, and what changed since the last one.

use std::collections::HashMap;
use std::fmt::Write;
use std::hash::Hash;

use crate::error::{Error, Kind, Result};

/// One document's fingerprint: its name, its inflated size and the CRC-32 of
/// its bytes, as the archive's central directory records them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Fingerprint {
    /// The document's name: its path in the archive or under the directory.
    pub name: String,
    /// Its size in bytes, inflated.
    pub size: u64,
    /// The CRC-32 of its bytes.
    pub crc32: u32,
}

/// The fingerprints of every document of a delivery, in the delivery's order.
///
/// Two deliveries are compared document by document: a document in both with
/// the same size and CRC is unchanged, with another is changed; one only in
/// the newer delivery is added, only in the older one removed. What makes a
/// document the same document is its name ([`diff`](Self::diff)) or an
/// identity the caller derives from the name ([`diff_by`](Self::diff_by)) —
/// the German delivery dates every file name, so there the identity is the
/// name without its date. The text form
/// ([`to_text`](Self::to_text), [`parse`](Self::parse)) is one line per
/// document — CRC in hexadecimal, size, name, tab-separated — so that a
/// manifest can be kept beside the index it was built for.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Manifest {
    /// The fingerprints, in the delivery's order.
    pub entries: Vec<Fingerprint>,
}

/// What changed from one delivery to the next; the lists keep the newer
/// delivery's order, `removed` the older one's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Changes<'a> {
    /// Documents in the newer delivery only.
    pub added: Vec<&'a Fingerprint>,
    /// Documents in both, with another size or CRC in the newer one; the
    /// newer fingerprint.
    pub changed: Vec<&'a Fingerprint>,
    /// Documents in the older delivery only; the older fingerprint.
    pub removed: Vec<&'a Fingerprint>,
    /// Documents in both, byte for byte the same; the newer fingerprint.
    pub unchanged: Vec<&'a Fingerprint>,
}

impl Changes<'_> {
    /// The names of the documents an importer has to read: the added and the
    /// changed ones.
    pub fn to_read(&self) -> impl Iterator<Item = &str> {
        self.added
            .iter()
            .chain(&self.changed)
            .map(|fingerprint| fingerprint.name.as_str())
    }
}

impl Manifest {
    /// What changed in this manifest since `previous`, a document being the
    /// same document when it has the same name.
    pub fn diff<'a>(&'a self, previous: &'a Manifest) -> Changes<'a> {
        self.diff_by(previous, |name| name)
    }

    /// What changed in this manifest since `previous`, a document being the
    /// same document when `identity` of its name is the same — the name
    /// without the date the German delivery puts in it, say. Two documents of
    /// one delivery with the same identity are compared as the later one.
    pub fn diff_by<'a, K, F>(&'a self, previous: &'a Manifest, identity: F) -> Changes<'a>
    where
        K: Eq + Hash,
        F: Fn(&'a str) -> K,
    {
        let before: HashMap<K, &Fingerprint> = previous
            .entries
            .iter()
            .map(|fingerprint| (identity(&fingerprint.name), fingerprint))
            .collect();
        let now: HashMap<K, &Fingerprint> = self
            .entries
            .iter()
            .map(|fingerprint| (identity(&fingerprint.name), fingerprint))
            .collect();
        let mut changes = Changes {
            added: Vec::new(),
            changed: Vec::new(),
            removed: Vec::new(),
            unchanged: Vec::new(),
        };
        for fingerprint in &self.entries {
            match before.get(&identity(&fingerprint.name)) {
                None => changes.added.push(fingerprint),
                Some(old) if old.size == fingerprint.size && old.crc32 == fingerprint.crc32 => {
                    changes.unchanged.push(fingerprint);
                }
                Some(_) => changes.changed.push(fingerprint),
            }
        }
        for fingerprint in &previous.entries {
            if !now.contains_key(&identity(&fingerprint.name)) {
                changes.removed.push(fingerprint);
            }
        }
        changes
    }

    /// The manifest as text: one line per document, `crc32<TAB>size<TAB>name`,
    /// the CRC as eight hexadecimal digits.
    pub fn to_text(&self) -> String {
        let mut text = String::new();
        for fingerprint in &self.entries {
            writeln!(
                text,
                "{:08x}\t{}\t{}",
                fingerprint.crc32, fingerprint.size, fingerprint.name
            )
            .expect("writing to a string cannot fail");
        }
        text
    }

    /// A manifest from its text form.
    pub fn parse(text: &str) -> Result<Manifest> {
        let mut entries = Vec::new();
        for (number, line) in text.lines().enumerate() {
            let malformed = || {
                Error::new(Kind::Io(format!(
                    "manifest line {}: expected crc32<TAB>size<TAB>name, got {line:?}",
                    number + 1
                )))
            };
            let mut fields = line.splitn(3, '\t');
            let crc32 = fields
                .next()
                .and_then(|field| u32::from_str_radix(field, 16).ok())
                .ok_or_else(malformed)?;
            let size = fields
                .next()
                .and_then(|field| field.parse().ok())
                .ok_or_else(malformed)?;
            let name = fields
                .next()
                .filter(|name| !name.is_empty())
                .ok_or_else(malformed)?;
            entries.push(Fingerprint {
                name: name.to_owned(),
                size,
                crc32,
            });
        }
        Ok(Manifest { entries })
    }
}
