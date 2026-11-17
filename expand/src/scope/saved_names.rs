use std::cell::OnceCell;
use std::ops::Range;
use std::rc::Rc;

use super::name_entries::NameEntries;
use super::name_key::NameKey;
use super::name_slots::NameSlots;
use super::{Scope, ScopeSet, Scopes};
use crate::core::Namespace;

type SavedParts = (
    Range<Scope>,
    &'static [u8],
    &'static [u8],
    &'static [u8],
    &'static [u8],
);

#[derive(Clone, Copy)]
struct NameRow {
    name_start: u32,
    name_end: u32,
    namespace: u32,
    entries_start: u32,
    entries_end: u32,
}

impl NameRow {
    const WORD: usize = NameSlots::WORD;
    const WORDS: usize = 5;
    const SIZE: usize = Self::WORDS * Self::WORD;

    fn read(bytes: &[u8]) -> Self {
        let word = |index: usize| NameSlots::word(bytes, index);
        Self {
            name_start: word(0),
            name_end: word(1),
            namespace: word(2),
            entries_start: word(3),
            entries_end: word(4),
        }
    }

    fn write(self, bytes: &mut Vec<u8>) {
        let words = [
            self.name_start,
            self.name_end,
            self.namespace,
            self.entries_start,
            self.entries_end,
        ];
        for word in words {
            bytes.extend(word.to_le_bytes());
        }
    }
}

#[derive(serde::Serialize)]
pub struct NameTables {
    prelude_scopes: Range<Scope>,
    slots: Vec<u8>,
    rows: Vec<u8>,
    names: Vec<u8>,
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
            slots: NameSlots::empty(entries.len()),
            rows: Vec::new(),
            names: Vec::new(),
            entries: Vec::new(),
        };
        let mut sorted = entries;
        sorted.sort_by(|left, right| {
            (&left.0, NameKey::code(left.1)).cmp(&(&right.0, NameKey::code(right.1)))
        });
        for (position, (name, namespace, entries)) in sorted.iter().enumerate() {
            tables.add(name, *namespace, entries);
            NameSlots::claim(&mut tables.slots, NameKey::new(name, *namespace), position);
        }
        tables
    }

    fn add(&mut self, name: &str, namespace: Namespace, entries: &NameEntries) {
        let name_start = NameSlots::index(self.names.len());
        self.names.extend(name.as_bytes());
        let name_end = NameSlots::index(self.names.len());
        let entries_start = NameSlots::index(self.entries.len());
        self.entries
            .extend(bincode::serialize(entries).expect("every name entry serializes"));
        let row = NameRow {
            name_start,
            name_end,
            namespace: NameKey::code(namespace),
            entries_start,
            entries_end: NameSlots::index(self.entries.len()),
        };
        row.write(&mut self.rows);
    }
}

pub struct SavedNames {
    prelude_scopes: Range<Scope>,
    slots: &'static [u8],
    rows: &'static [u8],
    names: &'static [u8],
    entries: &'static [u8],
    decoded: Vec<OnceCell<Box<NameEntries>>>,
}

impl<'de: 'static> serde::Deserialize<'de> for SavedNames {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let (prelude_scopes, slots, rows, names, entries): SavedParts =
            serde::Deserialize::deserialize(deserializer)?;
        let count = rows.len() / NameRow::SIZE;
        Ok(Self {
            prelude_scopes,
            slots,
            rows,
            names,
            entries,
            decoded: (0..count).map(|_| OnceCell::new()).collect(),
        })
    }
}

impl Default for SavedNames {
    fn default() -> Self {
        let nothing = Scopes::default().next_scope();
        Self {
            prelude_scopes: nothing..nothing,
            slots: &[],
            rows: &[],
            names: &[],
            entries: &[],
            decoded: Vec::new(),
        }
    }
}

impl SavedNames {
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
        let row = NameSlots::find(self.slots, key, |row| {
            self.holds_key(row, key.name, key.namespace)
        })?;
        Some(self.decoded_row(row))
    }

    pub(super) fn all(&self) -> impl Iterator<Item = (Rc<str>, Namespace, &NameEntries)> {
        (0..self.decoded.len()).map(|row| {
            let saved = self.row(row);
            let name: Rc<str> = Rc::from(self.name_of(saved));
            (
                name,
                NameKey::namespace(saved.namespace),
                self.decoded_row(row),
            )
        })
    }

    pub(super) fn all_of(&self, namespace: Namespace) -> impl Iterator<Item = &NameEntries> {
        let wanted = NameKey::code(namespace);
        (0..self.decoded.len())
            .filter(move |row| self.row(*row).namespace == wanted)
            .map(|row| self.decoded_row(row))
    }

    #[cfg(test)]
    pub(super) fn is_decoded(&self, name: &str, namespace: Namespace) -> bool {
        let found = (0..self.decoded.len()).find(|row| self.holds_key(*row, name, namespace));
        found.is_some_and(|row| self.decoded[row].get().is_some())
    }

    fn row(&self, index: usize) -> NameRow {
        NameRow::read(&self.rows[index * NameRow::SIZE..])
    }

    fn holds_key(&self, row: usize, name: &str, namespace: Namespace) -> bool {
        let saved = self.row(row);
        saved.namespace == NameKey::code(namespace) && self.name_bytes(saved) == name.as_bytes()
    }

    fn name_bytes(&self, row: NameRow) -> &'static [u8] {
        &self.names[row.name_start as usize..row.name_end as usize]
    }

    fn name_of(&self, row: NameRow) -> &'static str {
        std::str::from_utf8(self.name_bytes(row)).expect("a saved name is text")
    }

    fn decoded_row(&self, index: usize) -> &NameEntries {
        self.decoded[index].get_or_init(|| Box::new(self.decode(index)))
    }

    fn decode(&self, index: usize) -> NameEntries {
        let row = self.row(index);
        let encoded = &self.entries[row.entries_start as usize..row.entries_end as usize];
        bincode::deserialize(encoded).expect("a saved name entry decodes")
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
    fn a_saved_name_is_found_in_its_namespace_only() {
        let saved = SavedNamesFixture::saved(&[("map", Namespace::Value)]);
        assert!(
            saved
                .entries(NameKey::new("map", Namespace::Value))
                .is_some()
        );
        assert!(
            saved
                .entries(NameKey::new("map", Namespace::Type))
                .is_none()
        );
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
    fn a_missing_name_is_not_found() {
        let saved = SavedNamesFixture::saved(&[("map", Namespace::Value)]);
        assert!(
            saved
                .entries(NameKey::new("filter", Namespace::Value))
                .is_none()
        );
    }

    #[test]
    fn every_name_of_a_full_table_is_found() {
        let names: Vec<String> = (0..200).map(|index| format!("name-{index}")).collect();
        let keys: Vec<(&str, Namespace)> = names
            .iter()
            .map(|name| (name.as_str(), Namespace::Value))
            .collect();
        let saved = SavedNamesFixture::saved(&keys);
        assert!(names.iter().all(|name| {
            saved
                .entries(NameKey::new(name, Namespace::Value))
                .is_some()
        }));
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
