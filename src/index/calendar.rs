//! Calendars as bits: one bit per day of the dataset's span, kept only from
//! the first word holding a day operated to the last.
//!
//! The span runs from the first day anything of the delivery names to the
//! last, and one period to the year 9999 makes it millions of days long. A
//! calendar holding the days of one month keeps the words of that month
//! alone, whatever the span: what the period costs is the calendars of its
//! own days, not every calendar of the delivery.

/// The days a set of day types is operated on, one bit per day from the
/// dataset's first day.
///
/// The words before the first one holding a day operated and after the last
/// are not kept, so that two calendars of the same days are the same value.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub(crate) struct DayBits {
    /// The index of the first word kept.
    first_word: usize,
    /// The words from `first_word` on; neither the first nor the last is zero.
    words: Vec<u64>,
}

impl DayBits {
    /// A calendar of no day operated.
    pub(crate) fn none() -> Self {
        DayBits::default()
    }

    /// Whether day `day` (from the first day) is operated.
    pub(crate) fn get(&self, day: usize) -> bool {
        (day / 64)
            .checked_sub(self.first_word)
            .and_then(|word| self.words.get(word))
            .is_some_and(|word| word & (1 << (day % 64)) != 0)
    }

    /// Sets or clears day `day`.
    pub(crate) fn set(&mut self, day: usize, operated: bool) {
        if operated {
            self.set_range(day, day);
        } else if let Some(word) = (day / 64)
            .checked_sub(self.first_word)
            .and_then(|word| self.words.get_mut(word))
        {
            *word &= !(1 << (day % 64));
            self.trim();
        }
    }

    /// Sets every day from `first` to `last`, both included.
    pub(crate) fn set_range(&mut self, first: usize, last: usize) {
        if first > last {
            return;
        }
        self.cover(first / 64, last / 64);
        for word in first / 64..=last / 64 {
            let low = if word == first / 64 { first % 64 } else { 0 };
            let high = if word == last / 64 { last % 64 } else { 63 };
            // The bits from `low` to `high`, both included.
            let bits = (u64::MAX >> (63 - high)) & (u64::MAX << low);
            self.words[word - self.first_word] |= bits;
        }
    }

    /// Every operated day, from the first.
    pub(crate) fn days(&self) -> impl Iterator<Item = usize> + '_ {
        self.words
            .iter()
            .enumerate()
            .filter(|(_, &word)| word != 0)
            .flat_map(move |(index, &word)| {
                let base = (self.first_word + index) * 64;
                let mut rest = word;
                std::iter::from_fn(move || {
                    let bit = rest.trailing_zeros() as usize;
                    (rest != 0).then(|| {
                        rest &= rest - 1;
                        base + bit
                    })
                })
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
        let first = self.first_word.max(other.first_word);
        let end = self.end_word().min(other.end_word());
        if first >= end {
            *self = DayBits::none();
            return;
        }
        self.words.truncate(end - self.first_word);
        self.words.drain(..first - self.first_word);
        self.first_word = first;
        for (word, kept) in self
            .words
            .iter_mut()
            .zip(&other.words[first - other.first_word..])
        {
            *word &= kept;
        }
        self.trim();
    }

    /// Takes away every day of `other`.
    pub(crate) fn subtract(&mut self, other: &DayBits) {
        let first = self.first_word.max(other.first_word);
        let end = self.end_word().min(other.end_word());
        if first >= end {
            return;
        }
        let own = &mut self.words[first - self.first_word..end - self.first_word];
        for (word, gone) in own.iter_mut().zip(&other.words[first - other.first_word..]) {
            *word &= !gone;
        }
        self.trim();
    }

    /// Adds every day of `other`.
    pub(crate) fn union(&mut self, other: &DayBits) {
        if other.words.is_empty() {
            return;
        }
        self.cover(other.first_word, other.end_word() - 1);
        let own = &mut self.words[other.first_word - self.first_word..];
        for (word, more) in own.iter_mut().zip(&other.words) {
            *word |= more;
        }
    }

    /// Keeps only the days `keep` says to of those operated, each day given
    /// as its offset from the first day.
    pub(crate) fn retain(&mut self, mut keep: impl FnMut(usize) -> bool) {
        let first_word = self.first_word;
        for (index, word) in self.words.iter_mut().enumerate() {
            let mut rest = *word;
            while rest != 0 {
                let bit = rest.trailing_zeros() as usize;
                rest &= rest - 1;
                if !keep((first_word + index) * 64 + bit) {
                    *word &= !(1 << bit);
                }
            }
        }
        self.trim();
    }

    /// One past the index of the last word kept.
    fn end_word(&self) -> usize {
        self.first_word + self.words.len()
    }

    /// Keeps the words from `first` to `last`, both included, those not kept
    /// before zero.
    fn cover(&mut self, first: usize, last: usize) {
        if self.words.is_empty() {
            self.first_word = first;
            self.words = vec![0; last - first + 1];
            return;
        }
        if first < self.first_word {
            let before = self.first_word - first;
            self.words.splice(0..0, std::iter::repeat_n(0, before));
            self.first_word = first;
        }
        if last >= self.end_word() {
            self.words.resize(last - self.first_word + 1, 0);
        }
    }

    /// Lets go of the zero words at either end.
    fn trim(&mut self) {
        let Some(last) = self.words.iter().rposition(|&word| word != 0) else {
            *self = DayBits::none();
            return;
        };
        self.words.truncate(last + 1);
        let first = self
            .words
            .iter()
            .position(|&word| word != 0)
            .expect("a word is not zero");
        self.words.drain(..first);
        self.first_word += first;
    }
}

#[cfg(test)]
mod tests {
    use super::DayBits;

    fn of(days: &[usize]) -> DayBits {
        let mut bits = DayBits::none();
        for &day in days {
            bits.set(day, true);
        }
        bits
    }

    #[test]
    fn the_days_are_those_set_in_order_across_words() {
        let mut bits = of(&[64 * 39 + 7, 3, 63, 64, 130]);
        bits.set(130, false);
        let days: Vec<usize> = bits.days().collect();
        assert_eq!(days, [3, 63, 64, 64 * 39 + 7]);
        assert_eq!(bits.count(), days.len());
        assert!(bits.get(63) && !bits.get(62) && !bits.get(64 * 50));
    }

    #[test]
    fn a_calendar_keeps_the_words_of_its_own_days_alone() {
        let far = 64 * 1_000_000;
        let bits = of(&[far + 1, far + 70]);
        assert_eq!(bits.words.len(), 2, "two words, not a million");
        assert_eq!(bits.days().collect::<Vec<_>>(), [far + 1, far + 70]);
    }

    #[test]
    fn calendars_of_the_same_days_are_equal_however_they_were_made() {
        let mut made = of(&[5, 64 * 3, 64 * 9]);
        made.set(5, false);
        made.set(64 * 9, false);
        let mut cut = of(&[1, 64 * 3, 64 * 20]);
        cut.intersect(&of(&[64 * 3, 64 * 21]));
        assert_eq!(made, of(&[64 * 3]));
        assert_eq!(cut, of(&[64 * 3]));
        let mut emptied = of(&[7]);
        emptied.subtract(&of(&[7, 900]));
        assert_eq!(emptied, DayBits::none());
    }

    #[test]
    fn a_range_sets_every_day_of_it_within_and_across_words() {
        let mut bits = DayBits::none();
        bits.set_range(60, 200);
        bits.set_range(10, 12);
        let expected: Vec<usize> = (10..=12).chain(60..=200).collect();
        assert_eq!(bits.days().collect::<Vec<_>>(), expected);
        let mut word = DayBits::none();
        word.set_range(0, 63);
        assert_eq!(word.count(), 64);
    }

    #[test]
    fn union_intersection_and_difference_are_those_of_the_sets_of_days() {
        let (a, b) = ([3, 70, 500, 1_000], [70, 71, 1_000, 5_000]);
        let mut union = of(&a);
        union.union(&of(&b));
        assert_eq!(
            union.days().collect::<Vec<_>>(),
            [3, 70, 71, 500, 1_000, 5_000]
        );
        let mut both = of(&a);
        both.intersect(&of(&b));
        assert_eq!(both.days().collect::<Vec<_>>(), [70, 1_000]);
        let mut only = of(&a);
        only.subtract(&of(&b));
        assert_eq!(only.days().collect::<Vec<_>>(), [3, 500]);
        let mut earlier = of(&[5_000]);
        earlier.union(&of(&[2]));
        assert_eq!(earlier.days().collect::<Vec<_>>(), [2, 5_000]);
    }

    #[test]
    fn retaining_keeps_the_days_asked_for() {
        let mut bits = DayBits::none();
        bits.set_range(0, 300);
        bits.retain(|day| day % 7 == 0 && day > 64);
        let expected: Vec<usize> = (70..=300).step_by(7).collect();
        assert_eq!(bits.days().collect::<Vec<_>>(), expected);
        assert_eq!(bits.first_word, 1, "the emptied first word is let go");
    }
}
