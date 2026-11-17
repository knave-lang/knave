#[derive(Clone, Copy)]
pub(super) struct SavedWords<const WIDTH: usize> {
    bytes: &'static [u8],
}

impl<const WIDTH: usize> SavedWords<WIDTH> {
    const WORD: usize = size_of::<u32>();
    const ROW: usize = WIDTH * Self::WORD;

    pub(super) fn new(bytes: &'static [u8]) -> Self {
        Self { bytes }
    }

    pub(super) fn rows(&self) -> usize {
        self.bytes.len() / Self::ROW
    }

    pub(super) fn row(&self, position: usize) -> [u32; WIDTH] {
        let start = position * Self::ROW;
        let row = &self.bytes[start..start + Self::ROW];
        std::array::from_fn(|column| {
            let word = &row[column * Self::WORD..(column + 1) * Self::WORD];
            u32::from_le_bytes(word.try_into().expect("a saved word has four bytes"))
        })
    }
}

#[derive(Default, serde::Serialize)]
#[serde(transparent)]
pub(super) struct SavedWordsBuilder<const WIDTH: usize> {
    bytes: Vec<u8>,
}

impl<const WIDTH: usize> SavedWordsBuilder<WIDTH> {
    pub(super) fn push(&mut self, row: [usize; WIDTH]) {
        for value in row {
            let word = u32::try_from(value).expect("the saved prelude stays under 4 GiB");
            self.bytes.extend(word.to_le_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SavedWords, SavedWordsBuilder};

    #[test]
    fn a_saved_row_reads_back_each_word_in_its_column() {
        let mut builder = SavedWordsBuilder::<2>::default();
        builder.push([1, 2]);
        builder.push([3, 70_000]);
        let words = SavedWords::<2>::new(builder.bytes.leak());
        assert_eq!(words.rows(), 2);
        assert_eq!(words.row(1), [3, 70_000]);
    }
}
