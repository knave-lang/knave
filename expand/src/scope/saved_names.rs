use std::cell::OnceCell;
use std::ops::Range;
use std::rc::Rc;

use super::name_entries::NameEntries;
use super::name_key::NameKey;
use super::saved_keys::{SavedKeys, SavedKeysBuilder};
use super::saved_namespace::SavedNamespace;
use super::saved_words::{SavedWords, SavedWordsBuilder};
use super::{Scope, ScopeSet, Scopes};
use crate::core::Namespace;

#[derive(serde::Serialize)]
pub struct NameTables {
    prelude_scopes: Range<Scope>,
    keys: SavedKeysBuilder,
    ranges: SavedWordsBuilder<{ SavedNames::RANGE_WIDTH }>,
    entries: Vec<u8>,
}

impl NameTables {
    pub(super) fn of(
        entries: Vec<(Rc<str>, Namespace, NameEntries)>,
        prelude_scopes: Range<Scope>,
    ) -> Self {
        assert!(
            entries
                .iter()
                .all(|(_, _, held)| held.is_within(&prelude_scopes)),
            "every saved name is bound under a prelude scope"
        );
        let mut tables = Self {
            prelude_scopes,
            keys: SavedKeysBuilder::for_count(entries.len()),
            ranges: SavedWordsBuilder::default(),
            entries: Vec::new(),
        };
        let mut sorted = entries;
        sorted.sort_by(|left, right| {
            (&left.0, SavedNamespace::of(left.1)).cmp(&(&right.0, SavedNamespace::of(right.1)))
        });
        for (name, namespace, entries) in &sorted {
            tables.add(NameKey::new(name, *namespace), entries);
        }
        tables
    }

    fn add(&mut self, key: NameKey<'_>, entries: &NameEntries) {
        self.keys.add(key);
        let entries_start = self.entries.len();
        self.entries
            .extend(bincode::serialize(entries).expect("every name entry serializes"));
        self.ranges.push([entries_start, self.entries.len()]);
    }
}

type SavedParts = (Range<Scope>, SavedKeys, &'static [u8], &'static [u8]);

pub struct SavedNames {
    prelude_scopes: Range<Scope>,
    keys: SavedKeys,
    ranges: SavedWords<{ Self::RANGE_WIDTH }>,
    entries: &'static [u8],
    decoded: Vec<OnceCell<Box<NameEntries>>>,
}

impl<'de: 'static> serde::Deserialize<'de> for SavedNames {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (prelude_scopes, keys, ranges, entries): SavedParts =
            serde::Deserialize::deserialize(deserializer)?;
        Ok(Self {
            prelude_scopes,
            keys,
            ranges: SavedWords::new(ranges),
            entries,
            decoded: (0..keys.count()).map(|_| OnceCell::new()).collect(),
        })
    }
}

impl Default for SavedNames {
    fn default() -> Self {
        let nothing = Scopes::default().next_scope();
        Self {
            prelude_scopes: nothing..nothing,
            keys: SavedKeys::default(),
            ranges: SavedWords::new(&[]),
            entries: &[],
            decoded: Vec::new(),
        }
    }
}

impl SavedNames {
    const RANGE_WIDTH: usize = 2;

    pub(super) fn entries_seen_from(
        &self,
        scopes: &ScopeSet,
        key: NameKey<'_>,
    ) -> Option<&NameEntries> {
        if scopes.may_hold_scope_in(&self.prelude_scopes) {
            self.entries(key)
        } else {
            None
        }
    }

    pub(super) fn entries(&self, key: NameKey<'_>) -> Option<&NameEntries> {
        let position = self.keys.position(key)?;
        Some(self.decoded(position))
    }

    pub(super) fn all(&self) -> impl Iterator<Item = (Rc<str>, Namespace, &NameEntries)> {
        (0..self.decoded.len()).map(|position| {
            let (name, namespace) = self.keys.key(position);
            (Rc::from(name), namespace, self.decoded(position))
        })
    }

    pub(super) fn all_of(&self, namespace: Namespace) -> impl Iterator<Item = &NameEntries> {
        (0..self.decoded.len())
            .filter(move |position| self.keys.key(*position).1 == namespace)
            .map(|position| self.decoded(position))
    }

    #[cfg(test)]
    pub(super) fn is_decoded(&self, name: &str, namespace: Namespace) -> bool {
        let position = self.keys.position(NameKey::new(name, namespace));
        position.is_some_and(|position| self.decoded[position].get().is_some())
    }

    fn decoded(&self, position: usize) -> &NameEntries {
        self.decoded[position].get_or_init(|| Box::new(self.decode(position)))
    }

    fn decode(&self, position: usize) -> NameEntries {
        let [start, end] = self.ranges.row(position);
        bincode::deserialize(&self.entries[start as usize..end as usize])
            .expect("a saved name entry decodes")
    }
}

#[cfg(test)]
mod tests {
    use std::ops::Range;
    use std::rc::Rc;

    use super::{NameTables, SavedNames};
    use crate::core::Namespace;
    use crate::scope::name_entries::NameEntries;
    use crate::scope::name_key::NameKey;
    use crate::scope::{Scope, Scopes};

    struct SavedNamesFixture;

    impl SavedNamesFixture {
        fn no_scopes() -> Range<Scope> {
            let nothing = Scopes::default().next_scope();
            nothing..nothing
        }

        fn saved(keys: &[(&str, Namespace)]) -> SavedNames {
            let entries: Vec<(Rc<str>, Namespace, NameEntries)> = keys
                .iter()
                .map(|(name, namespace)| (Rc::from(*name), *namespace, NameEntries::default()))
                .collect();
            let bytes = bincode::serialize(&NameTables::of(entries, Self::no_scopes()))
                .expect("the tables serialize");
            bincode::deserialize(bytes.leak()).expect("the tables load")
        }
    }

    #[test]
    fn one_name_in_two_namespaces_keeps_two_rows() {
        let saved =
            SavedNamesFixture::saved(&[("str", Namespace::Value), ("str", Namespace::Type)]);
        assert!(
            saved
                .entries(NameKey::new("str", Namespace::Value))
                .is_some()
        );
        assert!(
            saved
                .entries(NameKey::new("str", Namespace::Type))
                .is_some()
        );
        assert_eq!(saved.all().count(), 2);
    }

    #[test]
    fn an_empty_table_finds_nothing() {
        assert!(
            SavedNames::default()
                .entries(NameKey::new("map", Namespace::Value))
                .is_none()
        );
    }

    #[test]
    fn a_lookup_decodes_only_its_name() {
        let saved =
            SavedNamesFixture::saved(&[("map", Namespace::Value), ("filter", Namespace::Value)]);
        saved.entries(NameKey::new("map", Namespace::Value));
        assert!(saved.is_decoded("map", Namespace::Value));
        assert!(!saved.is_decoded("filter", Namespace::Value));
    }

    #[test]
    fn a_listing_by_namespace_skips_the_other_namespace() {
        let saved =
            SavedNamesFixture::saved(&[("map", Namespace::Value), ("vec", Namespace::Type)]);
        assert_eq!(saved.all_of(Namespace::Type).count(), 1);
        assert!(!saved.is_decoded("map", Namespace::Value));
    }
}
