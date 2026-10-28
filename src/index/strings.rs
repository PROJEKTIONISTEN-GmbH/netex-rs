//! Interned strings: every identifier and name once, referred to by number.

use std::hash::{BuildHasher, BuildHasherDefault};

use hashbrown::HashTable;
use rustc_hash::FxHasher;

/// A string's number in the interner.
pub(crate) type Sym = u32;

/// The interner: a string is stored once and handed out as a [`Sym`].
///
/// The table holds numbers, not strings: a lookup hashes the text and
/// compares it against the string the number stands for, so that the
/// twenty-five million strings of the German delivery are stored once, not
/// once as a key and once in the list.
#[derive(Debug, Default)]
pub(crate) struct Strings {
    table: HashTable<Sym>,
    list: Vec<Box<str>>,
    hasher: BuildHasherDefault<FxHasher>,
}

impl PartialEq for Strings {
    fn eq(&self, other: &Self) -> bool {
        self.list == other.list
    }
}

impl Strings {
    /// The number of `text`, interning it if it is new.
    pub(crate) fn intern(&mut self, text: &str) -> Sym {
        let hash = self.hasher.hash_one(text);
        let list = &self.list;
        *self
            .table
            .entry(
                hash,
                |&sym| &*list[sym as usize] == text,
                |&sym| self.hasher.hash_one(&*list[sym as usize]),
            )
            .or_insert_with(|| {
                let sym = Sym::try_from(self.list.len()).expect("fewer than 2^32 distinct strings");
                self.list.push(text.into());
                sym
            })
            .get()
    }

    /// The number of `text`, if it is interned.
    pub(crate) fn get(&self, text: &str) -> Option<Sym> {
        let hash = self.hasher.hash_one(text);
        self.table
            .find(hash, |&sym| &*self.list[sym as usize] == text)
            .copied()
    }

    /// The string with this number.
    pub(crate) fn resolve(&self, sym: Sym) -> &str {
        &self.list[sym as usize]
    }

    /// How many strings are interned.
    pub(crate) fn len(&self) -> usize {
        self.list.len()
    }

    /// Every string, in the order of its number.
    pub(crate) fn as_slice(&self) -> &[Box<str>] {
        &self.list
    }

    /// Makes room for `additional` more strings, so that interning them
    /// grows the table once rather than doubling by doubling.
    pub(crate) fn reserve(&mut self, additional: usize) {
        let list = &self.list;
        self.table.reserve(additional, |&sym| {
            self.hasher.hash_one(&*list[sym as usize])
        });
        self.list.reserve(additional);
    }
}
