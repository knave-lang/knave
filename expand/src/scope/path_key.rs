use super::saved_key::SavedKey;
use super::text_hash::TextHash;

#[derive(Clone, Copy)]
pub(super) struct PathKey<'a> {
    path: &'a str,
    hash: u64,
}

impl PathKey<'_> {
    pub(super) const ROW_WIDTH: usize = 2;
}

impl<'a> PathKey<'a> {
    pub(super) fn new(path: &'a str) -> Self {
        Self {
            path,
            hash: TextHash::untagged(path),
        }
    }
}

impl<'a> SavedKey<'a> for PathKey<'a> {
    fn text(self) -> &'a str {
        self.path
    }

    fn first_slot(self, mask: usize) -> usize {
        TextHash::first_slot(self.hash, mask)
    }
}

#[cfg(test)]
mod tests {
    use super::PathKey;
    use crate::core::Namespace;
    use crate::scope::name_key::NameKey;
    use crate::scope::saved_key::SavedKey;

    struct KeyFixture;

    impl KeyFixture {
        const WHOLE_HASH: usize = usize::MAX;

        fn hash<'a>(key: impl SavedKey<'a>) -> usize {
            key.first_slot(Self::WHOLE_HASH)
        }
    }

    #[test]
    fn a_path_key_and_a_name_key_of_one_text_hash_apart() {
        let path = KeyFixture::hash(PathKey::new("prelude/text"));
        let value = KeyFixture::hash(NameKey::new("prelude/text", Namespace::Value));
        let type_name = KeyFixture::hash(NameKey::new("prelude/text", Namespace::Type));
        assert_ne!(path, value);
        assert_ne!(path, type_name);
    }
}
