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

    /// The number of `text`, keeping `text` itself if it is new: a string
    /// another interner allocated is taken over rather than copied, so that
    /// merging interners does not hold each new string twice.
    pub(crate) fn intern_owned(&mut self, text: Box<str>) -> Sym {
        let hash = self.hasher.hash_one(&*text);
        let list = &self.list;
        if let Some(&sym) = self.table.find(hash, |&sym| *list[sym as usize] == *text) {
            return sym;
        }
        let sym = Sym::try_from(self.list.len()).expect("fewer than 2^32 distinct strings");
        self.list.push(text);
        let list = &self.list;
        self.table
            .insert_unique(hash, sym, |&sym| self.hasher.hash_one(&*list[sym as usize]));
        sym
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

    /// Every string, in the order of its number, the interner given up.
    pub(crate) fn into_list(self) -> Vec<Box<str>> {
        self.list
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_string_new_to_the_interner_is_taken_over_and_one_it_holds_keeps_its_number() {
        let mut strings = Strings::default();
        let held = strings.intern("ch:1:ServiceJourney:1");
        let text: Box<str> = "ch:1:ServiceJourney:2".into();
        let allocation = text.as_ptr();
        let new = strings.intern_owned(text);
        assert_eq!(strings.resolve(new), "ch:1:ServiceJourney:2");
        assert_eq!(strings.resolve(new).as_ptr(), allocation, "not copied");
        assert_eq!(strings.intern_owned("ch:1:ServiceJourney:1".into()), held);
        assert_eq!(strings.get("ch:1:ServiceJourney:2"), Some(new));
        assert_eq!(strings.intern("ch:1:ServiceJourney:2"), new);
        assert_eq!(strings.len(), 2);
    }
}
