use super::saved_key::SavedKey;
use super::saved_words::{SavedWords, SavedWordsBuilder};

#[derive(Clone, Copy)]
pub(super) struct NameSlots {
    positions: SavedWords<1>,
}

impl NameSlots {
    const EMPTY: usize = 0;

    pub(super) fn new(bytes: &'static [u8]) -> Self {
        Self {
            positions: SavedWords::new(bytes),
        }
    }

    pub(super) fn find<'a>(
        &self,
        key: impl SavedKey<'a>,
        holds_key: impl Fn(usize) -> bool,
    ) -> Option<usize> {
        let count = self.positions.rows();
        if count == 0 {
            return None;
        }
        let mask = count - 1;
        let mut slot = key.first_slot(mask);
        loop {
            let [claimed] = self.positions.row(slot);
            let claimed = claimed as usize;
            if claimed == Self::EMPTY {
                return None;
            }
            let row = claimed - 1;
            if holds_key(row) {
                return Some(row);
            }
            slot = (slot + 1) & mask;
        }
    }
}

pub(super) struct NameSlotsBuilder {
    claimed: Vec<usize>,
}

impl NameSlotsBuilder {
    const LOAD_DIVISOR: usize = 2;

    pub(super) fn for_count(names: usize) -> Self {
        let capacity = (names * Self::LOAD_DIVISOR).max(1).next_power_of_two();
        Self {
            claimed: vec![NameSlots::EMPTY; capacity],
        }
    }

    pub(super) fn claim<'a>(&mut self, key: impl SavedKey<'a>, position: usize) {
        let mask = self.claimed.len() - 1;
        let mut slot = key.first_slot(mask);
        while self.claimed[slot] != NameSlots::EMPTY {
            slot = (slot + 1) & mask;
        }
        self.claimed[slot] = position + 1;
    }

    pub(super) fn words(&self) -> SavedWordsBuilder<1> {
        let mut words = SavedWordsBuilder::default();
        for claimed in &self.claimed {
            words.push([*claimed]);
        }
        words
    }
}
