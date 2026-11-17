use super::name_key::NameKey;
use super::name_slots::{NameSlots, NameSlotsBuilder};
use super::saved_words::{SavedWords, SavedWordsBuilder};
use crate::core::Namespace;

#[derive(Clone, Copy, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub(super) struct SavedKeys {
    #[serde(borrow)]
    slots: &'static [u8],
    #[serde(borrow)]
    rows: &'static [u8],
    #[serde(borrow)]
    names: &'static [u8],
}

impl SavedKeys {
    pub(super) fn position(&self, key: NameKey<'_>) -> Option<usize> {
        NameSlots::new(self.slots).find(key, |position| self.holds(position, key))
    }

    pub(super) fn key(&self, position: usize) -> (&'static str, Namespace) {
        let [name_start, name_end, namespace] = self.rows().row(position);
        let name = &self.names[name_start as usize..name_end as usize];
        let name = std::str::from_utf8(name).expect("a saved name is text");
        (name, NameKey::namespace(namespace))
    }

    pub(super) fn count(&self) -> usize {
        self.rows().rows()
    }

    fn holds(&self, position: usize, key: NameKey<'_>) -> bool {
        let [name_start, name_end, namespace] = self.rows().row(position);
        namespace == NameKey::code(key.namespace)
            && &self.names[name_start as usize..name_end as usize] == key.name.as_bytes()
    }

    fn rows(&self) -> SavedWords<3> {
        SavedWords::new(self.rows)
    }
}

pub(super) struct SavedKeysBuilder {
    slots: NameSlotsBuilder,
    rows: SavedWordsBuilder,
    names: Vec<u8>,
    count: usize,
}

impl SavedKeysBuilder {
    pub(super) fn for_count(count: usize) -> Self {
        Self {
            slots: NameSlotsBuilder::for_count(count),
            rows: SavedWordsBuilder::default(),
            names: Vec::new(),
            count: 0,
        }
    }

    pub(super) fn add(&mut self, key: NameKey<'_>) {
        let name_start = self.names.len();
        self.names.extend(key.name.as_bytes());
        let namespace = NameKey::code(key.namespace) as usize;
        self.rows.push(&[name_start, self.names.len(), namespace]);
        self.slots.claim(key, self.count);
        self.count += 1;
    }
}

impl serde::Serialize for SavedKeysBuilder {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        (&self.slots.words(), &self.rows, &self.names).serialize(serializer)
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
    fn a_key_holds_only_a_lookup_in_its_own_namespace() {
        let keys = KeysFixture::saved(&[("map", Namespace::Value)]);
        assert!(keys.holds(0, NameKey::new("map", Namespace::Value)));
        assert!(!keys.holds(0, NameKey::new("map", Namespace::Type)));
        assert!(!keys.holds(0, NameKey::new("mop", Namespace::Value)));
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
    fn an_empty_table_finds_nothing() {
        let keys = KeysFixture::saved(&[]);
        assert_eq!(keys.count(), 0);
        assert_eq!(KeysFixture::position(&keys, "map", Namespace::Value), None);
    }
}
