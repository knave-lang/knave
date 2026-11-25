use super::saved_key::SavedKey;
use super::saved_namespace::SavedNamespace;
use super::text_hash::TextHash;
use crate::core::Namespace;

#[derive(Clone, Copy)]
pub(super) struct NameKey<'a> {
    name: &'a str,
    namespace: Namespace,
    hash: u64,
}

impl NameKey<'_> {
    pub(super) const ROW_WIDTH: usize = 3;
    pub(super) const NAMESPACE_WORD: usize = 2;
}

impl<'a> NameKey<'a> {
    pub(super) fn new(name: &'a str, namespace: Namespace) -> Self {
        Self {
            name,
            namespace,
            hash: TextHash::of(name, SavedNamespace::of(namespace).code()),
        }
    }

    pub(super) fn namespace(self) -> Namespace {
        self.namespace
    }
}

impl<'a> SavedKey<'a> for NameKey<'a> {
    fn text(self) -> &'a str {
        self.name
    }

    fn first_slot(self, mask: usize) -> usize {
        TextHash::first_slot(self.hash, mask)
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
