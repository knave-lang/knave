use rustc_hash::FxHashMap;

use super::project_modules::ProjectModules;
use crate::core::{CoreModule, ModuleId};
use crate::expand::module_notes::ModuleNotes;
use crate::reader::Form;

pub(in crate::expand) struct ProjectNotes(FxHashMap<ModuleId, Vec<(String, Vec<Form>)>>);

impl ProjectNotes {
    pub(in crate::expand) fn of(notes: &mut ModuleNotes, project: &ProjectModules) -> Self {
        let mut by_module: FxHashMap<ModuleId, Vec<(String, Vec<Form>)>> = FxHashMap::default();
        for ((module, key), notes) in std::mem::take(notes) {
            if project.contains(module) {
                by_module.entry(module).or_default().push((key, notes));
            }
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
