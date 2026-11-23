use rustc_hash::FxHashMap;

use crate::core::{CoreModule, ModuleId};
use crate::expand::module_notes::ModuleNotes;
use crate::reader::Form;

pub(in crate::expand) struct NotesByModule(FxHashMap<ModuleId, Vec<(String, Vec<Form>)>>);

impl NotesByModule {
    pub(in crate::expand) fn of(notes: &mut ModuleNotes) -> Self {
        let mut by_module: FxHashMap<ModuleId, Vec<(String, Vec<Form>)>> = FxHashMap::default();
        for ((module, key), notes) in std::mem::take(notes) {
            by_module.entry(module).or_default().push((key, notes));
        }
        Self(by_module)
    }

    pub(in crate::expand) fn hand_to(mut self, modules: &mut [CoreModule]) {
        for module in modules {
            if let Some(notes) = self.0.remove(&module.id) {
                module.lint_facts.set_notes(notes);
            }
        }
    }
}
