use std::ops::Range;

use crate::core::BindingId;

pub(super) struct SavedWords;

impl SavedWords {
    pub(super) const SIZE: usize = size_of::<u32>();

    pub(super) fn read(bytes: &[u8]) -> u32 {
        let (word, _) = bytes.split_at(Self::SIZE);
        u32::from_le_bytes(word.try_into().expect("a saved word has four bytes"))
    }

    pub(super) fn index(value: usize) -> u32 {
        u32::try_from(value).expect("the saved prelude stays under 4 GiB")
    }

    pub(super) fn write(words: &[u32], bytes: &mut Vec<u8>) {
        for word in words {
            bytes.extend(word.to_le_bytes());
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct BindingIndex {
    rows: &'static [u8],
    row_size: usize,
}

impl BindingIndex {
    pub(super) fn new(rows: &'static [u8], words_per_row: usize) -> Self {
        Self {
            rows,
            row_size: words_per_row * SavedWords::SIZE,
        }
    }

    pub(super) fn all_rows(&self) -> Range<usize> {
        0..self.rows.len() / self.row_size
    }

    pub(super) fn row(&self, index: usize) -> &'static [u8] {
        &self.rows[index * self.row_size..(index + 1) * self.row_size]
    }

    pub(super) fn word(&self, index: usize, field: usize) -> u32 {
        SavedWords::read(&self.row(index)[field * SavedWords::SIZE..])
    }

    pub(super) fn binding(&self, index: usize) -> BindingId {
        BindingId::from_index(self.word(index, 0))
    }

    pub(super) fn rows_of(&self, binding: BindingId) -> Range<usize> {
        let all = self.all_rows();
        let start = self.first_row_at_or_after(binding.index());
        let end = (start..all.end)
            .find(|index| self.word(*index, 0) != binding.index())
            .unwrap_or(all.end);
        start..end
    }

    fn first_row_at_or_after(&self, binding: u32) -> usize {
        let (mut low, mut high) = (0, self.all_rows().end);
        while low < high {
            let middle = low.midpoint(high);
            if self.word(middle, 0) < binding {
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        low
    }
}

#[cfg(test)]
mod tests {
    use super::{BindingIndex, SavedWords};
    use crate::core::BindingId;

    struct IndexFixture;

    impl IndexFixture {
        const WORDS_PER_ROW: usize = 2;

        fn index(bindings: &[u32]) -> BindingIndex {
            let mut rows = Vec::new();
            for (position, binding) in bindings.iter().enumerate() {
                SavedWords::write(&[*binding, SavedWords::index(position)], &mut rows);
            }
            BindingIndex::new(rows.leak(), Self::WORDS_PER_ROW)
        }
    }

    #[test]
    fn every_row_of_a_binding_is_found() {
        let index = IndexFixture::index(&[1, 3, 3, 3, 8]);
        assert_eq!(index.rows_of(BindingId::from_index(3)), 1..4);
    }

    #[test]
    fn the_first_and_the_last_binding_are_found() {
        let index = IndexFixture::index(&[1, 3, 8]);
        assert_eq!(index.rows_of(BindingId::from_index(1)), 0..1);
        assert_eq!(index.rows_of(BindingId::from_index(8)), 2..3);
    }

    #[test]
    fn a_missing_binding_has_no_rows() {
        let index = IndexFixture::index(&[1, 3, 8]);
        assert!(index.rows_of(BindingId::from_index(5)).is_empty());
        assert!(index.rows_of(BindingId::from_index(9)).is_empty());
    }

    #[test]
    fn a_row_reads_its_words() {
        let index = IndexFixture::index(&[4, 6]);
        assert_eq!(index.word(1, 1), 1);
        assert_eq!(index.binding(1), BindingId::from_index(6));
    }
}
