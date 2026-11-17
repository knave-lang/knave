use super::saved_namespace::SavedNamespace;
use crate::core::Namespace;

#[derive(Clone, Copy)]
pub(super) struct NameKey<'a> {
    name: &'a str,
    namespace: Namespace,
    hash: u64,
}

impl<'a> NameKey<'a> {
    const HASH_FACTOR: u64 = 0x517c_c1b7_2722_0a95;
    const HASH_ROTATION: u32 = 5;
    const HASH_FINAL_ROTATION: u32 = 26;
    const HASH_CHUNK: usize = size_of::<u64>();

    pub(super) fn new(name: &'a str, namespace: Namespace) -> Self {
        Self {
            name,
            namespace,
            hash: Self::hash(name, namespace),
        }
    }

    pub(super) fn name(self) -> &'a str {
        self.name
    }

    pub(super) fn namespace(self) -> Namespace {
        self.namespace
    }

    pub(super) fn first_slot(self, mask: usize) -> usize {
        let wide_mask = u64::try_from(mask).expect("a table's mask fits 64 bits");
        usize::try_from(self.hash & wide_mask).expect("a slot fits its mask")
    }

    fn hash(name: &str, namespace: Namespace) -> u64 {
        let chunks = name.as_bytes().chunks(Self::HASH_CHUNK);
        let hash = chunks.fold(0, |hash, chunk| Self::mix(hash, Self::chunk_word(chunk)));
        Self::mix(hash, Self::closing_word(name.len(), namespace))
            .rotate_left(Self::HASH_FINAL_ROTATION)
    }

    fn closing_word(length: usize, namespace: Namespace) -> u64 {
        (length as u64) << u32::BITS | u64::from(SavedNamespace::of(namespace).code())
    }

    fn mix(hash: u64, word: u64) -> u64 {
        (hash.rotate_left(Self::HASH_ROTATION) ^ word).wrapping_mul(Self::HASH_FACTOR)
    }

    fn chunk_word(chunk: &[u8]) -> u64 {
        let mut word = [0; Self::HASH_CHUNK];
        word[..chunk.len()].copy_from_slice(chunk);
        u64::from_le_bytes(word)
    }
}

#[cfg(test)]
mod tests {
    use super::NameKey;
    use crate::core::Namespace;

    #[test]
    fn a_slot_does_not_depend_on_the_word_size() {
        const MAP_VALUE_HASH: u64 = 0xbac4a085bbe42db9;
        assert_eq!(NameKey::new("map", Namespace::Value).hash, MAP_VALUE_HASH);
    }

    #[test]
    fn the_namespace_changes_a_slot() {
        const MAP_TYPE_HASH: u64 = 0x976128b00d2a20c0;
        assert_eq!(NameKey::new("map", Namespace::Type).hash, MAP_TYPE_HASH);
    }

    #[test]
    fn a_name_of_exactly_two_chunks_has_a_fixed_hash() {
        const TWO_CHUNK_HASH: u64 = 0x1d2f75f1d58f090e;
        let hash = NameKey::new("sixteen-bytes-ok", Namespace::Value).hash;
        assert_eq!(hash, TWO_CHUNK_HASH);
    }

    #[test]
    fn a_name_past_two_chunks_has_a_fixed_hash() {
        const THREE_CHUNK_HASH: u64 = 0xb8e2b8c46ab4ebfc;
        let hash = NameKey::new("eight-by-tail-one", Namespace::Value).hash;
        assert_eq!(hash, THREE_CHUNK_HASH);
    }

    #[test]
    fn a_trailing_zero_byte_changes_the_hash() {
        let hash = |name| NameKey::new(name, Namespace::Value).hash;
        assert_ne!(hash("a"), hash("a\0"));
        assert_ne!(hash("abcdefgh"), hash("abcdefgh\0"));
    }

    #[test]
    fn a_long_name_hashes_every_chunk() {
        let hash = |name| NameKey::new(name, Namespace::Value).hash;
        assert_ne!(hash("aight-by-tail-one"), hash("eight-by-tail-one"));
        assert_ne!(hash("eight-by-tail-one"), hash("eight-by-tail-two"));
    }
}
