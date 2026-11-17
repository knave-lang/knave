use std::collections::hash_map;
use std::ops::Range;
use std::rc::Rc;

use rustc_hash::FxHashMap;

use crate::core::{BindingId, Namespace};
use crate::scope::{ExportBytes, SavedExports};

pub enum ExportTable {
    Built(FxHashMap<(Rc<str>, Namespace), BindingId>),
    Saved(SavedExports),
}

impl Default for ExportTable {
    fn default() -> Self {
        Self::Built(FxHashMap::default())
    }
}

impl ExportTable {
    pub fn insert(&mut self, name: Rc<str>, namespace: Namespace, binding: BindingId) {
        if let Self::Saved(saved) = self {
            *self = Self::Built(Self::built_from(*saved));
        }
        if let Self::Built(exports) = self {
            exports.insert((name, namespace), binding);
        }
    }

    pub fn get(&self, name: &str, namespace: Namespace) -> Option<BindingId> {
        match self {
            Self::Built(exports) => exports.get(&(Rc::from(name), namespace)).copied(),
            Self::Saved(saved) => saved.get(name, namespace),
        }
    }

    pub fn entries(&self) -> impl Iterator<Item = (Rc<str>, Namespace, BindingId)> + '_ {
        match self {
            Self::Built(exports) => ExportEntries::Built(exports.iter()),
            Self::Saved(saved) => ExportEntries::Saved(*saved, 0..saved.count()),
        }
    }

    pub fn saved(&self) -> Option<SavedExports> {
        match self {
            Self::Built(_) => None,
            Self::Saved(saved) => Some(*saved),
        }
    }

    fn built_from(saved: SavedExports) -> FxHashMap<(Rc<str>, Namespace), BindingId> {
        let entries = ExportEntries::Saved(saved, 0..saved.count());
        entries
            .map(|(name, namespace, binding)| ((name, namespace), binding))
            .collect()
    }
}

enum ExportEntries<'a> {
    Built(hash_map::Iter<'a, (Rc<str>, Namespace), BindingId>),
    Saved(SavedExports, Range<usize>),
}

impl Iterator for ExportEntries<'_> {
    type Item = (Rc<str>, Namespace, BindingId);

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Built(entries) => {
                let ((name, namespace), binding) = entries.next()?;
                Some((Rc::clone(name), *namespace, *binding))
            }
            Self::Saved(saved, positions) => {
                let (name, namespace, binding) = saved.export(positions.next()?);
                Some((Rc::from(name), namespace, binding))
            }
        }
    }
}

impl serde::Serialize for ExportTable {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Built(exports) => {
                let entries = exports.iter();
                let exports =
                    entries.map(|((name, namespace), binding)| (&**name, *namespace, *binding));
                ExportBytes::of(exports).serialize(serializer)
            }
            Self::Saved(saved) => saved.serialize(serializer),
        }
    }
}

impl<'de: 'static> serde::Deserialize<'de> for ExportTable {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        SavedExports::deserialize(deserializer).map(Self::Saved)
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::ExportTable;
    use crate::core::{BindingId, Namespace};

    struct ExportFixture {
        built: ExportTable,
    }

    impl ExportFixture {
        fn of(exports: &[(&str, Namespace, u32)]) -> Self {
            let mut built = ExportTable::default();
            for (name, namespace, binding) in exports {
                built.insert(Rc::from(*name), *namespace, BindingId::from_index(*binding));
            }
            Self { built }
        }

        fn saved(&self) -> ExportTable {
            bincode::deserialize(self.bytes().leak()).expect("the table loads")
        }

        fn bytes(&self) -> Vec<u8> {
            bincode::serialize(&self.built).expect("the table saves")
        }

        fn listed(table: &ExportTable) -> Vec<(Rc<str>, Namespace, BindingId)> {
            table.entries().collect()
        }
    }

    const EXPORTS: [(&str, Namespace, u32); 3] = [
        ("map", Namespace::Value, 3),
        ("str", Namespace::Value, 5),
        ("str", Namespace::Type, 8),
    ];

    #[test]
    fn a_saved_table_finds_an_export_in_its_namespace() {
        let saved = ExportFixture::of(&EXPORTS).saved();
        assert_eq!(
            saved.get("map", Namespace::Value),
            Some(BindingId::from_index(3))
        );
        assert_eq!(
            saved.get("str", Namespace::Type),
            Some(BindingId::from_index(8))
        );
        assert_eq!(saved.get("map", Namespace::Type), None);
        assert_eq!(saved.get("filter", Namespace::Value), None);
    }

    #[test]
    fn a_saved_table_lists_its_exports_in_the_built_order() {
        let fixture = ExportFixture::of(&EXPORTS);
        assert_eq!(
            ExportFixture::listed(&fixture.saved()),
            ExportFixture::listed(&fixture.built)
        );
    }

    #[test]
    fn a_saved_table_saves_to_the_bytes_it_loaded_from() {
        let fixture = ExportFixture::of(&EXPORTS);
        let saved = fixture.saved();
        assert_eq!(
            bincode::serialize(&saved).expect("the table saves"),
            fixture.bytes()
        );
    }

    #[test]
    fn a_saved_table_takes_a_new_export_beside_its_own() {
        let mut table = ExportFixture::of(&EXPORTS).saved();
        let filter = BindingId::from_index(9);
        table.insert(Rc::from("filter"), Namespace::Value, filter);
        assert!(table.saved().is_none());
        assert_eq!(table.get("filter", Namespace::Value), Some(filter));
        assert_eq!(
            table.get("str", Namespace::Type),
            Some(BindingId::from_index(8))
        );
        assert_eq!(table.entries().count(), EXPORTS.len() + 1);
    }
}
