use std::rc::Rc;

use rustc_hash::FxHashMap;

use crate::core::{BindingId, Namespace};
use crate::scope::{ExportBytes, ExportParts, SavedExports};

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
        let Self::Built(exports) = self else {
            panic!("a saved module takes no new export")
        };
        exports.insert((name, namespace), binding);
    }

    pub fn get(&self, name: &str, namespace: Namespace) -> Option<BindingId> {
        match self {
            Self::Built(exports) => exports.get(&(Rc::from(name), namespace)).copied(),
            Self::Saved(saved) => saved.get(name, namespace),
        }
    }

    pub fn entries(&self) -> impl Iterator<Item = (Rc<str>, Namespace, BindingId)> + '_ {
        let (built, saved) = match self {
            Self::Built(exports) => (Some(exports), None),
            Self::Saved(saved) => (None, Some(saved)),
        };
        let built_entries = built
            .into_iter()
            .flatten()
            .map(|((name, namespace), binding)| (Rc::clone(name), *namespace, *binding));
        built_entries.chain(saved.into_iter().flat_map(SavedExports::entries))
    }

    pub fn saved(&self) -> Option<SavedExports> {
        match self {
            Self::Built(_) => None,
            Self::Saved(saved) => Some(*saved),
        }
    }

    fn encoded(&self) -> ExportBytes {
        match self {
            Self::Built(exports) => ExportBytes::of(
                exports
                    .iter()
                    .map(|((name, namespace), binding)| (&**name, *namespace, *binding)),
            ),
            Self::Saved(saved) => saved.encoded(),
        }
    }
}

impl serde::Serialize for ExportTable {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.encoded().serialize(serializer)
    }
}

impl<'de: 'static> serde::Deserialize<'de> for ExportTable {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let parts: ExportParts = serde::Deserialize::deserialize(deserializer)?;
        Ok(Self::Saved(SavedExports::of(parts)))
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
    #[should_panic(expected = "a saved module takes no new export")]
    fn a_saved_table_takes_no_new_export() {
        let mut saved = ExportFixture::of(&EXPORTS).saved();
        saved.insert(
            Rc::from("filter"),
            Namespace::Value,
            BindingId::from_index(9),
        );
    }
}
