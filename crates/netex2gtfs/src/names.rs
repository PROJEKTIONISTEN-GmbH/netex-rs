//! Interned strings: the identifiers and texts the gather keeps, each once.

use std::hash::{BuildHasher, BuildHasherDefault};

use hashbrown::HashTable;
use rustc_hash::FxHasher;

/// A string's number among the [`Names`].
pub(crate) type Name = u32;

/// Every identifier and text gathered, each stored once and referred to by
/// number: the table holds numbers, compared against the strings they stand
/// for, so that no string is stored twice.
#[derive(Debug, Default)]
pub(crate) struct Names {
    table: HashTable<Name>,
    texts: Vec<Box<str>>,
    hasher: BuildHasherDefault<FxHasher>,
}

impl Names {
    /// The number of `text`, interning it if it is new.
    pub(crate) fn intern(&mut self, text: &str) -> Name {
        let hash = self.hasher.hash_one(text);
        let texts = &self.texts;
        *self
            .table
            .entry(
                hash,
                |&name| &*texts[name as usize] == text,
                |&name| self.hasher.hash_one(&*texts[name as usize]),
            )
            .or_insert_with(|| {
                let name = Name::try_from(self.texts.len()).expect("fewer than 2^32 names");
                self.texts.push(text.into());
                name
            })
            .get()
    }

    /// The number of `text`, if it is interned.
    pub(crate) fn get(&self, text: &str) -> Option<Name> {
        let hash = self.hasher.hash_one(text);
        self.table
            .find(hash, |&name| &*self.texts[name as usize] == text)
            .copied()
    }

    /// The text of a number.
    pub(crate) fn text(&self, name: Name) -> &str {
        &self.texts[name as usize]
    }

    /// How many names are interned.
    pub(crate) fn len(&self) -> usize {
        self.texts.len()
    }

    /// The numbers `other`'s names have among these, each interned here:
    /// the names new here taken over, not copied.
    pub(crate) fn absorb(&mut self, other: Names) -> Vec<Name> {
        other
            .texts
            .into_iter()
            .map(|text| self.intern_owned(text))
            .collect()
    }

    /// The number of `text`, keeping `text` itself if it is new: a name
    /// another gather allocated is taken over rather than copied.
    fn intern_owned(&mut self, text: Box<str>) -> Name {
        let hash = self.hasher.hash_one(&*text);
        let texts = &self.texts;
        if let Some(&name) = self
            .table
            .find(hash, |&name| *texts[name as usize] == *text)
        {
            return name;
        }
        let name = Name::try_from(self.texts.len()).expect("fewer than 2^32 names");
        self.texts.push(text);
        let texts = &self.texts;
        self.table.insert_unique(hash, name, |&name| {
            self.hasher.hash_one(&*texts[name as usize])
        });
        name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absorbing_names_renumbers_those_held_and_takes_over_the_new_ones() {
        let mut names = Names::default();
        let held = names.intern("ch:1:Line:IC1");
        let mut other = Names::default();
        other.intern("ch:1:Line:IC5");
        other.intern("ch:1:Line:IC1");
        let allocation = other.text(0).as_ptr();
        let renamed = names.absorb(other);
        assert_eq!(renamed, [1, held]);
        assert_eq!(names.text(1), "ch:1:Line:IC5");
        assert_eq!(names.text(1).as_ptr(), allocation, "not copied");
        assert_eq!(names.get("ch:1:Line:IC5"), Some(1));
        assert_eq!(names.len(), 2);
    }
}
