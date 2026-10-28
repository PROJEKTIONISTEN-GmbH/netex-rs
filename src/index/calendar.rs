//! Calendars as bits: one bit per day of the dataset's span.

/// The days a set of day types is operated on, one bit per day from the
/// dataset's first day.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct DayBits {
    words: Vec<u64>,
}

impl DayBits {
    /// A calendar of `days` days, none operated.
    pub(crate) fn new(days: usize) -> Self {
        DayBits {
            words: vec![0; days.div_ceil(64)],
        }
    }

    /// Whether day `day` (from the first day) is operated.
    pub(crate) fn get(&self, day: usize) -> bool {
        self.words
            .get(day / 64)
            .is_some_and(|word| word & (1 << (day % 64)) != 0)
    }

    /// Sets or clears day `day`.
    pub(crate) fn set(&mut self, day: usize, operated: bool) {
        if let Some(word) = self.words.get_mut(day / 64) {
            if operated {
                *word |= 1 << (day % 64);
            } else {
                *word &= !(1 << (day % 64));
            }
        }
    }

    /// Every operated day, from the first.
    pub(crate) fn days(&self) -> impl Iterator<Item = usize> + '_ {
        self.words.iter().enumerate().flat_map(|(index, &word)| {
            (0..64)
                .filter(move |bit| word & (1 << bit) != 0)
                .map(move |bit| index * 64 + bit)
        })
    }

    /// How many days are operated.
    pub(crate) fn count(&self) -> usize {
        self.words
            .iter()
            .map(|word| word.count_ones() as usize)
            .sum()
    }

    /// Keeps only the days `other` operates too.
    pub(crate) fn intersect(&mut self, other: &DayBits) {
        for (word, kept) in self.words.iter_mut().zip(&other.words) {
            *word &= kept;
        }
    }

    /// Takes away every day of `other`.
    pub(crate) fn subtract(&mut self, other: &DayBits) {
        for (word, gone) in self.words.iter_mut().zip(&other.words) {
            *word &= !gone;
        }
    }

    /// Adds every day of `other`.
    pub(crate) fn union(&mut self, other: &DayBits) {
        for (word, more) in self.words.iter_mut().zip(&other.words) {
            *word |= more;
        }
    }
}
