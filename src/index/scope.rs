//! Where a reference resolves.
//!
//! Two kinds of entity, two answers:
//!
//! - A stop place, quay, stop point or level is one thing in the world, and
//!   the documents that repeat it describe the same thing: it is gathered once
//!   ([`Table`]), every repetition merged into it.
//! - A line, journey pattern, journey or anything of a calendar is what its
//!   document says it is. The Linz AG reuses the identifiers of its calendars
//!   in every line document with other days in each; the Swiss delivery
//!   defines its calendar in one document and names it from five hundred. So a
//!   reference resolves in the document it stands in first, and only where its
//!   document does not define what it names, in the delivery: there, of the
//!   documents defining it, the one whose name sorts first answers
//!   ([`Scoped`]).

use rustc_hash::FxHashMap;

use super::build::{version_is_higher, Key, Pending, Validity};
use super::strings::{Strings, Sym};
use super::Unresolved;

/// What a table answers a reference with.
pub(crate) trait Lookup {
    /// The definition of exactly `key`, as `document` sees it.
    fn exact(&self, key: Key, document: Sym) -> Option<u32>;

    /// The definition of `id` at its highest version, as `document` sees it.
    fn latest(&self, id: Sym, document: Sym) -> Option<u32>;

    /// Every definition of `id`, at any version, as `document` sees it.
    fn versions(&self, id: Sym, document: Sym) -> Vec<u32>;
}

/// Entities gathered once, by key, and the highest version of each identifier.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Table {
    by_key: FxHashMap<Key, u32>,
    /// Per identifier: the entity at its highest version, and that version.
    latest: FxHashMap<Sym, (u32, Option<Sym>)>,
}

impl Table {
    /// Adds an entity, unless one with the same key is in already; then the
    /// index of the entity that was kept.
    pub(crate) fn insert_or_kept(
        &mut self,
        strings: &Strings,
        key: Key,
        index: u32,
    ) -> Result<(), u32> {
        use std::collections::hash_map::Entry;
        match self.by_key.entry(key) {
            Entry::Occupied(kept) => return Err(*kept.get()),
            Entry::Vacant(slot) => {
                slot.insert(index);
            }
        }
        let newer = match self.latest.get(&key.id) {
            None => true,
            Some((_, current)) => version_is_higher(strings, key.version, *current),
        };
        if newer {
            self.latest.insert(key.id, (index, key.version));
        }
        Ok(())
    }

    /// The entity with this identifier at its highest version.
    pub(crate) fn latest_of(&self, id: Sym) -> Option<u32> {
        self.latest.get(&id).map(|(index, _)| *index)
    }
}

impl Lookup for Table {
    fn exact(&self, key: Key, _: Sym) -> Option<u32> {
        self.by_key.get(&key).copied()
    }

    fn latest(&self, id: Sym, _: Sym) -> Option<u32> {
        self.latest_of(id)
    }

    fn versions(&self, id: Sym, _: Sym) -> Vec<u32> {
        self.by_key
            .iter()
            .filter(|(key, _)| key.id == id)
            .map(|(_, &index)| index)
            .collect()
    }
}

/// Entities of one kind as each document defines them.
///
/// Built once every document is in, over the definitions in the order of
/// their documents' names — within a document, in the order it states them —
/// so that what a reference resolves to does not depend on the order the
/// documents were read in.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Scoped {
    /// Per key, the definition the delivery answers with: the first, in the
    /// order of the documents' names.
    by_key: FxHashMap<Key, u32>,
    /// Per identifier, the delivery's definition at its highest version.
    latest: FxHashMap<Sym, u32>,
    /// For an identifier more than one document defines: each document's
    /// first definition of each of its keys.
    in_document: FxHashMap<(Key, Sym), u32>,
    /// For an identifier more than one document defines: each document's
    /// definition at the highest version that document states.
    latest_in_document: FxHashMap<(Sym, Sym), u32>,
}

impl Scoped {
    /// The table of `definitions` — each a key and the document defining it,
    /// numbered by position, in the order of their documents' names.
    pub(crate) fn build(strings: &Strings, definitions: &[(Key, Sym)]) -> Scoped {
        let mut scoped = Scoped::default();
        // The first document defining each identifier, and whether another does too.
        let mut documents_of: FxHashMap<Sym, (Sym, bool)> = FxHashMap::default();
        for (index, &(key, document)) in definitions.iter().enumerate() {
            let index = u32::try_from(index).expect("fewer than 2^32 definitions");
            scoped.by_key.entry(key).or_insert(index);
            let higher = |kept: u32| {
                version_is_higher(strings, key.version, definitions[kept as usize].0.version)
            };
            match scoped.latest.get(&key.id) {
                Some(&kept) if !higher(kept) => {}
                _ => {
                    scoped.latest.insert(key.id, index);
                }
            }
            let (first, shared) = documents_of.entry(key.id).or_insert((document, false));
            *shared |= *first != document;
        }
        for (index, &(key, document)) in definitions.iter().enumerate() {
            if !documents_of[&key.id].1 {
                continue;
            }
            let index = u32::try_from(index).expect("fewer than 2^32 definitions");
            scoped.in_document.entry((key, document)).or_insert(index);
            let slot = scoped.latest_in_document.entry((key.id, document));
            let kept = *slot.or_insert(index);
            if version_is_higher(strings, key.version, definitions[kept as usize].0.version) {
                scoped.latest_in_document.insert((key.id, document), index);
            }
        }
        scoped
    }

    /// The definition the delivery answers with for `key`, whichever document asks.
    pub(crate) fn of_delivery(&self, key: Key) -> Option<u32> {
        self.by_key.get(&key).copied()
    }

    /// The definition the delivery answers with for `id`, at its highest version.
    pub(crate) fn latest_of_delivery(&self, id: Sym) -> Option<u32> {
        self.latest.get(&id).copied()
    }

    /// The definition of `id` at its highest version in `document`, if
    /// `document` defines it and another document does too.
    pub(crate) fn latest_in(&self, id: Sym, document: Sym) -> Option<u32> {
        self.latest_in_document.get(&(id, document)).copied()
    }
}

impl Lookup for Scoped {
    fn exact(&self, key: Key, document: Sym) -> Option<u32> {
        self.in_document
            .get(&(key, document))
            .or_else(|| self.by_key.get(&key))
            .copied()
    }

    fn latest(&self, id: Sym, document: Sym) -> Option<u32> {
        self.latest_in(id, document)
            .or_else(|| self.latest.get(&id).copied())
    }

    fn versions(&self, id: Sym, document: Sym) -> Vec<u32> {
        let own: Vec<u32> = self
            .in_document
            .iter()
            .filter(|((key, by), _)| key.id == id && *by == document)
            .map(|(_, &index)| index)
            .collect();
        if !own.is_empty() {
            return own;
        }
        self.by_key
            .iter()
            .filter(|(key, _)| key.id == id)
            .map(|(_, &index)| index)
            .collect()
    }
}

/// A reference resolved: the entity, and the version it names where that was
/// set aside to resolve it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Resolved {
    pub(crate) index: u32,
    pub(crate) version_set_aside: Option<Sym>,
}

/// Resolves a reference: the version it names, else its `versionRef`, else an
/// unversioned definition, else — with a day given — the version valid that
/// day, else the highest version; each as the reference's document sees the
/// table. A reference to nothing the table holds is a finding; one resolved
/// past the version it names says so, unless it names `any`.
pub(crate) fn resolve(
    strings: &Strings,
    table: &impl Lookup,
    validity: impl Fn(u32) -> Validity,
    at: Option<i64>,
    pending: &Pending,
) -> Result<Resolved, Unresolved> {
    let document = pending.origin;
    let exactly = |index| Resolved {
        index,
        version_set_aside: None,
    };
    if pending.key.version.is_some() {
        if let Some(found) = table.exact(pending.key, document) {
            return Ok(exactly(found));
        }
    }
    if let Some(version_ref) = pending.version_ref {
        let key = Key {
            id: pending.key.id,
            version: Some(version_ref),
        };
        if let Some(found) = table.exact(key, document) {
            return Ok(exactly(found));
        }
    }
    let named = pending
        .key
        .version
        .or(pending.version_ref)
        .filter(|&version| strings.resolve(version) != "any");
    let past_the_version = |index| Resolved {
        index,
        version_set_aside: named,
    };
    let unversioned = Key {
        id: pending.key.id,
        version: None,
    };
    if let Some(found) = table.exact(unversioned, document) {
        return Ok(past_the_version(found));
    }
    if let Some(day) = at {
        if let Some(valid) = table
            .versions(pending.key.id, document)
            .into_iter()
            .filter(|&index| validity(index).covers(day))
            .max()
        {
            return Ok(past_the_version(valid));
        }
    }
    table
        .latest(pending.key.id, document)
        .map(past_the_version)
        .ok_or_else(|| Unresolved {
            origin: strings.resolve(pending.origin).to_owned(),
            referrer: format!(
                "{} {}",
                pending.referrer.element,
                strings.resolve(pending.referrer.id)
            ),
            element: pending.element.to_owned(),
            id: strings.resolve(pending.key.id).to_owned(),
            version: pending
                .key
                .version
                .or(pending.version_ref)
                .map(|v| strings.resolve(v).to_owned()),
        })
}
