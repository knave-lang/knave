use std::cell::OnceCell;
use std::ops::Range;
use std::rc::Rc;

use super::binding_table::NameEntries;
use super::{Scope, ScopeAllocator, ScopeSet};
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
    const WORD: usize = size_of::<u32>();
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

struct NameSlots;

impl NameSlots {
    const EMPTY: u32 = 0;
    const LOAD_DIVISOR: usize = 2;

    fn word(bytes: &[u8], index: usize) -> u32 {
        let start = index * NameRow::WORD;
        let word = &bytes[start..start + NameRow::WORD];
        u32::from_le_bytes(word.try_into().expect("a saved word has four bytes"))
    }

    const HASH_FACTOR: u64 = 0x517c_c1b7_2722_0a95;
    const HASH_ROTATION: u32 = 5;
    const HASH_FINAL_ROTATION: u32 = 26;
    const HASH_CHUNK: usize = size_of::<u64>();

    fn hash(name: &str, namespace: Namespace) -> u64 {
        let chunks = name.as_bytes().chunks(Self::HASH_CHUNK);
        let hash = chunks.fold(0, |hash, chunk| Self::mix(hash, Self::chunk_word(chunk)));
        Self::mix(hash, Self::closing_word(name.len(), namespace))
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

    fn start(name: &str, namespace: Namespace, mask: usize) -> usize {
        let wide_mask = u64::try_from(mask).expect("a table's mask fits 64 bits");
        usize::try_from(Self::hash(name, namespace) & wide_mask).expect("a slot fits its mask")
    }

    fn code(namespace: Namespace) -> u32 {
        match namespace {
            Namespace::Value => 0,
            Namespace::Type => 1,
        }
    }

    fn namespace(code: u32) -> Namespace {
        match code {
            0 => Namespace::Value,
            _ => Namespace::Type,
        }
    }

    fn capacity(names: usize) -> usize {
        (names * Self::LOAD_DIVISOR).max(1).next_power_of_two()
    }

    fn index(value: usize) -> u32 {
        u32::try_from(value).expect("the saved prelude stays under 4 GiB")
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
            slots: vec![0; NameSlots::capacity(entries.len()) * NameRow::WORD],
            rows: Vec::new(),
            names: Vec::new(),
            entries: Vec::new(),
        };
        let mut sorted = entries;
        sorted.sort_by(|left, right| {
            (&left.0, NameSlots::code(left.1)).cmp(&(&right.0, NameSlots::code(right.1)))
        });
        for (position, (name, namespace, entries)) in sorted.iter().enumerate() {
            tables.add(name, *namespace, entries);
            tables.claim_slot(name, *namespace, position);
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
            namespace: NameSlots::code(namespace),
            entries_start,
            entries_end: NameSlots::index(self.entries.len()),
        };
        row.write(&mut self.rows);
    }

    fn claim_slot(&mut self, name: &str, namespace: Namespace, position: usize) {
        let mask = self.slots.len() / NameRow::WORD - 1;
        let mut slot = NameSlots::start(name, namespace, mask);
        while NameSlots::word(&self.slots, slot) != NameSlots::EMPTY {
            slot = (slot + 1) & mask;
        }
        let claimed = NameSlots::index(position + 1).to_le_bytes();
        self.slots[slot * NameRow::WORD..(slot + 1) * NameRow::WORD].copy_from_slice(&claimed);
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
        let nothing = ScopeAllocator::default().next_scope();
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
        name: &str,
        namespace: Namespace,
    ) -> Option<&NameEntries> {
        if scopes.may_hold_scope_in(&self.prelude_scopes) {
            self.entries(name, namespace)
        } else {
            None
        }
    }

    pub(super) fn entries(&self, name: &str, namespace: Namespace) -> Option<&NameEntries> {
        let count = self.slots.len() / NameRow::WORD;
        if count == 0 {
            return None;
        }
        let mask = count - 1;
        let mut slot = NameSlots::start(name, namespace, mask);
        loop {
            let position = NameSlots::word(self.slots, slot);
            if position == NameSlots::EMPTY {
                return None;
            }
            let row = position as usize - 1;
            if self.holds_key(row, name, namespace) {
                return Some(self.decoded_row(row));
            }
            slot = (slot + 1) & mask;
        }
    }

    pub(super) fn all(&self) -> impl Iterator<Item = (Rc<str>, Namespace, &NameEntries)> {
        (0..self.decoded.len()).map(|row| {
            let saved = self.row(row);
            let name: Rc<str> = Rc::from(self.name_of(saved));
            (
                name,
                NameSlots::namespace(saved.namespace),
                self.decoded_row(row),
            )
        })
    }

    pub(super) fn all_of(&self, namespace: Namespace) -> impl Iterator<Item = &NameEntries> {
        let wanted = NameSlots::code(namespace);
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
        saved.namespace == NameSlots::code(namespace) && self.name_bytes(saved) == name.as_bytes()
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

    use super::{NameSlots, NameTables, SavedNames};
    use crate::core::Namespace;
    use crate::scope::binding_table::NameEntries;
    use crate::scope::{Scope, ScopeAllocator};

    struct SavedNamesFixture;

    impl SavedNamesFixture {
        fn no_scopes() -> Range<Scope> {
            let nothing = ScopeAllocator::default().next_scope();
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
    fn a_slot_does_not_depend_on_the_word_size() {
        const MAP_VALUE_HASH: u64 = 0xbac4a085bbe42db9;
        assert_eq!(NameSlots::hash("map", Namespace::Value), MAP_VALUE_HASH);
    }

    #[test]
    fn the_namespace_changes_a_slot() {
        const MAP_TYPE_HASH: u64 = 0x976128b00d2a20c0;
        assert_eq!(NameSlots::hash("map", Namespace::Type), MAP_TYPE_HASH);
    }

    #[test]
    fn a_name_of_exactly_two_chunks_has_a_fixed_hash() {
        const TWO_CHUNK_HASH: u64 = 0x1d2f75f1d58f090e;
        let hash = NameSlots::hash("sixteen-bytes-ok", Namespace::Value);
        assert_eq!(hash, TWO_CHUNK_HASH);
    }

    #[test]
    fn a_name_past_two_chunks_has_a_fixed_hash() {
        const THREE_CHUNK_HASH: u64 = 0xb8e2b8c46ab4ebfc;
        let hash = NameSlots::hash("eight-by-tail-one", Namespace::Value);
        assert_eq!(hash, THREE_CHUNK_HASH);
    }

    #[test]
    fn a_trailing_zero_byte_changes_the_hash() {
        let hash = |name| NameSlots::hash(name, Namespace::Value);
        assert_ne!(hash("a"), hash("a\0"));
        assert_ne!(hash("abcdefgh"), hash("abcdefgh\0"));
    }

    #[test]
    fn a_long_name_hashes_every_chunk() {
        let hash = |name| NameSlots::hash(name, Namespace::Value);
        assert_ne!(hash("aight-by-tail-one"), hash("eight-by-tail-one"));
        assert_ne!(hash("eight-by-tail-one"), hash("eight-by-tail-two"));
    }

    #[test]
    fn a_saved_name_is_found_in_its_namespace_only() {
        let saved = SavedNamesFixture::saved(&[("map", Namespace::Value)]);
        assert!(saved.entries("map", Namespace::Value).is_some());
        assert!(saved.entries("map", Namespace::Type).is_none());
    }

    #[test]
    fn one_name_in_two_namespaces_keeps_two_rows() {
        let saved =
            SavedNamesFixture::saved(&[("str", Namespace::Value), ("str", Namespace::Type)]);
        assert!(saved.entries("str", Namespace::Value).is_some());
        assert!(saved.entries("str", Namespace::Type).is_some());
        assert_eq!(saved.all().count(), 2);
    }

    #[test]
    fn a_missing_name_is_not_found() {
        let saved = SavedNamesFixture::saved(&[("map", Namespace::Value)]);
        assert!(saved.entries("filter", Namespace::Value).is_none());
    }

    #[test]
    fn every_name_of_a_full_table_is_found() {
        let names: Vec<String> = (0..200).map(|index| format!("name-{index}")).collect();
        let keys: Vec<(&str, Namespace)> = names
            .iter()
            .map(|name| (name.as_str(), Namespace::Value))
            .collect();
        let saved = SavedNamesFixture::saved(&keys);
        assert!(
            names
                .iter()
                .all(|name| saved.entries(name, Namespace::Value).is_some())
        );
    }

    #[test]
    fn an_empty_table_finds_nothing() {
        assert!(
            SavedNames::default()
                .entries("map", Namespace::Value)
                .is_none()
        );
    }

    #[test]
    fn a_lookup_decodes_only_its_name() {
        let saved =
            SavedNamesFixture::saved(&[("map", Namespace::Value), ("filter", Namespace::Value)]);
        saved.entries("map", Namespace::Value);
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
