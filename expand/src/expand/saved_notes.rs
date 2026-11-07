use std::cell::OnceCell;

use rustc_hash::FxHashMap;
use serde::{Deserialize, Deserializer, Serialize};

use crate::core::ModuleId;
use crate::reader::Form;

pub type NotesKey = (ModuleId, String);
pub type NotesTable = FxHashMap<NotesKey, Vec<Form>>;
type ModuleTable = FxHashMap<String, Vec<Form>>;

#[derive(Default, Serialize)]
pub struct NoteTables {
    modules: Vec<(ModuleId, Vec<u8>)>,
}

impl NoteTables {
    pub fn encode(table: &NotesTable) -> Self {
        let mut tables: FxHashMap<ModuleId, ModuleTable> = FxHashMap::default();
        for ((module, key), notes) in table {
            tables
                .entry(*module)
                .or_default()
                .insert(key.clone(), notes.clone());
        }
        let mut modules: Vec<(ModuleId, Vec<u8>)> = tables
            .iter()
            .map(|(module, table)| (*module, Self::encode_module(table)))
            .collect();
        modules.sort_by_key(|(module, _)| module.index());
        Self { modules }
    }

    fn encode_module(table: &ModuleTable) -> Vec<u8> {
        bincode::serialize(table).expect("every note serializes")
    }
}

#[derive(Default)]
pub struct SavedNotes {
    modules: FxHashMap<ModuleId, SavedModuleNotes>,
}

impl<'de: 'static> Deserialize<'de> for SavedNotes {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let modules: Vec<(ModuleId, &'static [u8])> = Deserialize::deserialize(deserializer)?;
        let modules = modules
            .into_iter()
            .map(|(module, encoded)| (module, SavedModuleNotes::borrowing(encoded)))
            .collect();
        Ok(Self { modules })
    }
}

impl SavedNotes {
    pub fn of(&self, module: ModuleId, key: &str) -> Option<&Vec<Form>> {
        self.modules.get(&module)?.decoded().get(key)
    }

    #[cfg(test)]
    pub fn decoded_nothing(&self) -> bool {
        self.modules.values().all(SavedModuleNotes::decoded_nothing)
    }
}

struct SavedModuleNotes {
    encoded: &'static [u8],
    decoded: OnceCell<ModuleTable>,
}

impl SavedModuleNotes {
    fn borrowing(encoded: &'static [u8]) -> Self {
        Self {
            encoded,
            decoded: OnceCell::new(),
        }
    }

    #[cfg(test)]
    fn decoded_nothing(&self) -> bool {
        self.decoded.get().is_none()
    }

    fn decoded(&self) -> &ModuleTable {
        self.decoded
            .get_or_init(|| bincode::deserialize(self.encoded).expect("the saved notes decode"))
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::reader::FormKind;
    use crate::source::{SourceId, Span};

    struct SavedTable {
        noted_module: ModuleId,
        other_noted_module: ModuleId,
        other_module: ModuleId,
    }

    impl SavedTable {
        const KEY: &str = "docstrings";
        const NOTE: &str = "A saved note.";

        fn new() -> Self {
            Self {
                noted_module: ModuleId::new(1),
                other_noted_module: ModuleId::new(2),
                other_module: ModuleId::new(3),
            }
        }

        fn saved(&self) -> SavedNotes {
            let mut table = NotesTable::default();
            for module in [self.noted_module, self.other_noted_module] {
                table.insert((module, Self::KEY.to_string()), vec![Self::note()]);
            }
            let bytes =
                bincode::serialize(&NoteTables::encode(&table)).expect("saved notes serialize");
            bincode::deserialize(bytes.leak()).expect("saved notes deserialize")
        }

        fn note() -> Form {
            Form::new(
                FormKind::String(Rc::from(Self::NOTE)),
                Span::new(SourceId::new(0), 0..0),
            )
        }
    }

    #[test]
    fn a_loaded_table_decodes_no_note_before_a_read() {
        let saved = SavedTable::new().saved();
        assert!(saved.decoded_nothing());
    }

    #[test]
    fn a_read_of_a_noted_module_decodes_its_notes() {
        let table = SavedTable::new();
        let saved = table.saved();
        let notes = saved.of(table.noted_module, SavedTable::KEY);
        assert_eq!(notes.map(Vec::len), Some(1));
    }

    #[test]
    fn a_read_of_a_module_without_saved_notes_decodes_nothing() {
        let table = SavedTable::new();
        let saved = table.saved();
        assert!(saved.of(table.other_module, SavedTable::KEY).is_none());
        assert!(saved.decoded_nothing());
    }

    #[test]
    fn a_read_of_a_noted_module_leaves_other_modules_encoded() {
        let table = SavedTable::new();
        let saved = table.saved();
        saved.of(table.noted_module, SavedTable::KEY);
        let other = &saved.modules[&table.other_noted_module];
        assert!(other.decoded_nothing());
    }
}
