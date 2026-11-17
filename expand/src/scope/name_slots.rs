use super::name_key::NameKey;

pub(crate) struct NameSlots;

impl NameSlots {
    pub(crate) const WORD: usize = size_of::<u32>();
    const EMPTY: u32 = 0;
    const LOAD_DIVISOR: usize = 2;

    pub(crate) fn empty(names: usize) -> Vec<u8> {
        vec![0; Self::capacity(names) * Self::WORD]
    }

    pub(crate) fn claim(slots: &mut [u8], key: NameKey<'_>, position: usize) {
        let mask = slots.len() / Self::WORD - 1;
        let mut slot = Self::start(key, mask);
        while Self::word(slots, slot) != Self::EMPTY {
            slot = (slot + 1) & mask;
        }
        let claimed = Self::index(position + 1).to_le_bytes();
        slots[slot * Self::WORD..(slot + 1) * Self::WORD].copy_from_slice(&claimed);
    }

    pub(crate) fn find(
        slots: &[u8],
        key: NameKey<'_>,
        holds_key: impl Fn(usize) -> bool,
    ) -> Option<usize> {
        let count = slots.len() / Self::WORD;
        if count == 0 {
            return None;
        }
        let mask = count - 1;
        let mut slot = Self::start(key, mask);
        loop {
            let position = Self::word(slots, slot);
            if position == Self::EMPTY {
                return None;
            }
            let row = position as usize - 1;
            if holds_key(row) {
                return Some(row);
            }
            slot = (slot + 1) & mask;
        }
    }

    pub(crate) fn word(bytes: &[u8], index: usize) -> u32 {
        let start = index * Self::WORD;
        let word = &bytes[start..start + Self::WORD];
        u32::from_le_bytes(word.try_into().expect("a saved word has four bytes"))
    }

    pub(crate) fn index(value: usize) -> u32 {
        u32::try_from(value).expect("the saved prelude stays under 4 GiB")
    }

    fn start(key: NameKey<'_>, mask: usize) -> usize {
        let wide_mask = u64::try_from(mask).expect("a table's mask fits 64 bits");
        usize::try_from(key.hash & wide_mask).expect("a slot fits its mask")
    }

    fn capacity(names: usize) -> usize {
        (names * Self::LOAD_DIVISOR).max(1).next_power_of_two()
    }
}
