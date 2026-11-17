#[derive(Clone, Copy)]
pub(super) struct SavedWords {
    bytes: &'static [u8],
    width: usize,
}

impl SavedWords {
    const WORD: usize = size_of::<u32>();

    pub(super) fn new(bytes: &'static [u8], width: usize) -> Self {
        Self { bytes, width }
    }

    pub(super) fn rows(&self) -> usize {
        self.bytes.len() / (self.width * Self::WORD)
    }

    pub(super) fn word(&self, row: usize, column: usize) -> u32 {
        let start = (row * self.width + column) * Self::WORD;
        let word = &self.bytes[start..start + Self::WORD];
        u32::from_le_bytes(word.try_into().expect("a saved word has four bytes"))
    }
}

#[derive(Default, serde::Serialize)]
#[serde(transparent)]
pub(super) struct SavedWordsBuilder {
    bytes: Vec<u8>,
}

impl SavedWordsBuilder {
    pub(super) fn push(&mut self, row: &[usize]) {
        for value in row {
            let word = u32::try_from(*value).expect("the saved prelude stays under 4 GiB");
            self.bytes.extend(word.to_le_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SavedWords, SavedWordsBuilder};

    #[test]
    fn a_saved_row_reads_back_each_word_in_its_column() {
        let mut builder = SavedWordsBuilder::default();
        builder.push(&[1, 2]);
        builder.push(&[3, 70_000]);
        let words = SavedWords::new(builder.bytes.leak(), 2);
        assert_eq!(words.rows(), 2);
        assert_eq!(words.word(1, 0), 3);
        assert_eq!(words.word(1, 1), 70_000);
    }
}
