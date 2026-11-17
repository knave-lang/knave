use super::name_key::NameKey;
use crate::core::Namespace;

pub(crate) struct NameSlots;

impl NameSlots {
    pub(crate) const WORD: usize = size_of::<u32>();
    const EMPTY: u32 = 0;
    const LOAD_DIVISOR: usize = 2;
    const HASH_FACTOR: u64 = 0x517c_c1b7_2722_0a95;
    const HASH_ROTATION: u32 = 5;
    const HASH_FINAL_ROTATION: u32 = 26;
    const HASH_CHUNK: usize = size_of::<u64>();

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

    pub(crate) fn code(namespace: Namespace) -> u32 {
        match namespace {
            Namespace::Value => 0,
            Namespace::Type => 1,
        }
    }

    pub(crate) fn namespace(code: u32) -> Namespace {
        match code {
            0 => Namespace::Value,
            _ => Namespace::Type,
        }
    }

    pub(crate) fn index(value: usize) -> u32 {
        u32::try_from(value).expect("the saved prelude stays under 4 GiB")
    }

    fn hash(key: NameKey<'_>) -> u64 {
        let chunks = key.name.as_bytes().chunks(Self::HASH_CHUNK);
        let hash = chunks.fold(0, |hash, chunk| Self::mix(hash, Self::chunk_word(chunk)));
        Self::mix(hash, Self::closing_word(key.name.len(), key.namespace))
            .rotate_left(Self::HASH_FINAL_ROTATION)
    }

    fn closing_word(length: usize, namespace: Namespace) -> u64 {
        (length as u64) << u32::BITS | u64::from(Self::code(namespace))
    }

    fn mix(hash: u64, word: u64) -> u64 {
        (hash.rotate_left(Self::HASH_ROTATION) ^ word).wrapping_mul(Self::HASH_FACTOR)
    }

    fn chunk_word(chunk: &[u8]) -> u64 {
        let mut word = [0; Self::HASH_CHUNK];
        word[..chunk.len()].copy_from_slice(chunk);
        u64::from_le_bytes(word)
    }

    fn start(key: NameKey<'_>, mask: usize) -> usize {
        let wide_mask = u64::try_from(mask).expect("a table's mask fits 64 bits");
        usize::try_from(Self::hash(key) & wide_mask).expect("a slot fits its mask")
    }

    fn capacity(names: usize) -> usize {
        (names * Self::LOAD_DIVISOR).max(1).next_power_of_two()
    }
}

#[cfg(test)]
mod tests {
    use super::NameSlots;
    use crate::core::Namespace;
    use crate::scope::name_key::NameKey;

    #[test]
    fn a_slot_does_not_depend_on_the_word_size() {
        const MAP_VALUE_HASH: u64 = 0xbac4a085bbe42db9;
        assert_eq!(
            NameSlots::hash(NameKey::new("map", Namespace::Value)),
            MAP_VALUE_HASH
        );
    }

    #[test]
    fn the_namespace_changes_a_slot() {
        const MAP_TYPE_HASH: u64 = 0x976128b00d2a20c0;
        assert_eq!(
            NameSlots::hash(NameKey::new("map", Namespace::Type)),
            MAP_TYPE_HASH
        );
    }

    #[test]
    fn a_name_of_exactly_two_chunks_has_a_fixed_hash() {
        const TWO_CHUNK_HASH: u64 = 0x1d2f75f1d58f090e;
        let hash = NameSlots::hash(NameKey::new("sixteen-bytes-ok", Namespace::Value));
        assert_eq!(hash, TWO_CHUNK_HASH);
    }

    #[test]
    fn a_name_past_two_chunks_has_a_fixed_hash() {
        const THREE_CHUNK_HASH: u64 = 0xb8e2b8c46ab4ebfc;
        let hash = NameSlots::hash(NameKey::new("eight-by-tail-one", Namespace::Value));
        assert_eq!(hash, THREE_CHUNK_HASH);
    }

    #[test]
    fn a_trailing_zero_byte_changes_the_hash() {
        let hash = |name| NameSlots::hash(NameKey::new(name, Namespace::Value));
        assert_ne!(hash("a"), hash("a\0"));
        assert_ne!(hash("abcdefgh"), hash("abcdefgh\0"));
    }

    #[test]
    fn a_long_name_hashes_every_chunk() {
        let hash = |name| NameSlots::hash(NameKey::new(name, Namespace::Value));
        assert_ne!(hash("aight-by-tail-one"), hash("eight-by-tail-one"));
        assert_ne!(hash("eight-by-tail-one"), hash("eight-by-tail-two"));
    }
}
