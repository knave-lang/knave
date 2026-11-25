use super::name_key::NameKey;
use super::saved_namespace::SavedNamespace;
use super::saved_texts::{SavedTexts, SavedTextsBuilder};
use crate::core::Namespace;

#[derive(Clone, Copy, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(bound(deserialize = "'de: 'static"))]
pub(super) struct SavedKeys {
    texts: SavedTexts<{ NameKey::ROW_WIDTH }>,
}

impl SavedKeys {
    pub(super) fn position(&self, key: NameKey<'_>) -> Option<usize> {
        let namespace = SavedNamespace::of(key.namespace());
        self.texts
            .position(key, |row| Self::namespace(row) == namespace)
    }

    pub(super) fn key(&self, position: usize) -> (&'static str, Namespace) {
        let namespace = Self::namespace(self.texts.row(position));
        (self.texts.text(position), namespace.namespace())
    }

    pub(super) fn count(&self) -> usize {
        self.texts.count()
    }

    fn namespace(row: [u32; NameKey::ROW_WIDTH]) -> SavedNamespace {
        SavedNamespace::read(row[NameKey::NAMESPACE_WORD])
    }
}

#[derive(serde::Serialize)]
pub(super) struct SavedKeysBuilder {
    texts: SavedTextsBuilder<{ NameKey::ROW_WIDTH }>,
}

impl SavedKeysBuilder {
    pub(super) fn for_count(count: usize) -> Self {
        Self {
            texts: SavedTextsBuilder::for_count(count),
        }
    }

    pub(super) fn add(&mut self, key: NameKey<'_>) {
        let mut row = [0; NameKey::ROW_WIDTH];
        row[NameKey::NAMESPACE_WORD] = SavedNamespace::of(key.namespace()).code() as usize;
        self.texts.add(key, row);
    }
}

#[cfg(test)]
mod tests {
    use super::{SavedKeys, SavedKeysBuilder};
    use crate::core::Namespace;
    use crate::scope::name_key::NameKey;

    struct KeysFixture;

    impl KeysFixture {
        fn saved(keys: &[(&str, Namespace)]) -> SavedKeys {
            let mut builder = SavedKeysBuilder::for_count(keys.len());
            for (name, namespace) in keys {
                builder.add(NameKey::new(name, *namespace));
            }
            let bytes = bincode::serialize(&builder).expect("the keys save");
            bincode::deserialize(bytes.leak()).expect("the keys load")
        }

        fn position(keys: &SavedKeys, name: &str, namespace: Namespace) -> Option<usize> {
            keys.position(NameKey::new(name, namespace))
        }
    }

    #[test]
    fn a_saved_key_is_found_in_its_namespace_only() {
        let keys = KeysFixture::saved(&[("map", Namespace::Value)]);
        assert_eq!(
            KeysFixture::position(&keys, "map", Namespace::Value),
            Some(0)
        );
        assert_eq!(KeysFixture::position(&keys, "map", Namespace::Type), None);
    }

    #[test]
    fn a_key_of_the_same_length_and_namespace_is_not_found() {
        let keys = KeysFixture::saved(&[("map", Namespace::Value)]);
        assert_eq!(KeysFixture::position(&keys, "mop", Namespace::Value), None);
    }

    #[test]
    fn one_name_in_two_namespaces_keeps_two_keys() {
        let keys = KeysFixture::saved(&[("str", Namespace::Value), ("str", Namespace::Type)]);
        assert_eq!(
            KeysFixture::position(&keys, "str", Namespace::Type),
            Some(1)
        );
        assert_eq!(keys.key(1), ("str", Namespace::Type));
    }

    #[test]
    fn a_missing_name_is_not_found() {
        let keys = KeysFixture::saved(&[("map", Namespace::Value)]);
        assert_eq!(
            KeysFixture::position(&keys, "filter", Namespace::Value),
            None
        );
    }

    #[test]
    fn every_name_of_a_full_table_is_found_at_its_position() {
        let names: Vec<String> = (0..200).map(|index| format!("name-{index}")).collect();
        let pairs: Vec<(&str, Namespace)> = names
            .iter()
            .map(|name| (name.as_str(), Namespace::Value))
            .collect();
        let keys = KeysFixture::saved(&pairs);
        let found = names
            .iter()
            .map(|name| KeysFixture::position(&keys, name, Namespace::Value));
        assert!(
            found
                .enumerate()
                .all(|(position, found)| found == Some(position))
        );
    }

    #[test]
    fn a_name_is_not_found_in_the_other_namespace_even_at_its_own_slot() {
        let names: Vec<String> = (0..64).map(|index| format!("name-{index}")).collect();
        let found_as_type = names.iter().filter(|name| {
            let keys = KeysFixture::saved(&[(name.as_str(), Namespace::Value)]);
            KeysFixture::position(&keys, name, Namespace::Type).is_some()
        });
        assert_eq!(found_as_type.count(), 0);
    }

    #[test]
    fn an_empty_table_finds_nothing() {
        let keys = KeysFixture::saved(&[]);
        assert_eq!(keys.count(), 0);
        assert_eq!(KeysFixture::position(&keys, "map", Namespace::Value), None);
    }
}
