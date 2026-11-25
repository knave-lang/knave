use super::saved_words::{SavedWords, SavedWordsBuilder};

#[derive(serde::Serialize)]
pub struct PositionBytes {
    positions: SavedWordsBuilder<1>,
}

impl PositionBytes {
    pub fn of(positions: impl Iterator<Item = usize>) -> Self {
        let mut words = SavedWordsBuilder::default();
        for position in positions {
            words.push([position]);
        }
        Self { positions: words }
    }
}

#[derive(Clone, Copy, Default, serde::Deserialize)]
pub struct SavedPositions {
    #[serde(borrow)]
    positions: &'static [u8],
}

impl SavedPositions {
    pub fn at(&self, index: usize) -> usize {
        let [position] = self.words().row(index);
        position as usize
    }

    pub fn count(&self) -> usize {
        self.words().rows()
    }

    fn words(&self) -> SavedWords<1> {
        SavedWords::new(self.positions)
    }
}

#[cfg(test)]
mod tests {
    use super::{PositionBytes, SavedPositions};

    #[test]
    fn a_saved_position_reads_back_at_its_index() {
        let bytes = bincode::serialize(&PositionBytes::of([4, 0, 70_000].into_iter()))
            .expect("the positions save");
        let saved: SavedPositions = bincode::deserialize(bytes.leak()).expect("the positions load");
        assert_eq!(saved.count(), 3);
        assert_eq!([saved.at(0), saved.at(1), saved.at(2)], [4, 0, 70_000]);
    }
}
